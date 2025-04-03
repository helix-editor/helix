use quick_xml::de::from_reader;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::time::SystemTime;
use walkdir;

#[derive(Debug)]
pub struct Coverage {
    pub files: HashMap<std::path::PathBuf, FileCoverage>,
}

#[derive(Debug)]
pub struct FileCoverage {
    pub lines: HashMap<u32, bool>,
    pub modified_time: Option<SystemTime>,
}

#[derive(Deserialize, Debug)]
struct RawCoverage {
    #[serde(rename = "@version")]
    version: String,
    sources: Sources,
    packages: Packages,
    modified_time: Option<SystemTime>,
}

#[derive(Deserialize, Debug)]
struct Sources {
    source: Vec<Source>,
}

#[derive(Deserialize, Debug)]
struct Source {
    #[serde(rename = "$value")]
    name: String,
}

#[derive(Deserialize, Debug)]
struct Packages {
    package: Vec<Package>,
}

#[derive(Deserialize, Debug)]
struct Package {
    #[serde(rename = "@name")]
    name: String,
    classes: Classes,
}

#[derive(Deserialize, Debug)]
struct Classes {
    class: Vec<Class>,
}

#[derive(Deserialize, Debug)]
struct Class {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@filename")]
    filename: String,
    lines: Lines,
}

#[derive(Deserialize, Debug)]
struct Lines {
    line: Option<Vec<Line>>,
}

#[derive(Deserialize, Debug)]
struct Line {
    #[serde(rename = "@number")]
    number: u32,
    #[serde(rename = "@hits")]
    hits: u32,
}

/// Get coverage information for a document from the configured coverage file.
///
/// The coverage file is set by environment variable HELIX_COVERAGE_FILE. This
/// function will return None if the coverage file is not found, invalid, does
/// not contain the document, or if it is out of date compared to the document.
pub fn get_coverage(document_path: &std::path::PathBuf) -> Option<FileCoverage> {
    let coverage_path = find_coverage_file()?;
    log::debug!("coverage file is {:?}", coverage_path);
    let coverage = read_cobertura_coverage(&coverage_path)?;
    log::debug!("coverage is valid");

    log::debug!("document path: {:?}", document_path);

    let file_coverage = coverage.files.get(document_path)?;

    let coverage_time = file_coverage.modified_time?;
    let document_metadata = document_path.metadata().ok()?;
    let document_time = document_metadata.modified().ok()?;

    if document_time < coverage_time {
        log::debug!("file coverage contains {} lines", file_coverage.lines.len());
        return Some(FileCoverage {
            lines: file_coverage.lines.clone(),
            modified_time: file_coverage.modified_time,
        });
    } else {
        log::debug!("document is newer than coverage file, will not return coverage");
        return None;
    }
}

fn find_coverage_file() -> Option<std::path::PathBuf> {
    if let Some(coverage_path) = std::env::var("HELIX_COVERAGE_FILE").ok() {
        return Some(std::path::PathBuf::from(coverage_path));
    }
    for entry in walkdir::WalkDir::new(".")
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "coverage.xml" || entry.file_name() == "cobertura.xml" {
            return Some(entry.path().to_path_buf());
        }
    }
    return None;
}

fn read_cobertura_coverage(path: &std::path::PathBuf) -> Option<Coverage> {
    let file = File::open(path)
        .inspect_err(|e| log::info!("error opening {:?}: {:?}", path, e))
        .ok()?;
    let metadata = file
        .metadata()
        .inspect_err(|e| log::info!("error reading metadata for {:?}: {:?}", path, e))
        .ok()?;
    let modified = metadata
        .modified()
        .inspect_err(|e| log::info!("error reading timestamp for {:?}: {:?}", path, e))
        .ok()?;
    let reader = BufReader::new(file);
    let mut tmp: RawCoverage = from_reader(reader)
        .inspect_err(|e| log::info!("error parsing coverage for {:?}: {:?}", path, e))
        .ok()?;
    tmp.modified_time = Some(modified);
    Some(tmp.into())
}

impl From<RawCoverage> for Coverage {
    fn from(coverage: RawCoverage) -> Self {
        let mut files = HashMap::new();
        for package in coverage.packages.package {
            for class in package.classes.class {
                let mut lines = HashMap::new();
                if let Some(class_lines) = class.lines.line {
                    for line in class_lines {
                        lines.insert(line.number - 1, line.hits > 0);
                    }
                }
                for source in &coverage.sources.source {
                    // it is ambiguous to which source a coverage class might belong
                    // so check each in the path
                    let raw_path: std::path::PathBuf =
                        [&source.name, &class.filename].iter().collect();
                    if let Ok(path) = std::fs::canonicalize(raw_path) {
                        files.insert(
                            path,
                            FileCoverage {
                                lines,
                                modified_time: coverage.modified_time,
                            },
                        );
                        break;
                    }
                }
            }
        }
        Coverage { files }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::de::from_str;
    use std::path::PathBuf;

    fn test_string(use_relative_paths: bool) -> String {
        let source_path = if use_relative_paths {
            PathBuf::from("src")
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
        };
        return format!(
            r#"<?xml version="1.0" ?>
<coverage version="7.3.0" timestamp="4333222111000">
	<sources>
		<source>{}</source>
	</sources>
	<packages>
        <package name="a package">
            <classes>
                <class name="a class" filename="coverage.rs">
                    <lines>
                        <line number="3" hits="1"/>
                        <line number="5" hits="0"/>
                    </lines>
                </class>
                <class name="another class" filename="other.ext">
                    <lines>
                        <line number="1" hits="0"/>
                        <line number="7" hits="1"/>
                    </lines>
                </class>
            </classes>
        </package>
    </packages>
</coverage>"#,
            source_path.to_string_lossy()
        );
    }

    #[test]
    fn test_deserialize_raw_coverage_from_string() {
        let result: RawCoverage = from_str(&test_string(true)).unwrap();
        println!("result is {:?}", result);
        assert_eq!(result.version, "7.3.0");
        assert_eq!(result.sources.source[0].name, "src");
        assert_eq!(result.packages.package[0].name, "a package");
        let first = &result.packages.package[0].classes.class[0];
        assert_eq!(first.name, "a class");
        assert_eq!(first.filename, "coverage.rs");
        assert_eq!(first.lines.line[0].number, 3);
        assert_eq!(first.lines.line[0].hits, 1);
        assert_eq!(first.lines.line[1].number, 5);
        assert_eq!(first.lines.line[1].hits, 0);
        let second = &result.packages.package[0].classes.class[1];
        assert_eq!(second.name, "another class");
        assert_eq!(second.filename, "other.ext");
        assert_eq!(second.lines.line[0].number, 1);
        assert_eq!(second.lines.line[0].hits, 0);
        assert_eq!(second.lines.line[1].number, 7);
        assert_eq!(second.lines.line[1].hits, 1);
    }

    #[test]
    fn test_convert_raw_coverage_to_coverage_with_relative_path() {
        let tmp: RawCoverage = from_str(&test_string(true)).unwrap();
        check_coverage(tmp.into());
    }
    #[test]
    fn test_convert_raw_coverage_to_coverage_with_absolute_path() {
        let tmp: RawCoverage = from_str(&test_string(false)).unwrap();
        check_coverage(tmp.into());
    }

    fn check_coverage(result: Coverage) {
        println!("result is {:?}", result);
        // only one file should be included, since src/other.ext does not exist
        assert_eq!(result.files.len(), 1);
        // coverage will always canonicalize path
        let first = result
            .files
            .get(
                &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("src")
                    .join("coverage.rs"),
            )
            .unwrap();
        println!("cov {:?}", first);
        assert_eq!(first.lines.len(), 2);
        assert_eq!(first.lines.get(&2), Some(&true));
        assert_eq!(first.lines.get(&4), Some(&false));
    }
}

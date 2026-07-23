use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    thread,
};

use serde::{Deserialize, Serialize};

use crate::DynError;

#[derive(Deserialize)]
struct Languages {
    grammar: Vec<Grammar>,
}

#[derive(Deserialize)]
struct Grammar {
    name: String,
    source: Option<GrammarSource>,
}

#[derive(Deserialize)]
struct GrammarSource {
    git: Option<String>,
    rev: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct LockedSource {
    git: String,
    rev: String,
    url: String,
    hash: String,
}

struct PendingSource {
    git: String,
    rev: String,
    url: String,
}

#[derive(Deserialize)]
struct PrefetchResult {
    hash: String,
}

pub fn update(args: impl Iterator<Item = String>) -> Result<(), DynError> {
    let jobs = parse_jobs(args)?;
    let root = crate::path::project_root();
    let lock_path = root.join("nix/grammar-sources.json");
    let languages: Languages = toml::from_str(&fs::read_to_string(root.join("languages.toml"))?)?;
    let old: BTreeMap<String, LockedSource> = match fs::read(&lock_path) {
        Ok(contents) => serde_json::from_slice(&contents)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(error) => return Err(error.into()),
    };

    let mut locked = BTreeMap::new();
    let mut changed = Vec::new();
    for grammar in languages.grammar {
        let Some(source) = grammar.source else {
            continue;
        };
        let (Some(git), Some(rev)) = (source.git, source.rev) else {
            continue;
        };
        let url = archive_url(&git, &rev)?;
        match old.get(&grammar.name) {
            Some(previous) if previous.git == git && previous.rev == rev && previous.url == url => {
                locked.insert(grammar.name, previous.clone());
            }
            _ => changed.push((grammar.name, PendingSource { git, rev, url })),
        }
    }

    if !changed.is_empty() {
        println!(
            "Prefetching {} changed grammar source(s) with {} job(s)",
            changed.len(),
            jobs.min(changed.len())
        );
    }

    let next = AtomicUsize::new(0);
    let fetched = Mutex::new(Vec::with_capacity(changed.len()));
    thread::scope(|scope| {
        for _ in 0..jobs.min(changed.len()) {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some((name, source)) = changed.get(index) else {
                    break;
                };
                fetched
                    .lock()
                    .unwrap()
                    .push(prefetch(name, source).map(|source| (name.clone(), source)));
            });
        }
    });

    let mut failures = Vec::new();
    for result in fetched.into_inner().unwrap() {
        match result {
            Ok((name, source)) => {
                locked.insert(name, source);
            }
            Err(error) => failures.push(error),
        }
    }
    if !failures.is_empty() {
        failures.sort();
        return Err(format!(
            "failed to update grammar sources:\n{}",
            failures
                .into_iter()
                .map(|failure| format!("  - {failure}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
        .into());
    }

    let mut temporary = tempfile::NamedTempFile::new_in(&root)?;
    serde_json::to_writer_pretty(&mut temporary, &locked)?;
    writeln!(temporary)?;
    temporary.persist(&lock_path)?;
    println!("Updated {}", lock_path.display());
    Ok(())
}

fn parse_jobs(mut args: impl Iterator<Item = String>) -> Result<usize, DynError> {
    let default = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(16);
    let Some(argument) = args.next() else {
        return Ok(default);
    };
    if argument != "--jobs" {
        return Err(format!("unexpected argument: {argument}").into());
    }
    let jobs: usize = args
        .next()
        .ok_or("--jobs requires a count")?
        .parse()
        .map_err(|_| "--jobs must be a positive integer")?;
    if jobs == 0 {
        return Err("--jobs must be at least 1".into());
    }
    if let Some(argument) = args.next() {
        return Err(format!("unexpected argument: {argument}").into());
    }
    Ok(jobs)
}

fn archive_url(git: &str, rev: &str) -> Result<String, DynError> {
    let source = git
        .strip_prefix("https://")
        .ok_or_else(|| format!("unsupported grammar source: {git}"))?;
    let (host, path) = source
        .split_once('/')
        .ok_or_else(|| format!("unsupported grammar source: {git}"))?;
    let path = path.trim_matches('/');
    if path.is_empty() {
        return Err(format!("unsupported grammar source: {git}").into());
    }
    match host {
        "github.com" | "codeberg.org" | "git.sr.ht" => {
            Ok(format!("https://{host}/{path}/archive/{rev}.tar.gz"))
        }
        "gitlab.com" => Ok(format!(
            "https://gitlab.com/api/v4/projects/{}/repository/archive.tar.gz?sha={rev}",
            percent_encode(path)
        )),
        _ => Err(format!("unsupported grammar source host: {host}").into()),
    }
}

fn percent_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[(byte >> 4) as usize]));
            encoded.push(char::from(HEX[(byte & 0x0f) as usize]));
        }
    }
    encoded
}

fn prefetch(name: &str, source: &PendingSource) -> Result<LockedSource, String> {
    let output = Command::new("nix")
        .args(["store", "prefetch-file", "--json", &source.url])
        .output()
        .map_err(|error| format!("{name}: could not run nix: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{name}: failed to prefetch {}: {}",
            source.url,
            stderr.trim()
        ));
    }
    let result: PrefetchResult = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("{name}: invalid nix output: {error}"))?;
    Ok(LockedSource {
        git: source.git.clone(),
        rev: source.rev.clone(),
        url: source.url.clone(),
        hash: result.hash,
    })
}

#[cfg(test)]
mod tests {
    use super::archive_url;

    #[test]
    fn archive_urls() {
        assert_eq!(
            archive_url("https://github.com/tree-sitter/tree-sitter-rust", "abc").unwrap(),
            "https://github.com/tree-sitter/tree-sitter-rust/archive/abc.tar.gz"
        );
        assert_eq!(
            archive_url("https://gitlab.com/gabmus/tree-sitter-blueprint", "abc").unwrap(),
            "https://gitlab.com/api/v4/projects/gabmus%2Ftree-sitter-blueprint/repository/archive.tar.gz?sha=abc"
        );
    }
}

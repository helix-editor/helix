use std::{
    collections::{BTreeMap, VecDeque},
    fs, io,
    io::Write,
    path::PathBuf,
    process::Command,
    sync::Mutex,
    thread,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Debug)]
struct Languages {
    grammar: Vec<Grammar>,
}

#[derive(Deserialize, Debug, Default)]
struct Grammar {
    name: String,
    source: Option<GrammarSource>,
}

#[derive(Deserialize, Debug, Default, Clone)]
struct GrammarSource {
    git: Option<String>,
    rev: Option<String>,
    subpath: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NixArgs {
    args: BTreeMap<String, Value>,
    fetcher: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GrammarLocked {
    nurl: NixArgs,
    #[serde(skip_serializing_if = "Option::is_none")]
    subpath: Option<String>,
}

impl Languages {
    /// Load a file and return a `Languages` object with the grammars.
    fn load(file: Option<PathBuf>) -> Self {
        let path = file.unwrap_or(crate::path::project_root().join("languages.toml"));
        let lang = fs::read_to_string(path).expect("No languages.toml");
        toml::from_str(&lang).expect("failed to parse languages.toml")
    }

    // Create Create the lock file input as a BTreeMap
    fn lock(
        self,
        old: BTreeMap<String, GrammarLocked>,
        jobs: usize,
    ) -> io::Result<BTreeMap<String, GrammarLocked>> {
        let total = self.grammar.len();
        if total == 0 {
            return Ok(BTreeMap::new());
        }

        let worker_count = jobs.max(1).min(total);
        let work = Mutex::new(VecDeque::from(self.grammar));
        let completed: Mutex<Vec<io::Result<(String, GrammarLocked)>>> =
            Mutex::new(Vec::with_capacity(total));

        thread::scope(|scope| {
            for _ in 0..worker_count {
                scope.spawn(|| loop {
                    let grammar = work.lock().unwrap().pop_front();
                    let Some(grammar) = grammar else {
                        break;
                    };

                    let name = grammar.name.clone();
                    let subpath = grammar
                        .source
                        .as_ref()
                        .and_then(|source| source.subpath.clone());

                    // Grammar exists in old, but there is no revision listed
                    // Grammar exists in old, and the revision are the same as the rev in grammar
                    // Grammar exists in old, but the revision is not the same as the rev in grammar
                    // Grammar does not exist in old

                    let result = match old.get(&name) {
                        Some(exists)
                            if exists.nurl.args.get("rev").is_some_and(|f| f.is_string()) =>
                        {
                            let old_rev = match exists.nurl.args.get("rev").unwrap_or(&Value::Null)
                            {
                                Value::String(s) => s.to_string(),
                                _ => String::default(),
                            };
                            let cur_rev = grammar
                                .source
                                .as_ref()
                                .and_then(|source| source.rev.clone())
                                .unwrap_or("__INVLIAD_REF[]".into());
                            if old_rev == cur_rev {
                                Ok((name, exists.clone()))
                            } else {
                                grammar
                                    .nurl()
                                    .map(|nurl| (name, GrammarLocked { nurl, subpath }))
                            }
                        }
                        _ => grammar
                            .nurl()
                            .map(|nurl| (name, GrammarLocked { nurl, subpath })),
                    };
                    completed.lock().unwrap().push(result);
                });
            }
        });

        let mut locked = BTreeMap::new();
        for result in completed.into_inner().unwrap() {
            let (name, grammar) = result?;

            if locked.insert(name.clone(), grammar).is_some() {
                return Err(io::Error::other(format!("duplicate grammar name: {name}")));
            }
        }
        Ok(locked)
    }
}

impl Grammar {
    fn nurl(self) -> io::Result<NixArgs> {
        eprintln!("Fetching: {}...", self.name);

        let source = self
            .source
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "grammar has no source"))?;

        let git = source
            .git
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "source has no git URL"))?;

        let mut command = Command::new("nurl");
        command.args(["--json"]);
        command.arg(&git);
        if let Some(rev) = source.rev {
            command.arg(rev);
        }
        let output = command.output()?;

        if !output.status.success() {
            return Err(io::Error::other(format!(
                "nurl exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }
}

fn parse_jobs(mut args: impl Iterator<Item = String>) -> io::Result<usize> {
    let default = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(16);
    let Some(argument) = args.next() else {
        return Ok(default);
    };
    if argument != "--jobs" {
        return Err(io::Error::other(format!("unexpected argument: {argument}")));
    }
    let jobs: usize = args
        .next()
        .expect("--jobs requires a count")
        .parse()
        .expect("--jobs must be a positive integer");
    if jobs == 0 {
        return Err(io::Error::other("--jobs must be at least 1"));
    }
    if let Some(argument) = args.next() {
        return Err(io::Error::other(format!("unexpected argument: {argument}")));
    }
    Ok(jobs)
}

pub fn update(args: impl Iterator<Item = String>) -> io::Result<()> {
    let langs = Languages::load(None);
    eprintln!("Loaded {} grammars", langs.grammar.len());
    let jobs = parse_jobs(args)?;
    let root = crate::path::project_root();
    let lock_path = root.join("nix/grammar-sources.json");
    let old: BTreeMap<String, GrammarLocked> = match fs::read(&lock_path) {
        Ok(contents) => serde_json::from_slice(&contents)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(error) => return Err(error),
    };
    eprintln!("Loaded {:#?} old locked grammars", old.len());

    let new = langs.lock(old, jobs)?;
    eprintln!("Fetched {} grammars using nurl", &new.len());

    // Write out to file
    let mut temporary = tempfile::NamedTempFile::new_in(&root)?;
    serde_json::to_writer_pretty(&mut temporary, &new)?;
    writeln!(temporary)?;
    temporary.persist(&lock_path)?;
    eprintln!("Written {}", lock_path.display());

    Ok(())
}

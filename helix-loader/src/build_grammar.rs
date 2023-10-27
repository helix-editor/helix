use anyhow::{bail, Result};

use crate::grammar::{GrammarConfiguration, GrammarSource};

fn ensure_git_is_available() -> Result<()> {
    match which::which("git") {
        Ok(_cmd) => Ok(()),
        Err(err) => Err(anyhow::anyhow!("'git' could not be found ({err})")),
    }
}

pub fn fetch_grammars() -> Result<()> {
    ensure_git_is_available()?;

    // We do not need to fetch local grammars.
    let mut grammars = get_grammar_configs()?;
    grammars.retain(|grammar| !matches!(grammar.source, GrammarSource::Local { .. }));

    println!("Fetching {} grammars", grammars.len());
    let results = run_parallel(grammars, fetch_grammar);

    let mut errors = Vec::new();
    let mut git_updated = Vec::new();
    let mut git_up_to_date = 0;
    let mut non_git = Vec::new();

    for (grammar_id, res) in results {
        match res {
            Ok(FetchStatus::GitUpToDate) => git_up_to_date += 1,
            Ok(FetchStatus::GitUpdated { revision }) => git_updated.push((grammar_id, revision)),
            Ok(FetchStatus::NonGit) => non_git.push(grammar_id),
            Err(e) => errors.push((grammar_id, e)),
        }
    }

    non_git.sort_unstable();
    git_updated.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    if git_up_to_date != 0 {
        println!("{} up to date git grammars", git_up_to_date);
    }

    if !non_git.is_empty() {
        println!("{} non git grammars", non_git.len());
        println!("\t{:?}", non_git);
    }

    if !git_updated.is_empty() {
        println!("{} updated grammars", git_updated.len());
        // We checked the vec is not empty, unwrapping will not panic
        let longest_id = git_updated.iter().map(|x| x.0.len()).max().unwrap();
        for (id, rev) in git_updated {
            println!(
                "\t{id:width$} now on {rev}",
                id = id,
                width = longest_id,
                rev = rev
            );
        }
    }

    if !errors.is_empty() {
        let len = errors.len();
        for (i, (grammar, error)) in errors.into_iter().enumerate() {
            println!("Failure {}/{len}: {grammar} {error}", i + 1);
        }
        bail!("{len} grammars failed to fetch");
    }

    Ok(())
}

pub fn build_grammars(target: Option<String>) -> Result<()> {
    ensure_git_is_available()?;

    let grammars = get_grammar_configs()?;
    println!("Building {} grammars", grammars.len());
    let results = run_parallel(grammars, move |grammar| {
        build_grammar(grammar, target.as_deref())
    });

    let mut errors = Vec::new();
    let mut already_built = 0;
    let mut built = Vec::new();

    for (grammar_id, res) in results {
        match res {
            Ok(BuildStatus::AlreadyBuilt) => already_built += 1,
            Ok(BuildStatus::Built) => built.push(grammar_id),
            Err(e) => errors.push((grammar_id, e)),
        }
    }

    built.sort_unstable();

    if already_built != 0 {
        println!("{} grammars already built", already_built);
    }

    if !built.is_empty() {
        println!("{} grammars built now", built.len());
        println!("\t{:?}", built);
    }

    if !errors.is_empty() {
        let len = errors.len();
        for (i, (grammar_id, error)) in errors.into_iter().enumerate() {
            println!("Failure {}/{len}: {grammar_id} {error}", i + 1);
        }
        bail!("{len} grammars failed to build");
    }

    Ok(())
}

// Returns the set of grammar configurations the user requests.
// Grammars are configured in the default and user `languages.toml` and are
// merged. The `grammar_selection` key of the config is then used to filter
// down all grammars into a subset of the user's choosing.
fn get_grammar_configs() -> Result<Vec<GrammarConfiguration>> {
    let config: Configuration = crate::config::user_lang_config()
        .context("Could not parse languages.toml")?
        .try_into()?;

    let grammars = match config.grammar_selection {
        Some(GrammarSelection::Only { only: selections }) => config
            .grammar
            .into_iter()
            .filter(|grammar| selections.contains(&grammar.grammar_id))
            .collect(),
        Some(GrammarSelection::Except { except: rejections }) => config
            .grammar
            .into_iter()
            .filter(|grammar| !rejections.contains(&grammar.grammar_id))
            .collect(),
        None => config.grammar,
    };

    Ok(grammars)
}

fn run_parallel<F, Res>(grammars: Vec<GrammarConfiguration>, job: F) -> Vec<(String, Result<Res>)>
where
    F: Fn(GrammarConfiguration) -> Result<Res> + Send + 'static + Clone,
    Res: Send + 'static,
{
    let pool = threadpool::Builder::new().build();
    let (tx, rx) = channel();

    for grammar in grammars {
        let tx = tx.clone();
        let job = job.clone();

        pool.execute(move || {
            // Ignore any SendErrors, if any job in another thread has encountered an
            // error the Receiver will be closed causing this send to fail.
            let _ = tx.send((grammar.grammar_id.clone(), job(grammar)));
        });
    }

    drop(tx);

    rx.iter().collect()
}

fn fetch_grammar(grammar: GrammarConfiguration) -> Result<FetchStatus> {
    if let GrammarSource::Git {
        remote, revision, ..
    } = grammar.source
    {
        let grammar_dir = crate::runtime_dirs()
            .first()
            .expect("No runtime directories provided") // guaranteed by post-condition
            .join("grammars")
            .join("sources")
            .join(&grammar.grammar_id);

        fs::create_dir_all(&grammar_dir).context(format!(
            "Could not create grammar directory {:?}",
            grammar_dir
        ))?;

        // create the grammar dir contains a git directory
        if !grammar_dir.join(".git").exists() {
            git(&grammar_dir, ["init"])?;
        }

        // ensure the remote matches the configured remote
        if get_remote_url(&grammar_dir).map_or(true, |s| s != remote) {
            set_remote(&grammar_dir, &remote)?;
        }

        // ensure the revision matches the configured revision
        if get_revision(&grammar_dir).map_or(true, |s| s != revision) {
            // Fetch the exact revision from the remote.
            // Supported by server-side git since v2.5.0 (July 2015),
            // enabled by default on major git hosts.
            git(
                &grammar_dir,
                ["fetch", "--depth", "1", REMOTE_NAME, &revision],
            )?;
            git(&grammar_dir, ["checkout", &revision])?;

            Ok(FetchStatus::GitUpdated { revision })
        } else {
            Ok(FetchStatus::GitUpToDate)
        }
    } else {
        Ok(FetchStatus::NonGit)
    }
}

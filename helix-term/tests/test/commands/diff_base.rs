use std::{fs::File, io::Write, path::Path, process::Command, time::Duration};

use helix_term::config::Config;
use helix_view::doc;
use tempfile::TempDir;

use super::*;

fn exec_git_cmd(args: &str, git_dir: &Path) {
    let res = Command::new("git")
        .arg("-C")
        .arg(git_dir)
        .args(args.split_whitespace())
        .env_remove("GIT_DIR")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .env("GIT_TERMINAL_PROMPT", "false")
        .env("GIT_AUTHOR_DATE", "2000-01-01 00:00:00 +0000")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_AUTHOR_NAME", "author")
        .env("GIT_COMMITTER_DATE", "2000-01-02 00:00:00 +0000")
        .env("GIT_COMMITTER_EMAIL", "committer@example.com")
        .env("GIT_COMMITTER_NAME", "committer")
        .env("GIT_CONFIG_COUNT", "2")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "false")
        .env("GIT_CONFIG_KEY_1", "init.defaultBranch")
        .env("GIT_CONFIG_VALUE_1", "main")
        .output()
        .unwrap_or_else(|_| panic!("`git {args}` failed"));
    assert!(res.status.success(), "`git {args}` failed");
}

fn commit_file(repo: &Path, file: &Path, contents: &str) {
    File::create(file)
        .unwrap()
        .write_all(contents.as_bytes())
        .unwrap();
    exec_git_cmd("add -A", repo);
    exec_git_cmd("commit -m message", repo);
}

/// Builds a repository whose `feature` branch has diverged from `main`:
///
/// ```text
/// main:     A --- C
///            \
/// feature:    --- E   (checked out)
/// ```
///
/// `A` is the merge base, `C` is a change made on `main` after branching, and `E` is the
/// branch's own change. `C` and `E` touch lines that are far enough apart to stay separate
/// hunks, so each of the three possible diff bases yields a different hunk count.
fn diverged_repo() -> (TempDir, std::path::PathBuf) {
    let repo = tempfile::tempdir().expect("create temp dir for git testing");
    exec_git_cmd("init", repo.path());
    exec_git_cmd("config user.email test@helix.org", repo.path());
    exec_git_cmd("config user.name helix-test", repo.path());

    let file = repo.path().join("file.txt");

    // A: the merge base.
    commit_file(repo.path(), &file, "a\nb\nc\nd\ne\n");
    exec_git_cmd("checkout -b feature", repo.path());
    exec_git_cmd("checkout main", repo.path());
    // C: landed on `main` after the branch point. Not the branch's change.
    commit_file(repo.path(), &file, "CHANGED\nb\nc\nd\ne\n");
    exec_git_cmd("checkout feature", repo.path());
    // E: the branch's own change.
    commit_file(repo.path(), &file, "a\nb\nc\nd\nMINE\n");

    (repo, file)
}

/// Polls the focused document's diff, which is computed by a debounced background task.
fn assert_hunks(app: &Application, expected: u32) {
    for _ in 0..100 {
        let doc = doc!(app.editor);
        let hunks = doc.diff_handle().map_or(0, |handle| handle.load().len());
        if hunks == expected {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let doc = doc!(app.editor);
    let hunks = doc.diff_handle().map_or(0, |handle| handle.load().len());
    panic!("expected {expected} hunks, got {hunks}");
}

fn config_with_diff_base(revision: &str) -> Config {
    Config {
        editor: helix_view::editor::Config {
            diff_base: revision.to_string(),
            ..test_editor_config()
        },
        ..test_config()
    }
}

/// The default diff base is `HEAD`, so a clean worktree has no hunks.
#[tokio::test(flavor = "multi_thread")]
async fn diff_base_defaults_to_head() -> anyhow::Result<()> {
    let (_repo, file) = diverged_repo();
    let mut app = AppBuilder::new().with_file(&file, None).build()?;

    test_key_sequence(&mut app, None, Some(&|app| assert_hunks(app, 0)), false).await?;

    Ok(())
}

/// `editor.diff-base` is applied when the document is opened.
#[tokio::test(flavor = "multi_thread")]
async fn diff_base_from_config() -> anyhow::Result<()> {
    let (_repo, file) = diverged_repo();
    let mut app = AppBuilder::new()
        .with_config(config_with_diff_base("main"))
        .with_file(&file, None)
        .build()?;

    // Against the tip of `main` both the first line (`C`) and the last (`E`) differ.
    test_key_sequence(&mut app, None, Some(&|app| assert_hunks(app, 2)), false).await?;

    Ok(())
}

/// A trailing `...` diffs against the merge base, so the commit made on `main` after
/// branching drops out and only the branch's own change is left.
#[tokio::test(flavor = "multi_thread")]
async fn diff_base_merge_base_from_config() -> anyhow::Result<()> {
    let (_repo, file) = diverged_repo();
    let mut app = AppBuilder::new()
        .with_config(config_with_diff_base("main..."))
        .with_file(&file, None)
        .build()?;

    // Against the merge base only the last line (`E`) differs.
    test_key_sequence(&mut app, None, Some(&|app| assert_hunks(app, 1)), false).await?;

    Ok(())
}

/// `:diff-base` updates the gutter of an already-open document without a `:reload`.
#[tokio::test(flavor = "multi_thread")]
async fn diff_base_command_refreshes_open_documents() -> anyhow::Result<()> {
    let (_repo, file) = diverged_repo();
    let mut app = AppBuilder::new().with_file(&file, None).build()?;

    test_key_sequences(
        &mut app,
        vec![
            (None, Some(&|app| assert_hunks(app, 0))),
            (
                Some(":diff-base main<ret>"),
                Some(&|app: &Application| {
                    assert_eq!(app.editor.config().diff_base, "main");
                    assert_hunks(app, 2);
                }),
            ),
            (
                Some(":diff-base main...<ret>"),
                Some(&|app| assert_hunks(app, 1)),
            ),
            (
                Some(":diff-base HEAD<ret>"),
                Some(&|app| assert_hunks(app, 0)),
            ),
            // With no argument the command reports the current base instead of setting it.
            (
                Some(":diff-base<ret>"),
                Some(&|app: &Application| {
                    let (status, _) = app.editor.get_status().unwrap();
                    assert_eq!(status, "diff base: HEAD");
                }),
            ),
        ],
        false,
    )
    .await?;

    Ok(())
}

/// A revision that cannot be resolved reports an error but keeps the gutter working by
/// falling back to `HEAD`.
#[tokio::test(flavor = "multi_thread")]
async fn diff_base_unresolvable_revision_reports_error() -> anyhow::Result<()> {
    let (_repo, file) = diverged_repo();
    let mut app = AppBuilder::new()
        .with_config(config_with_diff_base("no-such-revision"))
        .with_file(&file, None)
        .build()?;

    test_key_sequence(
        &mut app,
        None,
        Some(&|app: &Application| {
            assert!(app.editor.is_err(), "expected an error status");
            // Fell back to HEAD rather than dropping the diff.
            assert_hunks(app, 0);
        }),
        false,
    )
    .await?;

    Ok(())
}

/// `:diff-base <tab>` offers the repository's branches and tags.
#[tokio::test(flavor = "multi_thread")]
async fn diff_base_completes_revisions() -> anyhow::Result<()> {
    let (repo, file) = diverged_repo();
    exec_git_cmd("tag v1", repo.path());
    let mut app = AppBuilder::new().with_file(&file, None).build()?;

    test_key_sequence(
        &mut app,
        None,
        Some(&|app: &Application| {
            let revisions: Vec<String> = helix_term::ui::completers::git_revision(&app.editor, "")
                .into_iter()
                .map(|(_, span)| span.content.to_string())
                .collect();

            for expected in ["HEAD", "main", "feature", "v1"] {
                assert!(
                    revisions.iter().any(|rev| rev == expected),
                    "expected {expected} in {revisions:?}"
                );
            }
        }),
        false,
    )
    .await?;

    Ok(())
}

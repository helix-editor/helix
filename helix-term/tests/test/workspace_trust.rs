use std::{
    fs,
    io::{BufRead, Read, Write},
    sync::Mutex,
};

use helix_loader::workspace_trust::{Config as TrustConfig, WorkspaceTrust};
use helix_view::doc;

use super::*;

/// The workspace trust machinery and the language config refresh resolve the
/// current workspace from the process working directory, which is global
/// state: tests that change it must not run concurrently.
static WORKSPACE_LOCK: Mutex<()> = Mutex::new(());

/// Environment variable that turns [`dummy_lsp_server`] into a minimal LSP
/// server. Used as the `environment` of the language server defined in the
/// test workspace's `.helix/languages.toml`, so the variable is only set for
/// the spawned child process.
const DUMMY_LSP_ENV: &str = "HELIX_TEST_DUMMY_LSP";

/// Quote `s` as a TOML basic string.
fn toml_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A minimal LSP server spawned as this very test binary (the language server
/// command points at `current_exe`). When this test runs normally (as part of
/// the test suite) the environment variable is unset and it returns
/// immediately; when spawned as a language server it responds to `initialize`
/// and `shutdown` and exits on `exit`, so helix considers it initialized and
/// attaches it to the document.
#[test]
fn dummy_lsp_server() {
    if std::env::var_os(DUMMY_LSP_ENV).is_none() {
        return;
    }

    fn reply(
        prefix: &str,
        id: &serde_json::Value,
        result: serde_json::Value,
    ) -> std::io::Result<()> {
        let body = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string();
        let mut stdout = std::io::stdout().lock();
        write!(
            stdout,
            "{prefix}Content-Length: {}\r\n\r\n{body}",
            body.len()
        )?;
        stdout.flush()
    }

    // The test harness prints `test <name> ... ` without a trailing newline
    // before the test runs, which would end up on the same line as (and
    // corrupt) the first JSON-RPC header. Terminate that line first; the
    // transport skips non-header lines.
    let mut first_reply = true;

    let mut reader = std::io::BufReader::new(std::io::stdin().lock());
    loop {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                // stdin closed: helix went away
                return;
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(("Content-Length", value)) = line.split_once(": ") {
                content_length = value.parse().ok();
            }
        }
        let Some(content_length) = content_length else {
            continue;
        };
        let mut body = vec![0u8; content_length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let Ok(message) = serde_json::from_slice::<serde_json::Value>(&body) else {
            continue;
        };
        let prefix = if std::mem::take(&mut first_reply) {
            "\r\n"
        } else {
            ""
        };
        match message.get("method").and_then(|method| method.as_str()) {
            Some("initialize") => {
                reply(
                    prefix,
                    &message["id"],
                    serde_json::json!({ "capabilities": {} }),
                )
                .expect("failed to reply to initialize");
            }
            Some("shutdown") => {
                reply(prefix, &message["id"], serde_json::Value::Null)
                    .expect("failed to reply to shutdown");
            }
            Some("exit") => return,
            _ => {}
        }
    }
}

/// Regression test for <https://github.com/helix-editor/helix/issues/16045>:
/// granting a workspace trust must apply its `.helix/languages.toml` to the
/// documents that are already open, including the configured language servers.
///
/// Before the fix, the config refresh triggered by `:workspace-trust`
/// re-detected the language of open documents but never refreshed their
/// language servers, so servers picked up before the grant kept running even
/// when the workspace config disabled them (for example `language-servers =
/// []`).
#[tokio::test(flavor = "multi_thread")]
async fn workspace_languages_toml_applies_after_trust() -> anyhow::Result<()> {
    let _guard = WORKSPACE_LOCK.lock().unwrap();

    let workspace = tempfile::tempdir()?;
    fs::create_dir_all(workspace.path().join(".helix"))?;

    // The language server for the workspace config is this test binary itself,
    // running [`dummy_lsp_server`] as a minimal LSP server.
    let dummy_command = std::env::current_exe()?;
    fs::write(
        workspace.path().join(".helix").join("languages.toml"),
        format!(
            indoc! {r#"
                [[language]]
                name = "rust"
                language-servers = ["dummy"]

                [language-server.dummy]
                command = {command}
                args = ["dummy_lsp_server", "--nocapture", "--test-threads", "1"]
                environment = {{ {env} = "1" }}
            "#},
            command = toml_string(&dummy_command.to_string_lossy()),
            env = DUMMY_LSP_ENV,
        ),
    )?;
    let file = tempfile::Builder::new()
        .suffix(".rs")
        .tempfile_in(workspace.path())?;

    // The refresh triggered by `:workspace-trust` re-reads the global config
    // file: point it at a scratch file instead of the user's config. LSP is
    // enabled there so the refreshed editor config allows the launch (the
    // default test config keeps `lsp.enable = false`, matching an untrusted
    // workspace where no server is attached to the document yet).
    let global_config = tempfile::NamedTempFile::new()?;
    fs::write(
        global_config.path(),
        indoc! {"
            [editor.lsp]
            enable = true

            [editor.workspace-trust]
            prompt = false
        "},
    )?;
    helix_loader::initialize_config_file(Some(global_config.path().to_path_buf()));

    // `user_lang_config` and `:workspace-trust` resolve the workspace from the
    // current working directory: make it the scratch workspace, like starting
    // `hx` inside a project.
    let old_cwd = helix_stdx::env::current_working_dir();
    helix_stdx::env::set_current_working_dir(workspace.path())?;

    // Startup state: the workspace is not trusted yet, so the loader built at
    // startup would not merge the workspace `languages.toml` (mirrored here by
    // the default test loader).
    let mut app = AppBuilder::new()
        .with_file(file.path(), None)
        .with_workspace_trust(WorkspaceTrust::new(TrustConfig {
            prompt: false,
            ..TrustConfig::default()
        }))
        .build()?;

    let result = test_key_sequence(
        &mut app,
        Some(":workspace-trust<ret>"),
        Some(&|app| {
            let doc = doc!(app.editor);
            let servers: Vec<_> = doc
                .language_servers()
                .map(|ls| ls.name().to_string())
                .collect();
            assert_eq!(
                servers,
                vec!["dummy".to_owned()],
                "the language servers of the trusted workspace's languages.toml \
                 should be running after `:workspace-trust`"
            );
        }),
        false,
    )
    .await;

    helix_stdx::env::set_current_working_dir(old_cwd)?;
    result
}

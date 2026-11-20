use std::{fs, process::Command};

fn health(command: &str, args: &[&str], configured: bool) -> String {
    let workspace = tempfile::tempdir().unwrap();
    fs::create_dir(workspace.path().join(".helix")).unwrap();
    let command = toml::Value::String(command.to_owned());
    let args = toml::Value::Array(
        args.iter()
            .map(|arg| toml::Value::String((*arg).to_owned()))
            .collect(),
    );
    let tools = if configured {
        format!(
            r#"
language-servers = ["health-first", "health-second"]
formatter = {{ command = {command}, args = {args} }}

[language-server.health-first]
command = {command}
args = {args}

[language-server.health-second]
command = {command}
args = ["run", "pylsp"]
"#
        )
    } else {
        String::new()
    };
    fs::write(
        workspace.path().join(".helix/languages.toml"),
        format!(
            r#"
[[language]]
name = "health-test"
scope = "source.health-test"
file-types = []
{tools}
"#
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_hx"))
        .args(["--health", "health-test"])
        .current_dir(workspace.path())
        .env("HOME", workspace.path())
        .env("USERPROFILE", workspace.path())
        .env("APPDATA", workspace.path())
        .env("LOCALAPPDATA", workspace.path())
        .env("XDG_CONFIG_HOME", workspace.path())
        .env("XDG_CACHE_HOME", workspace.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn wrapper_arguments_identify_tools() {
    let output = health(env!("CARGO_BIN_EXE_hx"), &["run", "black", "-"], true);
    assert!(output.contains("health-first:"), "{output}");
    assert!(output.contains("health-second:"), "{output}");
    assert_eq!(
        output
            .matches("Arguments: [\"run\", \"black\", \"-\"]")
            .count(),
        2,
        "{output}"
    );
    assert!(
        output.contains("Arguments: [\"run\", \"pylsp\"]"),
        "{output}"
    );
    assert!(output.contains(env!("CARGO_BIN_EXE_hx")), "{output}");
    assert!(!output.contains("not found in $PATH"), "{output}");
}

#[test]
fn missing_wrapper_keeps_diagnostic_and_literal_arguments() {
    let missing = tempfile::tempdir().unwrap();
    let command = missing.path().join("missing-wrapper");
    let output = health(
        command.to_str().unwrap(),
        &["run", "black", "two words", "", "%{buffer_name}", "\n\x1b"],
        true,
    );
    assert_eq!(output.matches("not found in $PATH").count(), 3, "{output}");
    assert!(output.contains(command.to_str().unwrap()), "{output}");
    assert_eq!(
        output
            .matches(
                r#"Arguments: ["run", "black", "two words", "", "%{buffer_name}", "\n\u{1b}"]"#
            )
            .count(),
        2,
        "{output}"
    );
}

#[test]
fn empty_arguments_and_unconfigured_tools() {
    let output = health(env!("CARGO_BIN_EXE_hx"), &[], true);
    // Only the second language server has arguments.
    assert_eq!(output.matches("Arguments:").count(), 1, "{output}");
    assert!(
        output.contains("Arguments: [\"run\", \"pylsp\"]"),
        "{output}"
    );

    let output = health("", &[], false);
    assert!(!output.contains("Arguments:"), "{output}");
    assert!(!output.contains("health-first"), "{output}");
    assert!(!output.contains("not found in $PATH"), "{output}");
}

use super::*;

use helix_core::diagnostic::Severity;

#[tokio::test(flavor = "multi_thread")]
async fn history_completion() -> anyhow::Result<()> {
    test_key_sequence(
        &mut AppBuilder::new().build()?,
        Some(":asdf<ret>:theme d<C-n><tab>"),
        Some(&|app| {
            assert!(!app.editor.is_err());
        }),
        false,
    )
    .await?;

    Ok(())
}

async fn test_statusline(
    line: &str,
    expected_status: &str,
    expected_severity: Severity,
) -> anyhow::Result<()> {
    test_key_sequence(
        &mut AppBuilder::new().build()?,
        Some(&format!("{line}<ret>")),
        Some(&|app| {
            let (status, &severity) = app.editor.get_status().unwrap();
            assert_eq!(
                severity, expected_severity,
                "'{line}' printed {severity:?}: {status}"
            );
            assert_eq!(status.as_ref(), expected_status);
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn variable_expansion() -> anyhow::Result<()> {
    test_statusline(r#":echo %{cursor_line}"#, "1", Severity::Info).await?;
    // Double quotes can be used with expansions:
    test_statusline(
        r#":echo "line%{cursor_line}line""#,
        "line1line",
        Severity::Info,
    )
    .await?;
    // Within double quotes you can escape the percent token for an expansion by doubling it.
    test_statusline(
        r#":echo "%%{cursor_line}""#,
        "%{cursor_line}",
        Severity::Info,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn unicode_expansion() -> anyhow::Result<()> {
    test_statusline(r#":echo %u{20}"#, " ", Severity::Info).await?;
    test_statusline(r#":echo %u{0020}"#, " ", Severity::Info).await?;
    test_statusline(r#":echo %u{25CF}"#, "●", Severity::Info).await?;
    // Not a valid Unicode codepoint:
    test_statusline(
        r#":echo %u{deadbeef}"#,
        "'echo': could not interpret 'deadbeef' as a Unicode character code",
        Severity::Error,
    )
    .await?;

    Ok(())
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn shell_expansion() -> anyhow::Result<()> {
    test_statusline(
        r#":echo %sh{echo "hello world"}"#,
        "hello world",
        Severity::Info,
    )
    .await?;

    // Shell expansion is recursive.
    test_statusline(":echo %sh{echo '%{cursor_line}'}", "1", Severity::Info).await?;

    Ok(())
}

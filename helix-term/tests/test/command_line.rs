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

#[tokio::test(flavor = "multi_thread")]
async fn prompt_reset_anchor() -> anyhow::Result<()> {
    test_key_sequence(
        &mut AppBuilder::new().build()?,
        Some(":string wider than the terminal window causing the anchor location to be non zero which would panic when the line is deleted<C-u>"),
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
async fn buffer_directory_opens_neighbor() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let parent = dir.path().join("src/renderer's %u{41} directory");
    std::fs::create_dir_all(&parent)?;
    let file = parent.join("renderer.txt");
    let neighbor = parent.join("shader file.vert.glsl");
    std::fs::write(&file, "renderer\n")?;
    std::fs::write(&neighbor, "shader\n")?;

    assert_ne!(parent, helix_stdx::env::current_working_dir());
    test_key_sequence(
        &mut AppBuilder::new().with_file(file, None).build()?,
        Some(r#":open "%{buffer_directory}/shader file.vert.glsl"<ret>"#),
        Some(&|app| {
            assert_status_not_error(&app.editor);
            let (_, doc) = helix_view::current_ref!(app.editor);
            assert_eq!(
                doc.path(),
                Some(helix_stdx::path::canonicalize(&neighbor).as_path())
            );
            assert_eq!(doc.text().to_string(), "shader\n");
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn buffer_directory_follows_current_buffer() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    std::fs::create_dir(&first)?;
    std::fs::create_dir(&second)?;
    std::fs::write(first.join("renderer.txt"), "first\n")?;
    std::fs::write(second.join("renderer.txt"), "second\n")?;
    let neighbor = second.join("shader.vert.glsl");
    std::fs::write(&neighbor, "shader\n")?;

    test_key_sequence(
        &mut AppBuilder::new()
            .with_file(first.join("renderer.txt"), None)
            .build()?,
        Some(
            r#":open "%{buffer_directory}/../second/renderer.txt"<ret>:open "%{buffer_directory}/shader.vert.glsl"<ret>"#,
        ),
        Some(&|app| {
            assert_status_not_error(&app.editor);
            let (_, doc) = helix_view::current_ref!(app.editor);
            assert_eq!(doc.path(), Some(helix_stdx::path::canonicalize(&neighbor).as_path()));
            assert_eq!(doc.text().to_string(), "shader\n");
        }),
        false,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn buffer_directory_escaped_filename() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let neighbor = dir.path().join("shader's %u{41}.glsl");
    std::fs::write(&neighbor, "shader\n")?;

    test_key_sequence(
        &mut AppBuilder::new()
            .with_file(dir.path().join("renderer.txt"), None)
            .build()?,
        Some(r#":open "%{buffer_directory}/shader's %%u{41}.glsl"<ret>"#),
        Some(&|app| {
            assert_status_not_error(&app.editor);
            let (_, doc) = helix_view::current_ref!(app.editor);
            assert_eq!(
                doc.path(),
                Some(helix_stdx::path::canonicalize(&neighbor).as_path())
            );
            assert_eq!(doc.text().to_string(), "shader\n");
        }),
        false,
    )
    .await?;

    Ok(())
}

async fn test_buffer_directory(
    path: Option<&std::path::Path>,
    expected: &std::path::Path,
) -> anyhow::Result<()> {
    let mut app = AppBuilder::new().build()?;
    let (_, doc) = helix_view::current!(app.editor);
    doc.set_path(path);

    test_key_sequence(
        &mut app,
        Some(":echo %{buffer_directory}<ret>"),
        Some(&|app| {
            let (status, &severity) = app.editor.get_status().unwrap();
            assert_eq!(severity, Severity::Info);
            assert_eq!(status.as_ref(), expected.to_string_lossy());
        }),
        false,
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn buffer_directory_scratch_buffer() -> anyhow::Result<()> {
    test_buffer_directory(None, &helix_stdx::env::current_working_dir()).await
}

#[tokio::test(flavor = "multi_thread")]
async fn buffer_directory_new_file() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let parent = dir.path().join("not-yet-created");
    let file = parent.join("renderer.txt");
    assert!(!parent.exists());
    test_buffer_directory(Some(&file), &helix_stdx::path::canonicalize(parent)).await
}

#[tokio::test(flavor = "multi_thread")]
async fn buffer_directory_root() -> anyhow::Result<()> {
    let cwd = helix_stdx::env::current_working_dir();
    let root = cwd.ancestors().last().unwrap();
    test_buffer_directory(Some(&root.join("renderer.txt")), root).await
}

#[tokio::test(flavor = "multi_thread")]
async fn buffer_directory_completion() -> anyhow::Result<()> {
    test_statusline(
        ":echo %{buffer_dir<tab>}",
        &helix_stdx::env::current_working_dir().to_string_lossy(),
        Severity::Info,
    )
    .await
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

#[tokio::test(flavor = "multi_thread")]
async fn register_expansion() -> anyhow::Result<()> {
    test_statusline(
        r#":set-register a hello world<ret>:echo %reg{a}"#,
        "hello world",
        Severity::Info,
    )
    .await?;
    test_statusline(r#":echo %reg{a}"#, "", Severity::Info).await?;
    test_statusline(
        r#":echo %reg{abc}"#,
        "'echo': Invalid register `abc`: should only be a single character",
        Severity::Error,
    )
    .await?;

    // Register expansion evaluation is *not* recursive.
    test_statusline(
        r#":set-register a b<ret>:set-register b hello<ret>:echo %reg{%reg{a}}"#,
        "'echo': Invalid register `%reg{a}`: should only be a single character",
        Severity::Error,
    )
    .await?;
    test_statusline(
        r#":set-register a hello<ret>:set-register b %%reg{a}<ret>:echo %reg{b}"#,
        "%reg{a}",
        Severity::Info,
    )
    .await?;

    // However, you can copy the contents of one register into another with this expansion if you
    // want to.
    test_statusline(
        r#":set-register a hello<ret>:set-register b %reg{a}<ret>:echo %reg{b}"#,
        "hello",
        Severity::Info,
    )
    .await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn percent_escaping() -> anyhow::Result<()> {
    test_statusline(
        r#":sh echo hello 10%"#,
        "'run-shell-command': '%' was not properly escaped. Please use '%%'",
        Severity::Error,
    )
    .await?;
    Ok(())
}

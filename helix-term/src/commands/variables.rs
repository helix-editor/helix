use std::borrow::Cow;

use anyhow::Result;
use helix_core::coords_at_pos;
use helix_view::Editor;

pub fn expand<'a>(editor: &Editor, input: Cow<'a, str>, expand_sh: bool) -> Result<Cow<'a, str>> {
    let (view, doc) = current_ref!(editor);
    let shell = &editor.config().shell;

    let mut output: Option<String> = None;

    let mut chars = input.char_indices();
    let mut last_push_end: usize = 0;

    while let Some((index, char)) = chars.next() {
        if char == '%' {
            if let Some((_, char)) = chars.next() {
                if char == '{' {
                    for (end, char) in chars.by_ref() {
                        if char == '}' {
                            if output.is_none() {
                                output = Some(String::with_capacity(input.len()))
                            }

                            if let Some(o) = output.as_mut() {
                                o.push_str(&input[last_push_end..index]);
                                last_push_end = end + 1;

                                let value = match &input[index + 2..end] {
                                    "basename" | "b" => doc
                                        .path()
                                        .and_then(|it| it.file_name().and_then(|it| it.to_str()))
                                        .unwrap_or(crate::helix_view::document::SCRATCH_BUFFER_NAME)
                                        .to_owned(),
                                    "filename" | "f" => doc
                                        .path()
                                        .and_then(|it| it.to_str())
                                        .unwrap_or(crate::helix_view::document::SCRATCH_BUFFER_NAME)
                                        .to_owned(),
                                    "filename:repo_rel" => {
                                        // This will get repo root or cwd if not inside a git repo.
                                        let workspace_path = helix_loader::find_workspace().0;
                                        doc.path()
                                            .and_then(|p| {
                                                p.strip_prefix(workspace_path).unwrap_or(p).to_str()
                                            })
                                            .unwrap_or(
                                                crate::helix_view::document::SCRATCH_BUFFER_NAME,
                                            )
                                            .to_owned()
                                    }
                                    "filename:rel" => {
                                        let cwd = helix_stdx::env::current_working_dir();
                                        doc.path()
                                            .and_then(|p| p.strip_prefix(cwd).unwrap_or(p).to_str())
                                            .unwrap_or(
                                                crate::helix_view::document::SCRATCH_BUFFER_NAME,
                                            )
                                            .to_owned()
                                    }
                                    "dirname" | "d" => doc
                                        .path()
                                        .and_then(|p| p.parent())
                                        .and_then(std::path::Path::to_str)
                                        .unwrap_or({
                                            crate::helix_view::document::SCRATCH_BUFFER_NAME
                                        })
                                        .to_owned(),
                                    "repo" => helix_loader::find_workspace()
                                        .0
                                        .to_str()
                                        .unwrap_or("")
                                        .to_owned(),
                                    "cwd" => helix_stdx::env::current_working_dir()
                                        .to_str()
                                        .unwrap()
                                        .to_owned(),
                                    "linenumber" => (doc
                                        .selection(view.id)
                                        .primary()
                                        .cursor_line(doc.text().slice(..))
                                        + 1)
                                    .to_string(),
                                    "cursorcolumn" => (coords_at_pos(
                                        doc.text().slice(..),
                                        doc.selection(view.id)
                                            .primary()
                                            .cursor(doc.text().slice(..)),
                                    )
                                    .col + 1)
                                        .to_string(),
                                    "lang" => doc.language_name().unwrap_or("text").to_string(),
                                    "ext" => doc
                                        .relative_path()
                                        .and_then(|p| {
                                            p.extension()?.to_os_string().into_string().ok()
                                        })
                                        .unwrap_or_default(),
                                    "selection" => doc
                                        .selection(view.id)
                                        .primary()
                                        .fragment(doc.text().slice(..))
                                        .to_string(),
                                    _ => {
                                        anyhow::bail!("Unknown variable {}", &input[index + 2..end])
                                    }
                                };

                                o.push_str(value.trim());

                                break;
                            }
                        }
                    }
                } else if char == 's' && expand_sh {
                    if let (Some((_, 'h')), Some((_, '{'))) = (chars.next(), chars.next()) {
                        let mut right_bracket_remaining = 1;
                        for (end, char) in chars.by_ref() {
                            if char == '}' {
                                right_bracket_remaining -= 1;

                                if right_bracket_remaining == 0 {
                                    if output.is_none() {
                                        output = Some(String::with_capacity(input.len()))
                                    }

                                    if let Some(o) = output.as_mut() {
                                        let body = expand(
                                            editor,
                                            Cow::Borrowed(&input[index + 4..end]),
                                            expand_sh,
                                        )?;

                                        let output = tokio::task::block_in_place(move || {
                                            helix_lsp::block_on(async move {
                                                let mut command =
                                                    tokio::process::Command::new(&shell[0]);
                                                command.args(&shell[1..]).arg(&body[..]);

                                                let output =
                                                    command.output().await.map_err(|_| {
                                                        anyhow::anyhow!(
                                                            "Shell command failed: {body}"
                                                        )
                                                    })?;

                                                if output.status.success() {
                                                    String::from_utf8(output.stdout).map_err(|_| {
                                                        anyhow::anyhow!(
                                                            "Process did not output valid UTF-8"
                                                        )
                                                    })
                                                } else if output.stderr.is_empty() {
                                                    Err(anyhow::anyhow!(
                                                        "Shell command failed: {body}"
                                                    ))
                                                } else {
                                                    let stderr =
                                                        String::from_utf8_lossy(&output.stderr);

                                                    Err(anyhow::anyhow!("{stderr}"))
                                                }
                                            })
                                        });
                                        o.push_str(&input[last_push_end..index]);
                                        last_push_end = end + 1;

                                        o.push_str(output?.trim());

                                        break;
                                    }
                                }
                            } else if char == '{' {
                                right_bracket_remaining += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(o) = output.as_mut() {
        o.push_str(&input[last_push_end..]);
    }

    match output {
        Some(o) => Ok(Cow::Owned(o)),
        None => Ok(input),
    }
}

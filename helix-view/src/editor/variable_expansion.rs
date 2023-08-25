use std::borrow::Cow;

use crate::Editor;

pub fn expand_variables<'a>(editor: &Editor, input: &'a str) -> anyhow::Result<Cow<'a, str>> {
    let (view, doc) = current_ref!(editor);
    let shell = &editor.config().shell;

    let mut output: Option<String> = None;

    let mut chars = input.char_indices();
    let mut last_push_end: usize = 0;

    while let Some((index, char)) = chars.next() {
        if char == '%' {
            if let Some((_, char)) = chars.next() {
                if char == '{' {
                    while let Some((end, char)) = chars.next() {
                        if char == '}' {
                            if output == None {
                                output = Some(String::with_capacity(input.len()))
                            }

                            if let Some(o) = output.as_mut() {
                                o.push_str(&input[last_push_end..index]);
                                last_push_end = end + 1;

                                let value = match &input[index + 2..end] {
                                    "filename" => doc
                                        .path()
                                        .and_then(|it| it.to_str())
                                        .unwrap_or(crate::document::SCRATCH_BUFFER_NAME)
                                        .to_owned(),
                                    "dirname" => doc
                                        .path()
                                        .and_then(|p| p.parent())
                                        .and_then(std::path::Path::to_str)
                                        .unwrap()
                                        .to_owned(),
                                    "linenumber" => (doc
                                        .selection(view.id)
                                        .primary()
                                        .cursor_line(doc.text().slice(..))
                                        + 1)
                                    .to_string(),
                                    _ => anyhow::bail!("Unknown variable"),
                                };

                                o.push_str(&value);

                                break;
                            }
                        }
                    }
                } else if char == 's' {
                    if let (Some((_, 'h')), Some((_, '{'))) = (chars.next(), chars.next()) {
                        let mut right_bracket_remaining = 1;
                        while let Some((end, char)) = chars.next() {
                            if char == '}' {
                                right_bracket_remaining -= 1;

                                if right_bracket_remaining == 0 {
                                    if output == None {
                                        output = Some(String::with_capacity(input.len()))
                                    }

                                    if let Some(o) = output.as_mut() {
                                        let body =
                                            expand_variables(editor, &input[index + 4..end])?;

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

                                        o.push_str(&output?);

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
        Some(o) => Ok(std::borrow::Cow::Owned(o)),
        None => Ok(std::borrow::Cow::Borrowed(input)),
    }
}

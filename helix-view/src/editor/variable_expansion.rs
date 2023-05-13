use std::borrow::Cow;

use crate::Editor;

static EXPAND_VARIABLES_REGEX: once_cell::sync::Lazy<helix_core::regex::Regex> =
    once_cell::sync::Lazy::new(|| {
        helix_core::regex::Regex::new(r"%(\w+)\{([^{}]*(\{[^{}]*\}[^{}]*)*)\}").unwrap()
    });

pub fn expand_variables<'a>(editor: &Editor, input: &'a str) -> anyhow::Result<Cow<'a, str>> {
    let (view, doc) = current_ref!(editor);
    let shell = &editor.config().shell;

    replace_all(
        &EXPAND_VARIABLES_REGEX,
        Cow::Borrowed(input),
        move |keyword, body| match keyword.trim() {
            "val" => match body.trim() {
                "filename" => Ok(doc
                    .path()
                    .and_then(|it| it.to_str())
                    .unwrap_or(crate::document::SCRATCH_BUFFER_NAME)
                    .to_string()),
                "dirname" => doc
                    .path()
                    .and_then(|p| p.parent())
                    .and_then(std::path::Path::to_str)
                    .map_or(
                        Err(anyhow::anyhow!("Current buffer has no path or parent")),
                        |v| Ok(v.to_string()),
                    ),
                "line_number" => Ok((doc
                    .selection(view.id)
                    .primary()
                    .cursor_line(doc.text().slice(..))
                    + 1)
                .to_string()),
                _ => anyhow::bail!("Unknown variable: {body}"),
            },
            "sh" => tokio::task::block_in_place(move || {
                helix_lsp::block_on(async move {
                    let args = &expand_variables(editor, body)?[..];

                    let mut command = tokio::process::Command::new(&shell[0]);
                    command.args(&shell[1..]).arg(args);

                    let output = command
                        .output()
                        .await
                        .map_err(|_| anyhow::anyhow!("Shell command failed: {args}"))?;

                    if output.status.success() {
                        String::from_utf8(output.stdout)
                            .map_err(|_| anyhow::anyhow!("Process did not output valid UTF-8"))
                    } else if output.stderr.is_empty() {
                        Err(anyhow::anyhow!("Shell command failed: {args}"))
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);

                        Err(anyhow::anyhow!("{stderr}"))
                    }
                })
            }),
            _ => anyhow::bail!("Unknown keyword {keyword}"),
        },
    )
}

// Copy of regex::Regex::replace_all to allow using result in the replacer function
fn replace_all<'a>(
    regex: &helix_core::regex::Regex,
    text: Cow<'a, str>,
    matcher: impl Fn(&str, &str) -> anyhow::Result<String>,
) -> anyhow::Result<Cow<'a, str>> {
    let mut it = regex.captures_iter(&text).peekable();

    if it.peek().is_none() {
        return Ok(text);
    }

    let mut new = String::with_capacity(text.len());
    let mut last_match = 0;

    for cap in it {
        let m = cap.get(0).unwrap();
        new.push_str(&text[last_match..m.start()]);

        let replace = matcher(cap.get(1).unwrap().as_str(), cap.get(2).unwrap().as_str())?;

        new.push_str(&replace);

        last_match = m.end();
    }

    new.push_str(&text[last_match..]);

    replace_all(regex, Cow::Owned(new), matcher)
}

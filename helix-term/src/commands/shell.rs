use std::{borrow::Cow, process::Stdio};

use helix_core::{command_line::Args, encoding, Rope, Selection, SmallVec, Tendril, Transaction};
use helix_stdx::rope::RopeSliceExt;
use tokio::process::Command;

use crate::{
    compositor,
    ui::{self, PromptEvent},
};

use super::{
    typed::{complete_command_args, SHELL_COMPLETER, SHELL_SIGNATURE},
    Context,
};

#[derive(Eq, PartialEq)]
pub(super) enum ShellBehavior {
    Replace,
    Ignore,
    Insert,
    Append,
}

pub(super) fn shell_pipe(cx: &mut Context) {
    shell_prompt_for_behavior(cx, "pipe:".into(), ShellBehavior::Replace);
}

pub(super) fn shell_pipe_to(cx: &mut Context) {
    shell_prompt_for_behavior(cx, "pipe-to:".into(), ShellBehavior::Ignore);
}

pub(super) fn shell_insert_output(cx: &mut Context) {
    shell_prompt_for_behavior(cx, "insert-output:".into(), ShellBehavior::Insert);
}

pub(super) fn shell_append_output(cx: &mut Context) {
    shell_prompt_for_behavior(cx, "append-output:".into(), ShellBehavior::Append);
}

pub(super) fn shell_keep_pipe(cx: &mut Context) {
    shell_prompt(cx, "keep-pipe:".into(), |cx, args| {
        let shell = &cx.editor.config().shell;
        let (view, doc) = current!(cx.editor);
        let selection = doc.selection(view.id);

        let mut ranges = SmallVec::with_capacity(selection.len());
        let old_index = selection.primary_index();
        let mut index: Option<usize> = None;
        let text = doc.text().slice(..);

        for (i, range) in selection.ranges().iter().enumerate() {
            let fragment = range.slice(text);
            if let Err(err) = shell_impl(shell, args.join(" ").as_str(), Some(fragment.into())) {
                log::debug!("Shell command failed: {}", err);
            } else {
                ranges.push(*range);
                if i >= old_index && index.is_none() {
                    index = Some(ranges.len() - 1);
                }
            }
        }

        if ranges.is_empty() {
            cx.editor.set_error("No selections remaining");
            return;
        }

        let index = index.unwrap_or_else(|| ranges.len() - 1);
        doc.set_selection(view.id, Selection::new(ranges, index));
    });
}

fn shell_impl(shell: &[String], cmd: &str, input: Option<Rope>) -> anyhow::Result<Tendril> {
    tokio::task::block_in_place(|| helix_lsp::block_on(shell_impl_async(shell, cmd, input)))
}

pub(super) async fn shell_impl_async(
    shell: &[String],
    cmd: &str,
    input: Option<Rope>,
) -> anyhow::Result<Tendril> {
    anyhow::ensure!(!shell.is_empty(), "No shell set");

    let mut process = Command::new(&shell[0]);
    process
        .args(&shell[1..])
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if input.is_some() || cfg!(windows) {
        process.stdin(Stdio::piped());
    } else {
        process.stdin(Stdio::null());
    }

    let mut process = match process.spawn() {
        Ok(process) => process,
        Err(e) => {
            log::error!("Failed to start shell: {}", e);
            return Err(e.into());
        }
    };
    let output = if let Some(mut stdin) = process.stdin.take() {
        let input_task = tokio::spawn(async move {
            if let Some(input) = input {
                helix_view::document::to_writer(&mut stdin, (encoding::UTF_8, false), &input)
                    .await?;
            }
            anyhow::Ok(())
        });
        let (output, _) = tokio::join! {
            process.wait_with_output(),
            input_task,
        };
        output?
    } else {
        // Process has no stdin, so we just take the output
        process.wait_with_output().await?
    };

    let output = if !output.status.success() {
        if output.stderr.is_empty() {
            match output.status.code() {
                Some(exit_code) => anyhow::bail!("Shell command failed: status {}", exit_code),
                None => anyhow::bail!("Shell command failed"),
            }
        }
        String::from_utf8_lossy(&output.stderr)
        // Prioritize `stderr` output over `stdout`
    } else if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::debug!("Command printed to stderr: {stderr}");
        stderr
    } else {
        String::from_utf8_lossy(&output.stdout)
    };

    Ok(Tendril::from(output))
}

pub(super) fn shell(cx: &mut compositor::Context, cmd: &str, behavior: &ShellBehavior) {
    let pipe = match behavior {
        ShellBehavior::Replace | ShellBehavior::Ignore => true,
        ShellBehavior::Insert | ShellBehavior::Append => false,
    };

    let config = cx.editor.config();
    let shell = &config.shell;
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);

    let mut changes = Vec::with_capacity(selection.len());
    let mut ranges = SmallVec::with_capacity(selection.len());
    let text = doc.text().slice(..);

    let mut shell_output: Option<Tendril> = None;
    let mut offset = 0isize;
    for range in selection.ranges() {
        let output = if let Some(output) = shell_output.as_ref() {
            output.clone()
        } else {
            let input = range.slice(text);
            match shell_impl(shell, cmd, pipe.then(|| input.into())) {
                Ok(mut output) => {
                    if !input.ends_with("\n") && output.ends_with('\n') {
                        output.pop();
                        if output.ends_with('\r') {
                            output.pop();
                        }
                    }

                    if !pipe {
                        shell_output = Some(output.clone());
                    }
                    output
                }
                Err(err) => {
                    cx.editor.set_error(err.to_string());
                    return;
                }
            }
        };

        let output_len = output.chars().count();

        let (from, to, deleted_len) = match behavior {
            ShellBehavior::Replace => (range.from(), range.to(), range.len()),
            ShellBehavior::Insert => (range.from(), range.from(), 0),
            ShellBehavior::Append => (range.to(), range.to(), 0),
            _ => (range.from(), range.from(), 0),
        };

        // These `usize`s cannot underflow because selection ranges cannot overlap.
        let anchor = to
            .checked_add_signed(offset)
            .expect("Selection ranges cannot overlap")
            .checked_sub(deleted_len)
            .expect("Selection ranges cannot overlap");
        let new_range =
            helix_core::Range::new(anchor, anchor + output_len).with_direction(range.direction());
        ranges.push(new_range);
        offset = offset
            .checked_add_unsigned(output_len)
            .expect("Selection ranges cannot overlap")
            .checked_sub_unsigned(deleted_len)
            .expect("Selection ranges cannot overlap");

        changes.push((from, to, Some(output)));
    }

    if behavior != &ShellBehavior::Ignore {
        let transaction = Transaction::change(doc.text(), changes.into_iter())
            .with_selection(Selection::new(ranges, selection.primary_index()));
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view);
    }

    // after replace cursor may be out of bounds, do this to
    // make sure cursor is in view and update scroll as well
    view.ensure_cursor_in_view(doc, config.scrolloff);
}

fn shell_prompt<F>(cx: &mut Context, prompt: Cow<'static, str>, mut callback_fn: F)
where
    F: FnMut(&mut compositor::Context, Args) + 'static,
{
    ui::prompt(
        cx,
        prompt,
        Some('|'),
        |editor, input| complete_command_args(editor, SHELL_SIGNATURE, &SHELL_COMPLETER, input, 0),
        move |cx, input, event| {
            if event != PromptEvent::Validate || input.is_empty() {
                return;
            }
            match Args::parse(input, SHELL_SIGNATURE, true, |token| {
                super::expansion::expand(cx.editor, token).map_err(|err| err.into())
            }) {
                Ok(args) => callback_fn(cx, args),
                Err(err) => cx.editor.set_error(err.to_string()),
            }
        },
    );
}

fn shell_prompt_for_behavior(cx: &mut Context, prompt: Cow<'static, str>, behavior: ShellBehavior) {
    shell_prompt(cx, prompt, move |cx, args| {
        shell(cx, args.join(" ").as_str(), &behavior)
    })
}

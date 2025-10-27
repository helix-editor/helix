use std::path::Path;
use std::path::PathBuf;

use crate::compositor::Compositor;
use crate::job::Callback;
use crate::job::Job;
use crate::ui;
use anyhow::anyhow;
use helix_stdx::env::set_current_working_dir;
use helix_view::editor::WorkspaceTrust;
use helix_view::events::DocumentDidOpen;
use helix_view::handlers::Handlers;
use helix_view::theme::Modifier;
use helix_view::theme::Style;
use helix_view::Editor;
use tui::text::Span;
use tui::text::Spans;
use ui::overlay::overlaid;

pub fn trust_dialog(editor: &mut Editor, compositor: &mut Compositor) {
    let Some(file_path) = doc!(editor).path() else {
        // helix doesn't send the document open event when it has no paths, but the user may still use :trust-dialog anyways.
        editor.set_error("Could not open trust dialog: the file does not have a path.");
        return;
    };
    let (path, is_file) = helix_loader::find_workspace_in(file_path);

    let do_not_trust = "Do not trust";
    let trust = "Trust";
    let trust_parent = "Turn a parent into a workspace and trust it";
    let untrust_parent = "Turn a parent into a workspace and untrust it";
    let mut options = vec![
        (
            Span::styled(
                do_not_trust,
                Style::new()
                    .fg(helix_view::theme::Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            "Do not allow the usage of LSPs, formatters, debuggers and workspace config. "
                .to_string(),
        ),
        (
            Span::styled(
                trust,
                Style::new()
                    .fg(helix_view::theme::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            "Allow the usage of LSPs, formatters, debuggers and workspace config.".to_string(),
        ),
    ];
    if is_file {
        options.insert(1,(
            Span::styled(
                untrust_parent,
                Style::new()
                    .fg(helix_view::theme::Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            "Insert a '.helix' folder in a parent directory (dialog) and untrust it. See 'Do not trust' option for implications.".to_string(),
        ));

        options.push((
            Span::styled(
                trust_parent,
                Style::new()
                    .fg(helix_view::theme::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            "Insert a '.helix' folder in a parent directory (dialog) and trust it. See 'Trust' option for implications.".to_string(),
        ));
    }
    let file_or_workspace = if is_file { "file" } else { "workspace" };
    let columns = [
        ui::PickerColumn::new(
            format!("Trust {file_or_workspace} '{}'?", path.display()),
            |(t, _): &(Span<'_>, String), _| Spans(vec![t.clone()]).into(),
        ),
        ui::PickerColumn::new("", |(_, explain): &(_, String), _| explain.as_str().into()),
    ];

    let picker = ui::Picker::new(columns, 0, options, (), move |cx, str, _action| {
        let maybe_err = if str.0.content == do_not_trust {
            cx.editor.untrust_workspace()
        } else if str.0.content == trust {
            cx.editor.trust_workspace()
        } else if str.0.content == trust_parent {
            let path_clone = path.clone();
            let job = Job::with_callback(async move {
                Ok(Callback::EditorCompositor(Box::new(
                    |editor, compositor| {
                        choose_parent_dialog(path_clone, true, editor, compositor);
                    },
                )))
            });
            cx.jobs.add(job);
            Ok(())
        } else {
            let path_clone = path.clone();
            let job = Job::with_callback(async move {
                Ok(Callback::EditorCompositor(Box::new(
                    |editor, compositor| {
                        choose_parent_dialog(path_clone, false, editor, compositor);
                    },
                )))
            });
            cx.jobs.add(job);
            Ok(())
        };
        if let Err(e) = maybe_err {
            cx.editor.set_status(e.to_string());
        }
    });
    compositor.push(Box::new(overlaid(picker)));
}

fn choose_parent_dialog(
    path: impl AsRef<Path>,
    trust: bool,
    _editor: &mut Editor,
    compositor: &mut Compositor,
) {
    let path = path.as_ref().to_path_buf();
    let options = path
        .ancestors()
        .skip(1)
        .map(|p| p.to_path_buf())
        .collect::<Vec<PathBuf>>();
    let columns = [ui::PickerColumn::new(
        "Workspace to trust".to_string(),
        |path: &PathBuf, _| path.display().to_string().into(),
    )];
    let picker = ui::Picker::new(columns, 0, options, (), move |cx, path, _action| {
        let result = if let Err(e) = std::fs::create_dir(path.join(".helix")) {
            Err(anyhow!(
                "Couldn't make '{}' into a workspace: {e}",
                path.display()
            ))
        } else if let Err(e) = set_current_working_dir(path) {
            Err(anyhow!(
                "Couldn't set current working directory to '{}': {e}",
                path.display()
            ))
        } else if trust {
            cx.editor.trust_workspace()
        } else {
            cx.editor.untrust_workspace()
        };
        if let Err(e) = result {
            cx.editor.set_error(e.to_string());
        }
    })
    .always_show_header(true);
    compositor.push(Box::new(overlaid(picker)))
}

pub(super) fn register_hooks(_handlers: &Handlers) {
    helix_event::register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        if event.editor.config.load().workspace_trust == WorkspaceTrust::Ask
            && event
                .editor
                .document(event.doc)
                .is_some_and(|doc| doc.is_trusted.is_none())
        {
            tokio::spawn(async move {
                crate::job::dispatch(move |editor, compositor| {
                    trust_dialog(editor, compositor);
                })
                .await;
            });
        }

        Ok(())
    });
}

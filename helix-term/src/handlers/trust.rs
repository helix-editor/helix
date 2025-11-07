use std::path::Path;
use std::path::PathBuf;

use crate::compositor::Component;
use crate::ui;
use crate::ui::PromptEvent;
use anyhow::anyhow;
use helix_loader::trust_db::Trust;
use helix_stdx::env::set_current_working_dir;
use helix_view::editor::WorkspaceTrust;
use helix_view::events::DocumentDidOpen;
use helix_view::events::FileCreated;
use helix_view::handlers::Handlers;
use helix_view::theme::Modifier;
use helix_view::theme::Style;
use tui::text::Span;
use ui::overlay::overlaid;

#[derive(Clone, Copy)]
pub enum TrustOptions {
    Trust,
    DoNotTrust,
    DistrustParent,
    TrustParent,
}

impl ui::menu::Item for TrustOptions {
    type Data = ();
    fn format(&self, _data: &Self::Data) -> tui::widgets::Row<'_> {
        let style = match self {
            TrustOptions::Trust | TrustOptions::TrustParent => {
                Style::new().fg(helix_view::theme::Color::Green)
            }
            TrustOptions::DoNotTrust | TrustOptions::DistrustParent => {
                Style::new().fg(helix_view::theme::Color::Red)
            }
        }
        .add_modifier(Modifier::BOLD);
        Span::styled(self.as_ref(), style).into()
    }
}

impl AsRef<str> for TrustOptions {
    fn as_ref(&self) -> &str {
        match self {
            TrustOptions::Trust => "Trust",
            TrustOptions::DoNotTrust => "Do not trust",
            TrustOptions::DistrustParent => "Distrust a parent directory (picker)",
            TrustOptions::TrustParent => "Trust a parent directory (picker)",
        }
    }
}

pub fn trust_dialog(path: impl AsRef<Path>) -> impl Component + 'static {
    let (path, is_file) = helix_loader::find_workspace_in(path.as_ref());
    let options = vec![
        TrustOptions::DoNotTrust,
        TrustOptions::Trust,
        TrustOptions::DistrustParent,
        TrustOptions::TrustParent,
    ];
    let file_or_workspace = if is_file { "file" } else { "workspace" };
    let path_clone = path.clone();
    let warning = format!("Trusting the {file_or_workspace} will allow the usage of LSPs, formatters, debuggers and workspace config, all of which can lead to remote code execution.
Ensure you trust the source of the {file_or_workspace} before trusting it.");
    ui::Select::new(
        format!("Trust {file_or_workspace} '{}'?\n{warning}", path.display()),
        options,
        (),
        move |editor, option, event| {
            match event {
                PromptEvent::Update => return,
                PromptEvent::Abort => {
                    editor.set_status(
                        "Trust dialog aborted. Use :trust-dialog to bring it up again.",
                    );
                    return;
                }
                PromptEvent::Validate => (),
            }

            let maybe_err = match option {
                TrustOptions::Trust => editor.set_trust(&path_clone, Trust::Trusted),
                TrustOptions::DoNotTrust => editor.set_trust(&path_clone, Trust::Untrusted),
                TrustOptions::DistrustParent | TrustOptions::TrustParent => {
                    let path = path_clone.clone();
                    let option = option.clone();
                    crate::job::dispatch_blocking(move |_editor, compositor| {
                        let dialog =
                            choose_parent_dialog(path, matches!(option, TrustOptions::TrustParent));
                        compositor.push(Box::new(overlaid(dialog)))
                    });
                    Ok(())
                }
            };
            if let Err(e) = maybe_err {
                editor.set_error(e.to_string())
            }
        },
    )
}

fn choose_parent_dialog(path: impl AsRef<Path>, trust: bool) -> impl Component + 'static {
    let path = path.as_ref().to_path_buf();
    let options = path
        .ancestors()
        .skip(1)
        .map(|p| p.to_path_buf())
        .collect::<Vec<PathBuf>>();
    let trust_or_untrust = if trust { "trust" } else { "untrust " };
    let columns = [ui::PickerColumn::new(
        format!("Workspace to {trust_or_untrust}"),
        |path: &PathBuf, _| path.display().to_string().into(),
    )];
    ui::Picker::new(columns, 0, options, (), move |cx, path, _action| {
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
            cx.editor.set_trust(path, Trust::Trusted)
        } else {
            cx.editor.set_trust(path, Trust::Untrusted)
        };
        if let Err(e) = result {
            cx.editor.set_error(e.to_string());
        }
    })
    .always_show_header(true)
}

pub(super) fn register_hooks(_handlers: &Handlers) {
    helix_event::register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        if event.editor.config.load().workspace_trust == WorkspaceTrust::Ask
            && event.editor.document(event.doc).is_some_and(|doc| {
                doc.path().is_some_and(|p| p.exists()) && doc.is_trusted.is_none()
            })
        {
            // these unwraps are fine due to the above. TODO: change this to if let chains once rust is bumped to 1.88
            let path = event
                .editor
                .document(event.doc)
                .unwrap()
                .path()
                .unwrap()
                .to_path_buf();
            crate::job::dispatch_blocking(move |_, compositor| {
                let dialog = trust_dialog(path);
                compositor.push(Box::new(dialog));
            });
        }

        Ok(())
    });

    helix_event::register_hook!(move |event: &mut FileCreated<'_>| {
        if !matches!(
            event.editor.config().workspace_trust,
            WorkspaceTrust::Ask | WorkspaceTrust::Manual
        ) {
            return Ok(());
        }
        if let Some(doc) = event.editor.document(event.doc) {
            if doc.is_trusted.is_none() {
                if let Err(e) = event.editor.set_trust(&event.path, Trust::Trusted) {
                    event.editor.set_error(format!(
                        "Couldn't trust file: {e}; use :trust-workspace to trust it"
                    ))
                }
            }
        }
        Ok(())
    });
}

use std::{ops::Range, path::PathBuf};

use helix_core::Selection;
use helix_lsp::Url;
use helix_view::theme::Style;
use helix_view::Document;
use tui::widgets::{Cell, Row};

use super::{align_view, push_jump, Align, Context, SourcePathFormat};
use crate::ui::{menu::Item, FilePicker};

/// Picker to list the VCS changes in the current document
pub fn vcs_change_picker(cx: &mut Context) {
    let current_doc = doc!(cx.editor);

    let mut changes = Vec::new();
    add_changes_for_doc(&mut changes, current_doc);

    let picker = vcs_picker(cx, changes, current_doc.url(), SourcePathFormat::Hide);
    cx.push_layer(Box::new(picker));
}

// TODO: once both #5472 and #5645 are merged, do a workspace-changes picker

fn add_changes_for_doc(changes: &mut Vec<DiffBlock>, doc: &Document) {
    let url = match doc.url() {
        Some(url) => url,
        None => return,
    };

    if let Some(diff_handle) = doc.diff_handle() {
        let file_hunks = diff_handle.hunks();
        if file_hunks.is_empty() {
            return;
        }

        changes.reserve(file_hunks.len() as _);

        for i in 0..file_hunks.len() {
            let hunk = file_hunks.nth_hunk(i);

            let ty = if hunk.is_pure_insertion() {
                DiffType::PureInsertion
            } else if hunk.is_pure_removal() {
                DiffType::PureRemoval
            } else {
                DiffType::Delta
            };

            let range = hunk.after;
            let line = doc
                .text()
                .get_line(range.start as usize)
                .map_or_else(Default::default, |l| l.to_string());

            changes.push(DiffBlock {
                url: url.clone(),
                ty,
                line,
                range,
            });
        }
    }
}

struct DiffStyles {
    insertion: Style,
    removal: Style,
    delta: Style,
}

struct DiffBlock {
    url: Url,
    ty: DiffType,
    line: String,
    range: Range<u32>,
}

#[derive(Debug, Copy, Clone)]
enum DiffType {
    PureInsertion,
    PureRemoval,
    Delta,
}

impl DiffType {
    fn as_str(&self) -> &'static str {
        match self {
            DiffType::PureInsertion => "[+]",
            DiffType::PureRemoval => "[-]",
            DiffType::Delta => "[~]",
        }
    }
}

impl Item for DiffBlock {
    type Data = (DiffStyles, SourcePathFormat);

    fn format(&self, (styles, source_path_format): &Self::Data) -> Row {
        let path = match source_path_format {
            SourcePathFormat::Hide => String::new(),
            SourcePathFormat::Show => {
                let path = helix_core::path::get_truncated_path(self.url.path());
                format!("{}:", path.to_string_lossy())
            }
        };

        let diff_style = match self.ty {
            DiffType::PureInsertion => styles.insertion,
            DiffType::PureRemoval => styles.removal,
            DiffType::Delta => styles.delta,
        };

        Row::new([
            Cell::from(path),
            Cell::from(self.ty.as_str()).style(diff_style),
            Cell::from(self.line.as_str()),
        ])
    }
}

fn vcs_picker(
    cx: &Context,
    changes: Vec<DiffBlock>,
    current_path: Option<Url>,
    show_source_path: SourcePathFormat,
) -> FilePicker<DiffBlock> {
    let styles = DiffStyles {
        insertion: cx.editor.theme.get("diff.plus"),
        removal: cx.editor.theme.get("diff.minus"),
        delta: cx.editor.theme.get("diff.delta"),
    };

    FilePicker::new(
        changes,
        (styles, show_source_path),
        move |cx, DiffBlock { url, range, .. }, action| {
            if current_path.as_ref() == Some(url) {
                let (view, doc) = current!(cx.editor);
                push_jump(view, doc);
            } else {
                let path = url.to_file_path().unwrap();
                cx.editor.open(&path, action).expect("editor.open failed");
            }

            let (view, doc) = current!(cx.editor);

            let anchor = doc.text().line_to_char(range.start as usize);
            let head = doc
                .text()
                .line_to_char(range.end as usize)
                .saturating_sub(1);
            doc.set_selection(view.id, Selection::single(anchor, head));
            align_view(doc, view, Align::Center);
        },
        move |_editor, DiffBlock { url, range, .. }| {
            Some((
                PathBuf::from(url.path()).into(),
                Some((range.start as usize, range.end.saturating_sub(1) as usize)),
            ))
        },
    )
}

use crate::commands::Context;
use crate::ui::{Picker, PickerColumn};
use helix_core::Selection;
use helix_lsp::lsp::DiagnosticSeverity;
use helix_view::{
    align_view,
    make::{Entry, Location},
    theme::Style,
    Align,
};
use std::path::{Path, PathBuf};
use tui::text::Span;

// TODO(szulf): check the not closing error after opening logs on a non modified version of helix
// TODO(szulf): figure out how to display messages from the make_list the same way as diagnostics
// and make it togglable in the config i think

#[derive(Debug, Clone)]
pub struct MakePickerData {
    root: PathBuf,
    hint: Style,
    info: Style,
    warning: Style,
    error: Style,
}

type MakePicker = Picker<Entry, MakePickerData>;

pub fn make_picker(cx: &Context, root: PathBuf) -> MakePicker {
    let options = cx.editor.make_list.clone().into_iter();

    let data = MakePickerData {
        root: root,
        hint: cx.editor.theme.get("hint"),
        info: cx.editor.theme.get("info"),
        warning: cx.editor.theme.get("warning"),
        error: cx.editor.theme.get("error"),
    };

    let columns = vec![
        PickerColumn::new("severity", |entry: &Entry, data: &MakePickerData| {
            match entry.severity {
                DiagnosticSeverity::HINT => Span::styled("HINT", data.hint),
                DiagnosticSeverity::INFORMATION => Span::styled("INFO", data.info),
                DiagnosticSeverity::WARNING => Span::styled("WARN", data.warning),
                DiagnosticSeverity::ERROR => Span::styled("ERROR", data.error),
                _ => Span::raw(""),
            }
            .into()
        }),
        PickerColumn::new("path", |entry: &Entry, data: &MakePickerData| {
            let path = match entry.location.path.strip_prefix(&data.root) {
                Ok(path) => path.to_str(),
                Err(_) => entry.location.path.to_str(),
            };
            match path {
                Some(str) => str.into(),
                None => "".into(),
            }
        }),
        PickerColumn::new("message", |entry: &Entry, _data: &MakePickerData| {
            entry.msg.clone().into()
        }),
    ];

    Picker::new(columns, 0, options, data, move |cx, item, action| {
        // TODO(szulf): this is copied from the global_search function should i maybe pull it out?
        let doc = match cx.editor.open(&item.location.path, action) {
            Ok(id) => doc_mut!(cx.editor, &id),
            Err(e) => {
                cx.editor.set_error(format!(
                    "Failed to open file '{}': {}",
                    item.location.path.display(),
                    e
                ));
                return;
            }
        };

        let line_num = item.location.line;
        let view = view_mut!(cx.editor);
        let text = doc.text();
        if line_num >= text.len_lines() {
            cx.editor.set_error(
                "The line you jumped to does not exist anymore because the file has changed.",
            );
            return;
        }
        let start = text.line_to_char(line_num);
        let end = text.line_to_char((line_num + 1).min(text.len_lines()));

        doc.set_selection(view.id, Selection::single(start, end));
        if action.align_view(view, doc.id()) {
            align_view(doc, view, Align::Center);
        }
    })
}

// TODO(szulf): dont really see the point of this enum honestly
pub enum MakeFormatType {
    Default,
    Rust,
    Gcc,
    Clang,
    Msvc,
}

impl From<&str> for MakeFormatType {
    fn from(value: &str) -> Self {
        match value {
            "rust" => MakeFormatType::Rust,
            "gcc" => MakeFormatType::Rust,
            "clang" => MakeFormatType::Clang,
            "msvc" => MakeFormatType::Msvc,
            _ => MakeFormatType::Default,
        }
    }
}

fn parse_default<'a, T>(_lines: T) -> Vec<Entry>
where
    T: IntoIterator<Item = &'a str>,
{
    todo!();
}

fn parse_rust<'a, T>(_lines: T) -> Vec<Entry>
where
    T: IntoIterator<Item = &'a str>,
{
    todo!();
}

fn parse_gcc<'a, T>(lines: T) -> Vec<Entry>
where
    T: IntoIterator<Item = &'a str>,
{
    // NOTE(szulf): they SHOULD always be the same
    return parse_clang(lines);
}

// TODO(szulf): better naming
// TODO(szulf): make an error type?
fn check(s: &str) -> Result<Location, ()> {
    let mut loc = s.split(':').collect::<Vec<&str>>();
    loc.retain(|&s| s != "");

    if loc.len() < 3 {
        return Err(());
    }

    let mut loc_fixed: Vec<String> = loc.iter().map(|s| (*s).to_string()).collect();
    // NOTE(szulf): handle paths that contain ':'
    while loc_fixed.len() > 3 {
        let second = loc_fixed.remove(1);
        loc_fixed[0].push_str(second.as_str());
    }

    let path = Path::new(loc_fixed[0].as_str());
    if !path.exists() {
        return Err(());
    }

    let line = match loc_fixed[1].parse::<usize>() {
        Ok(l) => l - 1,
        Err(_) => {
            log::debug!("couldnt parse splits[1]: {:?}", loc_fixed[1]);
            return Err(());
        }
    };

    return Ok(Location {
        path: PathBuf::from(loc_fixed.remove(0)),
        line: line,
    });
}

fn parse_clang<'a, T>(lines: T) -> Vec<Entry>
where
    T: IntoIterator<Item = &'a str>,
{
    // TODO(szulf): better naming
    let e = lines
        .into_iter()
        .map(|s| s.split_whitespace().collect::<Vec<&str>>())
        .collect::<Vec<Vec<&str>>>();

    let mut entries = Vec::new();

    let mut message: String = String::default();
    let mut location = None;
    let mut severity = DiagnosticSeverity::ERROR;

    for s in e {
        let mut iter = s.into_iter().peekable();

        // TODO(szulf): the naming man
        // l loc locat location
        // beautiful
        while let Some(l) = iter.next() {
            let loc = check(l);
            match loc {
                Ok(locat) => {
                    location = Some(locat);

                    if let Some(sever) = iter.peek() {
                        match *sever {
                            "warning:" => {
                                severity = DiagnosticSeverity::WARNING;
                                iter.next();
                            }
                            "note:" => {
                                severity = DiagnosticSeverity::HINT;
                                iter.next();
                            }
                            "error:" => {
                                severity = DiagnosticSeverity::ERROR;
                                iter.next();
                            }
                            _ => severity = DiagnosticSeverity::ERROR,
                        }
                    }

                    message.clear();
                }
                Err(_) => {
                    if message.len() != 0 {
                        message.push_str(" ");
                    }
                    message.push_str(l);
                }
            }
        }

        match location {
            Some(loc) => {
                entries.push(Entry::new(loc, message.as_str(), severity));
            }
            None => {}
        }

        message.clear();
        severity = DiagnosticSeverity::ERROR;
        location = None;
    }

    entries
}

fn parse_msvc<'a, T>(_lines: T) -> Vec<Entry>
where
    T: IntoIterator<Item = &'a str>,
{
    todo!();
}

pub fn parse(format_type: MakeFormatType, source: &str) -> Vec<Entry> {
    let lines = source.lines();

    match format_type {
        MakeFormatType::Default => parse_default(lines),
        MakeFormatType::Rust => parse_rust(lines),
        MakeFormatType::Gcc => parse_gcc(lines),
        MakeFormatType::Clang => parse_clang(lines),
        MakeFormatType::Msvc => parse_msvc(lines),
    }
}

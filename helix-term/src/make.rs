use crate::commands::{goto_location, Context};
use crate::ui::{Picker, PickerColumn};
use helix_lsp::lsp::DiagnosticSeverity;
use helix_view::{
    make::{Entry, Location},
    theme::Style,
};
use std::path::{Path, PathBuf};
use tui::text::Span;

// TODO(szulf): check the not closing error after opening logs on a non modified version of helix
// TODO(szulf): figure out how to display messages from the make_list the same way as diagnostics
// and make it togglable in the config i think(off by default i think)

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
        goto_location(cx, &item.location.path, &item.location.line, action);
    })
}

// TODO(szulf): dont really see the point of this enum honestly
#[derive(Debug)]
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
            "gcc" => MakeFormatType::Gcc,
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

// TODO(szulf): make an error type?
fn get_location_from_token(token: &str, col_amount: usize) -> Result<Location, ()> {
    let mut loc = token
        .split(':')
        .map(|s| (*s).to_string())
        .collect::<Vec<String>>();
    loc.retain(|token| token != "");

    if loc.len() < col_amount {
        return Err(());
    }

    // NOTE(szulf): handle paths that contain ':'
    while loc.len() > col_amount {
        let second = loc.remove(1);
        loc[0].push_str(second.as_str());
    }

    let path = PathBuf::from(loc.remove(0));
    if !path.exists() {
        return Err(());
    }

    let line = match loc[0].parse::<usize>() {
        Ok(l) => l - 1,
        Err(_) => {
            return Err(());
        }
    };

    return Ok(Location {
        path: path,
        line: line,
    });
}

fn parse_gcc<'a, T>(lines: T) -> Vec<Entry>
where
    T: IntoIterator<Item = &'a str>,
{
    let tokenized_lines = lines
        .into_iter()
        .map(|s| s.split_whitespace().collect::<Vec<&str>>())
        .collect::<Vec<Vec<&str>>>();

    let mut entries = Vec::new();

    for line_tokens in tokenized_lines {
        let mut message = String::new();
        let mut location = None;
        let mut severity = DiagnosticSeverity::ERROR;

        let mut token_iter = line_tokens.into_iter().peekable();

        while let Some(token) = token_iter.next() {
            let location_result = get_location_from_token(token, 3);
            match location_result {
                Ok(loc) => {
                    location = Some(loc);

                    if let Some(sever) = token_iter.peek() {
                        match *sever {
                            "warning:" => {
                                severity = DiagnosticSeverity::WARNING;
                                token_iter.next();
                            }
                            "note:" => {
                                severity = DiagnosticSeverity::HINT;
                                token_iter.next();
                            }
                            "error:" => {
                                severity = DiagnosticSeverity::ERROR;
                                token_iter.next();
                            }
                            _ => severity = DiagnosticSeverity::ERROR,
                        }
                    }

                    // NOTE(szulf): discard any messages before the file location
                    message.clear();
                }
                Err(_) => {
                    if message.len() != 0 {
                        message.push_str(" ");
                    }
                    message.push_str(token);
                }
            }
        }

        match location {
            Some(loc) => {
                entries.push(Entry::new(loc, message, severity));
            }
            None => {}
        }
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
        MakeFormatType::Gcc | MakeFormatType::Clang => parse_gcc(lines),
        MakeFormatType::Msvc => parse_msvc(lines),
    }
}

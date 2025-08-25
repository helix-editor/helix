use crate::commands::{goto_location, Context};
use crate::ui::{Picker, PickerColumn};
use helix_lsp::lsp::DiagnosticSeverity;
use helix_view::{
    make::{Entry, Location},
    theme::Style,
};
use regex::RegexBuilder;
use std::path::PathBuf;
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

fn parse_with_regex(source: &str, regex: &str) -> Vec<Entry> {
    let regex = RegexBuilder::new(regex).multi_line(true).build().unwrap();

    let mut results = Vec::new();

    for cap in regex.captures_iter(source) {
        log::debug!("capture: {:?}", cap);

        let Some(path) = cap.name("path") else {
            continue;
        };
        let Some(line) = cap.name("line") else {
            continue;
        };

        let location = Location {
            path: path.as_str().into(),
            line: line.as_str().parse::<usize>().unwrap() - 1,
        };

        let severity = match cap.name("severity").map(|c| c.as_str()).unwrap_or_default() {
            "warning" => DiagnosticSeverity::WARNING,
            "note" => DiagnosticSeverity::HINT,
            "error" | _ => DiagnosticSeverity::ERROR,
        };

        let Some(message) = cap.name("message") else {
            continue;
        };

        results.push(Entry::new(location, message.as_str().to_owned(), severity));
    }

    results
}

fn parse_default(_source: &str) -> Vec<Entry> {
    todo!();
}

fn parse_rust(_source: &str) -> Vec<Entry> {
    todo!();
}

fn parse_gcc(source: &str) -> Vec<Entry> {
    parse_with_regex(
        source,
        r"^(?P<path>[^:\n\s]+)(?::(?P<line>\d+))?(?::\d+)?(?::\([^)]+\))?:\s(?P<severity>error|warning|note)?:?\s?(?P<message>.+)$",
    )
}

fn parse_msvc(_source: &str) -> Vec<Entry> {
    todo!();
}

pub fn parse(format_type: MakeFormatType, source: &str) -> Vec<Entry> {
    match format_type {
        MakeFormatType::Default => parse_default(source),
        MakeFormatType::Rust => parse_rust(source),
        MakeFormatType::Gcc | MakeFormatType::Clang => parse_gcc(source),
        MakeFormatType::Msvc => parse_msvc(source),
    }
}

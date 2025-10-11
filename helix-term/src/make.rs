use crate::commands::{goto_location, Context};
use crate::ui::{Picker, PickerColumn};
use helix_core::diagnostic::Severity;
use helix_core::regex::RegexBuilder;
use helix_view::{
    make::{Entry, FormatType, Location},
    theme::Style,
};
use std::path::PathBuf;
use tui::text::Span;

// TODO(szulf): figure out how to display messages from the make_list the same way as diagnostics
// and make it togglable in the config i think(off by default i think)

// TODO(szulf): add keybindings for going to next/prev item in make list

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
                Severity::Hint => Span::styled("HINT", data.hint),
                Severity::Info => Span::styled("INFO", data.info),
                Severity::Warning => Span::styled("WARN", data.warning),
                Severity::Error => Span::styled("ERROR", data.error),
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

fn parse_with_regex(source: &str, regex: &str) -> Vec<Entry> {
    let regex = RegexBuilder::new(regex).multi_line(true).build().unwrap();
    let mut results = Vec::new();

    for cap in regex.captures_iter(source) {
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
            "warning" => Severity::Warning,
            "note" | "help" => Severity::Hint,
            "error" | _ => Severity::Error,
        };

        let Some(message) = cap.name("message") else {
            continue;
        };

        results.push(Entry::new(location, message.as_str().to_owned(), severity));
    }

    results
}

fn parse_rust(source: &str) -> Vec<Entry> {
    parse_with_regex(
        source,
        r"^(?P<severity>help|warning|error)(?:\[.+\])?:?\s(?P<message>.+)\n\s+-->\s(?P<path>[^:\n\s]+):(?P<line>\d+):(\d+)$",
    )
}

fn parse_gcc(source: &str) -> Vec<Entry> {
    parse_with_regex(
        source,
        r"^(?P<path>[^:\n\s]+)(?::(?P<line>\d+))?(?::\d+)?(?::\([^)]+\))?:\s(?P<severity>error|warning|note)?:?\s?(?P<message>.+)$",
    )
}

fn parse_msvc(source: &str) -> Vec<Entry> {
    parse_with_regex(
        source,
        r"^<(?P<path>.+)>\((?P<line>\d+)\):\s(?P<severity>error|warning|note)(?:[^:]+)?:\s(?P<message>.+)$",
    )
}

pub fn parse(format_type: &FormatType, source: &str) -> Vec<Entry> {
    match format_type {
        FormatType::Rust => parse_rust(source),
        FormatType::Gcc | FormatType::Clang => parse_gcc(source),
        FormatType::Msvc => parse_msvc(source),
    }
}

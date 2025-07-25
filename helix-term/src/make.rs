use crate::commands;
use crate::commands::{jump_to_location, Context};
use crate::ui::{Picker, PickerColumn};
use helix_view::make::{Entry, Location};
use std::{path::PathBuf, str::FromStr};

fn make_location_to_location(location: &Location) -> commands::Location {
    // TODO(szulf): take offet_encoding as an argument
    commands::Location::new(
        location.path.clone(),
        location.range.clone(),
        helix_lsp::OffsetEncoding::Utf8,
    )
}

#[derive(Debug, Clone)]
pub struct MakePickerData {
    root: PathBuf,
}

type MakePicker = Picker<Entry, MakePickerData>;

pub fn make_picker(cx: &Context, root: PathBuf) -> MakePicker {
    let options = cx.editor.make_list.clone().into_iter();

    let data = MakePickerData { root: root };

    let columns = vec![
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
        // TODO(szulf): change value to something else
        PickerColumn::new("value", |entry: &Entry, _data: &MakePickerData| {
            entry.value.err_msg.clone().into()
        }),
    ];

    Picker::new(columns, 0, options, data, move |cx, item, action| {
        jump_to_location(
            cx.editor,
            &make_location_to_location(&item.location),
            action,
        );
    })
}

pub enum MakeFormatType {
    Rust,
    Gcc,
    Clang,
    Msvc,
}

impl FromStr for MakeFormatType {
    // TODO(szulf): change this later
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rust" => Ok(MakeFormatType::Rust),
            "gcc" => Ok(MakeFormatType::Gcc),
            "clang" => Ok(MakeFormatType::Clang),
            "msvc" => Ok(MakeFormatType::Msvc),
            _ => Err(()),
        }
    }
}

pub fn format_rust(_source: &str) -> Vec<Entry> {
    return vec![];
}

pub fn format_gcc(_source: &str) -> Vec<Entry> {
    return vec![];
}

pub fn format_clang(_source: &str) -> Vec<Entry> {
    return vec![];
}

pub fn format_msvc(_source: &str) -> Vec<Entry> {
    return vec![];
}

pub fn format(format_type: MakeFormatType, source: &str) -> Vec<Entry> {
    match format_type {
        MakeFormatType::Rust => format_rust(source),
        MakeFormatType::Gcc => format_gcc(source),
        MakeFormatType::Clang => format_clang(source),
        MakeFormatType::Msvc => format_msvc(source),
    }
}

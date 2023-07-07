use crate::{
    clipboard::ClipboardType, document::SCRATCH_BUFFER_NAME, editor::focus::EditorFocus,
    register::Register, Editor,
};
use std::borrow::Cow;

pub const SELECTION_INDICES: Register = Register::from_char('#');
pub const SELECTION_CONTENT: Register = Register::from_char('.');
pub const DOCUMENT_PATH: Register = Register::from_char('%');
pub const SYSTEM_CLIPBOARD: Register = Register::from_char('*');
pub const PRIMARY_CLIPBOARD: Register = Register::from_char('+');

pub const CONTEXT_REGISTERS: [Register; 5] = [
    SELECTION_INDICES,
    SELECTION_CONTENT,
    DOCUMENT_PATH,
    SYSTEM_CLIPBOARD,
    PRIMARY_CLIPBOARD,
];

pub const WRITABLE_CONTEXT_REGISTERS: [Register; 2] = [SYSTEM_CLIPBOARD, PRIMARY_CLIPBOARD];
pub const NON_WRITABLE_CONTEXT_REGISTERS: [Register; 3] =
    [SELECTION_INDICES, SELECTION_CONTENT, DOCUMENT_PATH];

trait ContextRegister {
    fn read(editor: &Editor) -> Cow<[String]>;
}

trait WritableContextRegister {
    fn write(editor: &mut Editor, values: Vec<String>);
}

pub fn context_register_read<'a>(editor: &'a Editor, register: &Register) -> Cow<'a, [String]> {
    match *register {
        SELECTION_INDICES => SelectionIndices::read(editor),
        SELECTION_CONTENT => SelectionContent::read(editor),
        DOCUMENT_PATH => DocumentPath::read(editor),
        SYSTEM_CLIPBOARD => SystemClipboard::read(editor),
        PRIMARY_CLIPBOARD => SystemPrimary::read(editor),
        _ => unreachable!(),
    }
}

pub fn context_register_write(editor: &mut Editor, register: &Register, values: Vec<String>) {
    match *register {
        SYSTEM_CLIPBOARD => SystemClipboard::write(editor, values),
        PRIMARY_CLIPBOARD => SystemPrimary::write(editor, values),
        _ => unreachable!(),
    }
}

pub struct SelectionIndices;
impl ContextRegister for SelectionIndices {
    fn read(editor: &Editor) -> Cow<[String]> {
        let (focused_view, focused_document) = &editor.focused_view_doc();

        focused_document
            .selection(focused_view.id)
            .iter()
            .enumerate()
            .map(|(index, _)| (index + 1).to_string())
            .collect()
    }
}

pub struct SelectionContent;
impl ContextRegister for SelectionContent {
    fn read(editor: &Editor) -> Cow<[String]> {
        let (current_view, current_document) = &editor.focused_view_doc();
        current_document
            .selection(current_view.id)
            .fragments(current_document.text().slice(..))
            .map(Cow::into_owned)
            .collect()
    }
}

pub struct DocumentPath;
impl ContextRegister for DocumentPath {
    fn read(editor: &Editor) -> Cow<[String]> {
        vec![editor
            .focused_document()
            .path()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| SCRATCH_BUFFER_NAME.into())]
        .into()
    }
}

pub struct SystemClipboard;
impl ContextRegister for SystemClipboard {
    fn read(editor: &Editor) -> Cow<[String]> {
        read_clipboard_register(editor, &ClipboardType::Clipboard)
    }
}

impl WritableContextRegister for SystemClipboard {
    fn write(editor: &mut Editor, values: Vec<String>) {
        write_clipboard_register(editor, &ClipboardType::Clipboard, values)
    }
}

pub struct SystemPrimary;
impl ContextRegister for SystemPrimary {
    fn read(editor: &Editor) -> Cow<[String]> {
        read_clipboard_register(editor, &ClipboardType::Selection)
    }
}
impl WritableContextRegister for SystemPrimary {
    fn write(editor: &mut Editor, values: Vec<String>) {
        write_clipboard_register(editor, &ClipboardType::Selection, values)
    }
}

fn read_clipboard_register<'a>(
    editor: &'a Editor,
    clipboard_type: &ClipboardType,
) -> Cow<'a, [String]> {
    match editor.clipboard_provider.get_contents(*clipboard_type) {
        Ok(contents) => vec![contents].into(),
        Err(err) => {
            // TODO: error handling in clipboard.rs
            log::error!(
                "Failed to read {} clipboard: {}",
                match clipboard_type {
                    ClipboardType::Clipboard => "system",
                    ClipboardType::Selection => "primary",
                },
                err
            );
            Default::default()
        }
    }
}

fn write_clipboard_register(
    editor: &mut Editor,
    clipboard_type: &ClipboardType,
    values: Vec<String>,
) {
    // TODO: error handling in clipboard.rs
    let _ = editor.clipboard_provider.set_contents(
        values.join(editor.focused_document().line_ending.as_str()),
        *clipboard_type,
    );
}

use helix_core::{tree_sitter::Query, Selection, Uri};
use helix_view::{align_view, Align, DocumentId};

use crate::ui::{overlay::overlaid, picker::PathOrId, Picker, PickerColumn};

use super::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SymbolKind {
    Function,
    Macro,
    Module,
    Constant,
    Struct,
    Interface,
    Type,
    Class,
}

impl SymbolKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Macro => "macro",
            Self::Module => "module",
            Self::Constant => "constant",
            Self::Struct => "struct",
            Self::Interface => "interface",
            Self::Type => "type",
            Self::Class => "class",
        }
    }
}

fn definition_symbol_kind_for_capture(symbols: &Query, capture_index: usize) -> Option<SymbolKind> {
    match symbols.capture_names()[capture_index] {
        "definition.function" => Some(SymbolKind::Function),
        "definition.macro" => Some(SymbolKind::Macro),
        "definition.module" => Some(SymbolKind::Module),
        "definition.constant" => Some(SymbolKind::Constant),
        "definition.struct" => Some(SymbolKind::Struct),
        "definition.interface" => Some(SymbolKind::Interface),
        "definition.type" => Some(SymbolKind::Type),
        "definition.class" => Some(SymbolKind::Class),
        _ => None,
    }
}

// NOTE: Uri is cheap to clone and DocumentId is Copy
#[derive(Debug, Clone)]
enum UriOrDocumentId {
    // TODO: the workspace symbol picker will take advantage of this.
    #[allow(dead_code)]
    Uri(Uri),
    Id(DocumentId),
}

impl UriOrDocumentId {
    fn path_or_id(&self) -> Option<PathOrId<'_>> {
        match self {
            Self::Id(id) => Some(PathOrId::Id(*id)),
            Self::Uri(uri) => uri.as_path().map(PathOrId::Path),
        }
    }
}

#[derive(Debug)]
struct Symbol {
    kind: SymbolKind,
    name: String,
    start: usize,
    end: usize,
    start_line: usize,
    end_line: usize,
    doc: UriOrDocumentId,
}

pub fn syntax_symbol_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);
    let Some((syntax, lang_config)) = doc.syntax().zip(doc.language_config()) else {
        cx.editor
            .set_error("Syntax tree is not available on this buffer");
        return;
    };
    let Some(symbols_query) = lang_config.symbols_query() else {
        cx.editor
            .set_error("Syntax-based symbols information not available for this language");
        return;
    };

    let doc_id = doc.id();
    let text = doc.text();

    let columns = vec![
        PickerColumn::new("kind", |symbol: &Symbol, _| symbol.kind.as_str().into()),
        PickerColumn::new("name", |symbol: &Symbol, _| symbol.name.as_str().into()),
    ];

    let symbols = syntax
        .captures(symbols_query, text.slice(..), None)
        .filter_map(move |(match_, capture_index)| {
            let capture = match_.captures[capture_index];
            let kind = definition_symbol_kind_for_capture(symbols_query, capture.index as usize)?;
            let node = capture.node;
            let start = text.byte_to_char(node.start_byte());
            let end = text.byte_to_char(node.end_byte());

            Some(Symbol {
                kind,
                name: text.slice(start..end).to_string(),
                start,
                end,

                start_line: text.char_to_line(start),
                end_line: text.char_to_line(end),
                doc: UriOrDocumentId::Id(doc_id),
            })
        });

    let picker = Picker::new(
        columns,
        1, // name
        symbols,
        (),
        move |cx, symbol, action| {
            cx.editor.switch(doc_id, action);
            let view = view_mut!(cx.editor);
            let doc = doc_mut!(cx.editor, &doc_id);
            doc.set_selection(view.id, Selection::single(symbol.start, symbol.end));
            if action.align_view(view, doc.id()) {
                align_view(doc, view, Align::Center)
            }
        },
    )
    .with_preview(|_editor, symbol| {
        Some((
            symbol.doc.path_or_id()?,
            Some((symbol.start_line, symbol.end_line)),
        ))
    })
    .truncate_start(false);

    cx.push_layer(Box::new(overlaid(picker)));
}

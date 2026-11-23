use helix_lsp::{
    lsp::{self, DocumentSymbol},
    OffsetEncoding,
};

use super::Document;

pub struct DocumentSymbols {
    pub tree: Vec<ThinDocumentSymbol>,
    pub offset_encoding: OffsetEncoding,
}

#[derive(Debug, Clone)]
pub struct ThinDocumentSymbol {
    pub name: Box<str>,
    pub kind: lsp::SymbolKind,
    pub range: lsp::Range,
    pub children: Option<Box<[Self]>>,
}

impl From<DocumentSymbol> for ThinDocumentSymbol {
    #[inline]
    fn from(symbol: DocumentSymbol) -> Self {
        Self {
            name: symbol.name.into(),
            kind: symbol.kind,
            range: symbol.range,
            children: symbol.children.map(|children| {
                let mut vec = Vec::with_capacity(children.len());
                vec.extend(children.into_iter().map(Self::from));
                vec.into_boxed_slice()
            }),
        }
    }
}

impl Document {
    #[inline(always)]
    pub fn set_document_symbols(
        &mut self,
        symbols: Vec<DocumentSymbol>,
        offset_encoding: OffsetEncoding,
    ) {
        self.symbols = Some(DocumentSymbols {
            tree: {
                let mut tree = Vec::with_capacity(symbols.len());
                tree.extend(symbols.into_iter().map(ThinDocumentSymbol::from));
                tree
            },
            offset_encoding,
        });
    }

    #[inline]
    pub fn clear_document_symbols(&mut self) {
        self.symbols = None;
        // As there are no more symbols, we should clear all breadcrumbs as well.
        self.clear_breadcrumbs();
    }
}

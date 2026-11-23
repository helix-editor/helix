use helix_lsp::lsp;

use crate::{document::symbols::ThinDocumentSymbol, ViewId};

use super::Document;

#[derive(Debug, Clone, Default)]
pub struct Breadcrumbs(Vec<Crumb>);

impl Breadcrumbs {
    #[inline]
    pub fn push(&mut self, crumb: Crumb) {
        self.0.push(crumb);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.0.len()
    }
}

impl<'a> IntoIterator for &'a Breadcrumbs {
    type Item = &'a Crumb;
    type IntoIter = std::slice::Iter<'a, Crumb>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, Clone)]
pub struct Crumb {
    pub name: Box<str>,
    pub kind: lsp::SymbolKind,
}

impl From<&ThinDocumentSymbol> for Crumb {
    #[inline]
    fn from(symbol: &ThinDocumentSymbol) -> Self {
        Self {
            name: symbol.name.clone(),
            kind: symbol.kind,
        }
    }
}

impl Document {
    #[inline]
    pub fn clear_breadcrumbs(&mut self) {
        self.breadcrumbs.clear();
    }

    // For all non-hotpaths, we use this function to prevent code bloat.
    #[inline(never)]
    pub fn update_breadcrumbs_for_view(&mut self, view_id: ViewId) {
        self.update_breadcrumbs_for_view_inlined(view_id);
    }

    // We want to make sure this is inlined in the hotpath (e.g., cursor position change).
    #[inline(always)]
    pub fn update_breadcrumbs_for_view_inlined(&mut self, view_id: ViewId) {
        #[inline(always)]
        const fn in_range(pos: lsp::Position, range: lsp::Range) -> bool {
            // PERF:
            // Line-based filtering is the most effective early exit indicator,
            // so do first, before other evaluations; this should be friendly to
            // the CPU branch predictor.
            if pos.line < range.start.line || pos.line > range.end.line {
                return false;
            }

            // Check if the cursor position is "in" the symbols "depth".
            //
            // In the context of breadcrumbs, this would be the difference between
            // if the cursor is in an impl block or in an impl block and in a
            // function of the impl block (`|` is the cursor):
            //
            // ```rust
            // impl Foo {
            //     f|n bar() {} // In `bar`: impl Foo > bar
            //
            //   | fn baz() {} // Not in `baz`: impl Foo
            //
            //     fn quux() {} | // Not in `quux`: impl Foo
            // }
            // ```
            if pos.line == range.start.line && pos.character < range.start.character {
                return false;
            }
            if pos.line == range.end.line && pos.character > range.end.character {
                return false;
            }

            true
        }

        let Some(symbols) = self.symbols.as_ref() else {
            return;
        };

        let position = self.position(view_id, symbols.offset_encoding);

        let breadcrumb = {
            let breadcrumb = self.breadcrumbs.entry(view_id).or_default();
            breadcrumb.clear();
            breadcrumb
        };

        let mut current = symbols.tree.as_slice();

        while let Some(symbol) = current
            .iter()
            .find(|&symbol| in_range(position, symbol.range))
        {
            breadcrumb.push(Crumb::from(symbol));
            match symbol.children.as_deref() {
                Some(children) => current = children,
                _ => break,
            }
        }
    }
}

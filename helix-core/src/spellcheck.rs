use crate::selection::Range;
use crate::syntax::LanguageConfiguration;
use crate::RopeSlice;
use tree_sitter::{Node, QueryCursor};

pub fn spellcheck_treesitter(
    doc_tree: Node,
    doc_slice: RopeSlice,
    lang_config: &LanguageConfiguration,
) -> Option<Vec<Range>> {
    let mut cursor = QueryCursor::new();
    let ranges: Vec<Range> = lang_config
        .spellcheck_query()?
        .capture_nodes("spell", doc_tree, doc_slice, &mut cursor)?
        .map(|node| {
            let start_char = doc_slice.byte_to_char(node.start_byte());
            let end_char = doc_slice.byte_to_char(node.end_byte());
            Range::new(start_char, end_char)
        })
        .collect();
    Some(ranges)
}

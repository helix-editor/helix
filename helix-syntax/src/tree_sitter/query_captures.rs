use std::ptr::{self, NonNull};

use regex_cursor::Cursor;

use crate::tree_sitter::query::Query;
use crate::tree_sitter::syntax_tree_node::SyntaxTreeNodeRaw;

enum QueryCursorData {}

pub struct QueryCaptures<'a> {
    query: &'a Query,
    query_cursor: &'a mut QueryCursorData,
    text_cursor: regex_cursor::RopeyCursor<'a>,
}

impl<C: Cursor> QueryCaptures<'_, C> {
    fn next(&mut self) {
        let mut query_match = TSQueryMatch {
            id: 0,
            pattern_index: 0,
            capture_count: 0,
            captures: ptr::null(),
        };
        let mut capture_idx = 0;
        loop {
            let success = unsafe {
                ts_query_cursor_next_capture(
                    &mut self.query_cursor,
                    &mut query_match,
                    &mut capture_idx,
                )
            };
            if !success {
                break;
            }
        }
        let mut input = regex_cursor::Input::new(self.text_cursor.clone());
    }
}

#[repr(C)]
#[derive(Debug)]
struct TSQueryCapture {
    node: SyntaxTreeNodeRaw,
    index: u32,
}

#[repr(C)]
#[derive(Debug)]
struct TSQueryMatch {
    id: u32,
    pattern_index: u16,
    capture_count: u16,
    captures: *const TSQueryCapture,
}

extern "C" {
    /// Advance to the next capture of the currently running query.
    /// If there is a capture, write its match to `*match` and its index within
    /// the matche's capture list to `*capture_index`. Otherwise, return `false`.
    fn ts_query_cursor_next_capture(
        self_: &mut QueryCursorData,
        match_: &mut TSQueryMatch,
        capture_index: &mut u32,
    ) -> bool;
}

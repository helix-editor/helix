use core::slice;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::replace;
use std::ops::Range;
use std::ptr::{self, NonNull};

use crate::tree_sitter::query::{Capture, Pattern, Query, QueryData};
use crate::tree_sitter::syntax_tree_node::SyntaxTreeNodeRaw;
use crate::tree_sitter::{SyntaxTree, SyntaxTreeNode, TsInput};

enum QueryCursorData {}

thread_local! {
    static CURSOR_CACHE: UnsafeCell<Vec<InactiveQueryCursor>> = UnsafeCell::new(Vec::with_capacity(8));
}

/// SAFETY: must not call itself recuresively
unsafe fn with_cache<T>(f: impl FnOnce(&mut Vec<InactiveQueryCursor>) -> T) -> T {
    CURSOR_CACHE.with(|cache| f(&mut *cache.get()))
}

pub struct QueryCursor<'a, 'tree, I: TsInput> {
    query: &'a Query,
    ptr: *mut QueryCursorData,
    tree: PhantomData<&'tree SyntaxTree>,
    input: I,
}

impl<'tree, I: TsInput> QueryCursor<'_, 'tree, I> {
    pub fn next_match(&mut self) -> Option<QueryMatch<'_, 'tree>> {
        let mut query_match = TSQueryMatch {
            id: 0,
            pattern_index: 0,
            capture_count: 0,
            captures: ptr::null(),
        };
        loop {
            let success = unsafe { ts_query_cursor_next_match(self.ptr, &mut query_match) };
            if !success {
                return None;
            }
            let matched_nodes = unsafe {
                slice::from_raw_parts(
                    query_match.captures.cast(),
                    query_match.capture_count as usize,
                )
            };
            let satisfies_predicates = self
                .query
                .pattern_text_predicates(query_match.pattern_index)
                .iter()
                .all(|predicate| predicate.satsified(&mut self.input, matched_nodes, self.query));
            if satisfies_predicates {
                let res = QueryMatch {
                    id: query_match.id,
                    pattern: Pattern(query_match.pattern_index as u32),
                    matched_nodes,
                    query_cursor: unsafe { &mut *self.ptr },
                    _tree: PhantomData,
                };
                return Some(res);
            }
        }
    }

    pub fn next_matched_node(&mut self) -> Option<(QueryMatch<'_, 'tree>, MatchedNodeIdx)> {
        let mut query_match = TSQueryMatch {
            id: 0,
            pattern_index: 0,
            capture_count: 0,
            captures: ptr::null(),
        };
        let mut capture_idx = 0;
        loop {
            let success = unsafe {
                ts_query_cursor_next_capture(self.ptr, &mut query_match, &mut capture_idx)
            };
            if !success {
                return None;
            }
            let matched_nodes = unsafe {
                slice::from_raw_parts(
                    query_match.captures.cast(),
                    query_match.capture_count as usize,
                )
            };
            let satisfies_predicates = self
                .query
                .pattern_text_predicates(query_match.pattern_index)
                .iter()
                .all(|predicate| predicate.satsified(&mut self.input, matched_nodes, self.query));
            if satisfies_predicates {
                let res = QueryMatch {
                    id: query_match.id,
                    pattern: Pattern(query_match.pattern_index as u32),
                    matched_nodes,
                    query_cursor: unsafe { &mut *self.ptr },
                    _tree: PhantomData,
                };
                return Some((res, capture_idx));
            } else {
                unsafe {
                    ts_query_cursor_remove_match(self.ptr, query_match.id);
                }
            }
        }
    }

    pub fn set_byte_range(&mut self, range: Range<usize>) {
        unsafe {
            ts_query_cursor_set_byte_range(self.ptr, range.start as u32, range.end as u32);
        }
    }

    pub fn reuse(mut self) -> InactiveQueryCursor {
        let ptr = replace(&mut self.ptr, ptr::null_mut());
        InactiveQueryCursor {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
        }
    }
}

impl<I: TsInput> Drop for QueryCursor<'_, '_, I> {
    fn drop(&mut self) {
        // we allow moving the cursor data out so we need the null check here
        // would be cleaner with a subtype but doesn't really matter at the end of the day
        if let Some(ptr) = NonNull::new(self.ptr) {
            unsafe { with_cache(|cache| cache.push(InactiveQueryCursor { ptr })) }
        }
    }
}

/// A query cursor that is not actively associated with a query
pub struct InactiveQueryCursor {
    ptr: NonNull<QueryCursorData>,
}

impl InactiveQueryCursor {
    pub fn new() -> Self {
        unsafe {
            with_cache(|cache| {
                cache.pop().unwrap_or_else(|| InactiveQueryCursor {
                    ptr: NonNull::new_unchecked(ts_query_cursor_new()),
                })
            })
        }
    }

    /// Return the maximum number of in-progress matches for this cursor.
    #[doc(alias = "ts_query_cursor_match_limit")]
    #[must_use]
    pub fn match_limit(&self) -> u32 {
        unsafe { ts_query_cursor_match_limit(self.ptr.as_ptr()) }
    }

    /// Set the maximum number of in-progress matches for this cursor.  The
    /// limit must be > 0 and <= 65536.
    #[doc(alias = "ts_query_cursor_set_match_limit")]
    pub fn set_match_limit(&mut self, limit: u32) {
        unsafe {
            ts_query_cursor_set_match_limit(self.ptr.as_ptr(), limit);
        }
    }

    /// Check if, on its last execution, this cursor exceeded its maximum number
    /// of in-progress matches.
    #[doc(alias = "ts_query_cursor_did_exceed_match_limit")]
    #[must_use]
    pub fn did_exceed_match_limit(&self) -> bool {
        unsafe { ts_query_cursor_did_exceed_match_limit(self.ptr.as_ptr()) }
    }

    pub fn set_byte_range(&mut self, range: Range<usize>) {
        unsafe {
            ts_query_cursor_set_byte_range(self.ptr.as_ptr(), range.start as u32, range.end as u32);
        }
    }

    pub fn execute_query<'a, 'tree, I: TsInput>(
        self,
        query: &'a Query,
        node: &SyntaxTreeNode<'tree>,
        input: I,
    ) -> QueryCursor<'a, 'tree, I> {
        let ptr = self.ptr.as_ptr();
        unsafe { ts_query_cursor_exec(self.ptr.as_ptr(), query.raw.as_ref(), node.as_raw()) };
        QueryCursor {
            query,
            ptr,
            tree: PhantomData,
            input,
        }
    }
}

impl Drop for InactiveQueryCursor {
    fn drop(&mut self) {
        unsafe { ts_query_cursor_delete(self.ptr.as_ptr()) }
    }
}

pub type MatchedNodeIdx = u32;

#[repr(C)]
#[derive(Clone)]
pub struct MatchedNode<'tree> {
    pub syntax_node: SyntaxTreeNode<'tree>,
    pub capture: Capture,
}

pub struct QueryMatch<'cursor, 'tree> {
    id: u32,
    pattern: Pattern,
    matched_nodes: &'cursor [MatchedNode<'tree>],
    query_cursor: &'cursor mut QueryCursorData,
    _tree: PhantomData<&'tree super::SyntaxTree>,
}

impl<'tree> QueryMatch<'_, 'tree> {
    pub fn matched_nodes(&self) -> impl Iterator<Item = &MatchedNode<'tree>> {
        self.matched_nodes.iter()
    }

    pub fn nodes_for_capture(
        &self,
        capture: Capture,
    ) -> impl Iterator<Item = &SyntaxTreeNode<'tree>> {
        self.matched_nodes
            .iter()
            .filter(move |mat| mat.capture == capture)
            .map(|mat| &mat.syntax_node)
    }

    pub fn matched_node(&self, i: MatchedNodeIdx) -> &MatchedNode {
        &self.matched_nodes[i as usize]
    }

    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    #[must_use]
    pub const fn pattern(&self) -> Pattern {
        self.pattern
    }

    #[doc(alias = "ts_query_cursor_remove_match")]
    /// removes this match from the cursor so that further captures
    /// from its cursor so that future captures that belong to this match
    /// are no longer returned by capture iterators
    pub fn remove(self) {
        unsafe {
            ts_query_cursor_remove_match(self.query_cursor, self.id);
        }
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
        self_: *mut QueryCursorData,
        match_: &mut TSQueryMatch,
        capture_index: &mut u32,
    ) -> bool;

    /// Advance to the next match of the currently running query.
    ///
    /// If there is a match, write it to `*match` and return `true`.
    /// Otherwise, return `false`.
    pub fn ts_query_cursor_next_match(
        self_: *mut QueryCursorData,
        match_: &mut TSQueryMatch,
    ) -> bool;
    fn ts_query_cursor_remove_match(self_: *mut QueryCursorData, match_id: u32);
    /// Delete a query cursor, freeing all of the memory that it used
    pub fn ts_query_cursor_delete(self_: *mut QueryCursorData);
    /// Create a new cursor for executing a given query.
    /// The cursor stores the state that is needed to iteratively search
    /// for matches. To use the query cursor, first call [`ts_query_cursor_exec`]
    /// to start running a given query on a given syntax node. Then, there are
    /// two options for consuming the results of the query:
    /// 1. Repeatedly call [`ts_query_cursor_next_match`] to iterate over all of the
    ///    *matches* in the order that they were found. Each match contains the
    ///    index of the pattern that matched, and an array of captures. Because
    ///    multiple patterns can match the same set of nodes, one match may contain
    ///    captures that appear *before* some of the captures from a previous match.
    /// 2. Repeatedly call [`ts_query_cursor_next_capture`] to iterate over all of the
    ///    individual *captures* in the order that they appear. This is useful if
    ///    don't care about which pattern matched, and just want a single ordered
    ///    sequence of captures.
    /// If you don't care about consuming all of the results, you can stop calling
    /// [`ts_query_cursor_next_match`] or [`ts_query_cursor_next_capture`] at any point.
    ///  You can then start executing another query on another node by calling
    ///  [`ts_query_cursor_exec`] again."]
    pub fn ts_query_cursor_new() -> *mut QueryCursorData;

    /// Start running a given query on a given node.
    pub fn ts_query_cursor_exec(
        self_: *mut QueryCursorData,
        query: &QueryData,
        node: SyntaxTreeNodeRaw,
    );
    /// Manage the maximum number of in-progress matches allowed by this query
    /// cursor.
    ///
    /// Query cursors have an optional maximum capacity for storing lists of
    /// in-progress captures. If this capacity is exceeded, then the
    /// earliest-starting match will silently be dropped to make room for further
    /// matches. This maximum capacity is optional — by default, query cursors allow
    /// any number of pending matches, dynamically allocating new space for them as
    /// needed as the query is executed.
    pub fn ts_query_cursor_did_exceed_match_limit(self_: *const QueryCursorData) -> bool;
    pub fn ts_query_cursor_match_limit(self_: *const QueryCursorData) -> u32;
    pub fn ts_query_cursor_set_match_limit(self_: *mut QueryCursorData, limit: u32);
    /// Set the range of bytes or (row, column) positions in which the query
    /// will be executed.
    pub fn ts_query_cursor_set_byte_range(
        self_: *mut QueryCursorData,
        start_byte: u32,
        end_byte: u32,
    );

}

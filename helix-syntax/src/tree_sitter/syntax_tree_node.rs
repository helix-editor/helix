use std::ffi::c_void;
use std::marker::PhantomData;
use std::ops::Range;
use std::ptr::NonNull;

use crate::tree_sitter::syntax_tree::SyntaxTree;
use crate::tree_sitter::Grammar;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct SyntaxTreeNodeRaw {
    context: [u32; 4],
    id: *const c_void,
    tree: *const c_void,
}

impl From<SyntaxTreeNode<'_>> for SyntaxTreeNodeRaw {
    fn from(node: SyntaxTreeNode) -> SyntaxTreeNodeRaw {
        SyntaxTreeNodeRaw {
            context: node.context,
            id: node.id.as_ptr(),
            tree: node.tree.as_ptr(),
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct SyntaxTreeNode<'tree> {
    context: [u32; 4],
    id: NonNull<c_void>,
    tree: NonNull<c_void>,
    _phantom: PhantomData<&'tree SyntaxTree>,
}

impl<'tree> SyntaxTreeNode<'tree> {
    #[inline]
    pub(super) unsafe fn from_raw(raw: SyntaxTreeNodeRaw) -> Option<Self> {
        Some(SyntaxTreeNode {
            context: raw.context,
            id: NonNull::new(raw.id as *mut _)?,
            tree: unsafe { NonNull::new_unchecked(raw.tree as *mut _) },
            _phantom: PhantomData,
        })
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> SyntaxTreeNodeRaw {
        SyntaxTreeNodeRaw {
            context: self.context,
            id: self.id.as_ptr(),
            tree: self.tree.as_ptr(),
        }
    }

    /// Get this node's type as a numerical id.
    #[inline]
    pub fn kind_id(&self) -> u16 {
        unsafe { ts_node_symbol(self.as_raw()) }
    }

    /// Get the [`Language`] that was used to parse this node's syntax tree.
    #[inline]
    pub fn grammar(&self) -> Grammar {
        unsafe { ts_node_language(self.as_raw()) }
    }

    /// Check if this node is *named*.
    ///
    /// Named nodes correspond to named rules in the grammar, whereas
    /// *anonymous* nodes correspond to string literals in the grammar.
    #[inline]
    pub fn is_named(&self) -> bool {
        unsafe { ts_node_is_named(self.as_raw()) }
    }

    /// Check if this node is *missing*.
    ///
    /// Missing nodes are inserted by the parser in order to recover from
    /// certain kinds of syntax errors.
    #[inline]
    pub fn is_missing(&self) -> bool {
        unsafe { ts_node_is_missing(self.as_raw()) }
    }
    /// Get the byte offsets where this node starts.
    #[inline]
    pub fn start_byte(&self) -> usize {
        unsafe { ts_node_start_byte(self.as_raw()) as usize }
    }

    /// Get the byte offsets where this node end.
    #[inline]
    pub fn end_byte(&self) -> usize {
        unsafe { ts_node_end_byte(self.as_raw()) as usize }
    }

    /// Get the byte range of source code that this node represents.
    // TODO: use helix_stdx::Range once available
    #[inline]
    pub fn byte_range(&self) -> Range<usize> {
        self.start_byte()..self.end_byte()
    }

    /// Get the node's child at the given index, where zero represents the first
    /// child.
    ///
    /// This method is fairly fast, but its cost is technically log(i), so if
    /// you might be iterating over a long list of children, you should use
    /// [`SyntaxTreeNode::children`] instead.
    #[inline]
    pub fn child(&self, i: usize) -> Option<SyntaxTreeNode<'tree>> {
        unsafe { SyntaxTreeNode::from_raw(ts_node_child(self.as_raw(), i as u32)) }
    }

    /// Get this node's number of children.
    #[inline]
    pub fn child_count(&self) -> usize {
        unsafe { ts_node_child_count(self.as_raw()) as usize }
    }

    /// Get this node's *named* child at the given index.
    ///
    /// See also [`SyntaxTreeNode::is_named`].
    /// This method is fairly fast, but its cost is technically log(i), so if
    /// you might be iterating over a long list of children, you should use
    /// [`SyntaxTreeNode::named_children`] instead.
    #[inline]
    pub fn named_child(&self, i: usize) -> Option<SyntaxTreeNode<'tree>> {
        unsafe { SyntaxTreeNode::from_raw(ts_node_named_child(self.as_raw(), i as u32)) }
    }

    /// Get this node's number of *named* children.
    ///
    /// See also [`SyntaxTreeNode::is_named`].
    #[inline]
    pub fn named_child_count(&self) -> usize {
        unsafe { ts_node_named_child_count(self.as_raw()) as usize }
    }

    #[inline]
    unsafe fn map(
        &self,
        f: unsafe extern "C" fn(SyntaxTreeNodeRaw) -> SyntaxTreeNodeRaw,
    ) -> Option<SyntaxTreeNode<'tree>> {
        SyntaxTreeNode::from_raw(f(self.as_raw()))
    }

    /// Get this node's immediate parent.
    #[inline]
    pub fn parent(&self) -> Option<Self> {
        unsafe { self.map(ts_node_parent) }
    }

    /// Get this node's next sibling.
    #[inline]
    pub fn next_sibling(&self) -> Option<Self> {
        unsafe { self.map(ts_node_next_sibling) }
    }

    /// Get this node's previous sibling.
    #[inline]
    pub fn prev_sibling(&self) -> Option<Self> {
        unsafe { self.map(ts_node_prev_sibling) }
    }

    /// Get this node's next named sibling.
    #[inline]
    pub fn next_named_sibling(&self) -> Option<Self> {
        unsafe { self.map(ts_node_next_named_sibling) }
    }

    /// Get this node's previous named sibling.
    #[inline]
    pub fn prev_named_sibling(&self) -> Option<Self> {
        unsafe { self.map(ts_node_prev_named_sibling) }
    }

    /// Get the smallest node within this node that spans the given range.
    #[inline]
    pub fn descendant_for_byte_range(&self, start: usize, end: usize) -> Option<Self> {
        unsafe {
            Self::from_raw(ts_node_descendant_for_byte_range(
                self.as_raw(),
                start as u32,
                end as u32,
            ))
        }
    }

    /// Get the smallest named node within this node that spans the given range.
    #[inline]
    pub fn named_descendant_for_byte_range(&self, start: usize, end: usize) -> Option<Self> {
        unsafe {
            Self::from_raw(ts_node_named_descendant_for_byte_range(
                self.as_raw(),
                start as u32,
                end as u32,
            ))
        }
    }
    // /// Iterate over this node's children.
    // ///
    // /// A [`TreeCursor`] is used to retrieve the children efficiently. Obtain
    // /// a [`TreeCursor`] by calling [`Tree::walk`] or [`SyntaxTreeNode::walk`]. To avoid
    // /// unnecessary allocations, you should reuse the same cursor for
    // /// subsequent calls to this method.
    // ///
    // /// If you're walking the tree recursively, you may want to use the
    // /// [`TreeCursor`] APIs directly instead.
    // pub fn children<'cursor>(
    //     &self,
    //     cursor: &'cursor mut TreeCursor<'tree>,
    // ) -> impl ExactSizeIterator<Item = SyntaxTreeNode<'tree>> + 'cursor {
    //     cursor.reset(self.to_raw());
    //     cursor.goto_first_child();
    //     (0..self.child_count()).map(move |_| {
    //         let result = cursor.node();
    //         cursor.goto_next_sibling();
    //         result
    //     })
    // }
}

unsafe impl Send for SyntaxTreeNode<'_> {}
unsafe impl Sync for SyntaxTreeNode<'_> {}

extern "C" {
    /// Get the node's type as a numerical id.
    fn ts_node_symbol(node: SyntaxTreeNodeRaw) -> u16;

    /// Get the node's language.
    fn ts_node_language(node: SyntaxTreeNodeRaw) -> Grammar;

    /// Check if the node is *named*. Named nodes correspond to named rules in
    /// the grammar, whereas *anonymous* nodes correspond to string literals in
    /// the grammar
    fn ts_node_is_named(node: SyntaxTreeNodeRaw) -> bool;

    /// Check if the node is *missing*. Missing nodes are inserted by the parser
    /// in order to recover from certain kinds of syntax errors
    fn ts_node_is_missing(node: SyntaxTreeNodeRaw) -> bool;

    /// Get the node's immediate parent
    fn ts_node_parent(node: SyntaxTreeNodeRaw) -> SyntaxTreeNodeRaw;

    /// Get the node's child at the given index, where zero represents the first
    /// child
    fn ts_node_child(node: SyntaxTreeNodeRaw, child_index: u32) -> SyntaxTreeNodeRaw;

    /// Get the node's number of children
    fn ts_node_child_count(node: SyntaxTreeNodeRaw) -> u32;

    /// Get the node's *named* child at the given index. See also
    /// [`ts_node_is_named`]
    fn ts_node_named_child(node: SyntaxTreeNodeRaw, child_index: u32) -> SyntaxTreeNodeRaw;

    /// Get the node's number of *named* children. See also [`ts_node_is_named`]
    fn ts_node_named_child_count(node: SyntaxTreeNodeRaw) -> u32;

    /// Get the node's next sibling
    fn ts_node_next_sibling(node: SyntaxTreeNodeRaw) -> SyntaxTreeNodeRaw;

    fn ts_node_prev_sibling(node: SyntaxTreeNodeRaw) -> SyntaxTreeNodeRaw;

    /// Get the node's next *named* sibling
    fn ts_node_next_named_sibling(node: SyntaxTreeNodeRaw) -> SyntaxTreeNodeRaw;

    fn ts_node_prev_named_sibling(node: SyntaxTreeNodeRaw) -> SyntaxTreeNodeRaw;

    /// Get the smallest node within this node that spans the given range of
    /// bytes or (row, column) positions
    fn ts_node_descendant_for_byte_range(
        node: SyntaxTreeNodeRaw,

        start: u32,
        end: u32,
    ) -> SyntaxTreeNodeRaw;

    /// Get the smallest named node within this node that spans the given range
    /// of bytes or (row, column) positions
    fn ts_node_named_descendant_for_byte_range(
        node: SyntaxTreeNodeRaw,
        start: u32,
        end: u32,
    ) -> SyntaxTreeNodeRaw;

    /// Get the node's start byte.
    fn ts_node_start_byte(self_: SyntaxTreeNodeRaw) -> u32;

    /// Get the node's end byte.
    fn ts_node_end_byte(node: SyntaxTreeNodeRaw) -> u32;
}

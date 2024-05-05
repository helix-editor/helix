use std::fmt;
use std::ptr::NonNull;

use crate::tree_sitter::syntax_tree_node::{SyntaxTreeNode, SyntaxTreeNodeRaw};
use crate::tree_sitter::Point;

// opaque pointers
pub(super) enum SyntaxTreeData {}

pub struct SyntaxTree {
    ptr: NonNull<SyntaxTreeData>,
}

impl SyntaxTree {
    pub(super) unsafe fn from_raw(raw: NonNull<SyntaxTreeData>) -> SyntaxTree {
        SyntaxTree { ptr: raw }
    }

    pub(super) fn as_raw(&self) -> NonNull<SyntaxTreeData> {
        self.ptr
    }

    pub fn root_node(&self) -> SyntaxTreeNode<'_> {
        unsafe { SyntaxTreeNode::from_raw(ts_tree_root_node(self.ptr)).unwrap() }
    }

    pub fn edit(&mut self, edit: &InputEdit) {
        unsafe { ts_tree_edit(self.ptr, edit) }
    }
}

impl fmt::Debug for SyntaxTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{Tree {:?}}}", self.root_node())
    }
}

impl Drop for SyntaxTree {
    fn drop(&mut self) {
        unsafe { ts_tree_delete(self.ptr) }
    }
}

impl Clone for SyntaxTree {
    fn clone(&self) -> Self {
        unsafe {
            SyntaxTree {
                ptr: ts_tree_copy(self.ptr),
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct InputEdit {
    pub start_byte: u32,
    pub old_end_byte: u32,
    pub new_end_byte: u32,
    pub start_point: Point,
    pub old_end_point: Point,
    pub new_end_point: Point,
}

extern "C" {
    /// Create a shallow copy of the syntax tree. This is very fast. You need to
    /// copy a syntax tree in order to use it on more than one thread at a time,
    /// as syntax trees are not thread safe.
    fn ts_tree_copy(self_: NonNull<SyntaxTreeData>) -> NonNull<SyntaxTreeData>;
    /// Delete the syntax tree, freeing all of the memory that it used.
    fn ts_tree_delete(self_: NonNull<SyntaxTreeData>);
    /// Get the root node of the syntax tree.
    fn ts_tree_root_node<'tree>(self_: NonNull<SyntaxTreeData>) -> SyntaxTreeNodeRaw;
    /// Edit the syntax tree to keep it in sync with source code that has been
    /// edited.
    ///
    /// You must describe the edit both in terms of byte offsets and in terms of
    /// row/column coordinates.
    fn ts_tree_edit(self_: NonNull<SyntaxTreeData>, edit: &InputEdit);
}

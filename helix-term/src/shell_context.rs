use std::collections::HashMap;

use helix_core::coords_at_pos;

use helix_view::Editor;

use std::path::PathBuf;

#[derive(Debug)]
/// ShellContext contains editor metadata passed to shell commands in the form
/// of environment variables.
pub struct ShellContext {
    path_absolute: Option<PathBuf>,
    /// The *one-indexed* line number on which the cursor currently sits.
    cursor_line: usize,
    /// The *one-indexed* column number on which the cursor currently sits.
    cursor_col: usize,
    selection_primary_byte_start: usize,
    selection_primary_byte_end: usize,
    /// The *one-indexed* line number on which the primary selection starts.
    selection_primary_line_start: usize,
    /// The *one-indexed* line number on which the primary selection ends.
    selection_primary_line_end: usize,
}
impl ShellContext {
    pub fn for_editor(editor: &Editor) -> Self {
        let (view, doc) = current_ref!(editor);
        let doc_slice = doc.text().slice(..);
        let primary_sel = doc.selection(view.id).primary();

        let selection_primary_byte_range = primary_sel.into_byte_range(doc_slice);
        let selection_primary_line_range = primary_sel.line_range(doc_slice);
        let cursor_pos = coords_at_pos(doc_slice, primary_sel.cursor(doc_slice));

        Self {
            path_absolute: doc.path().cloned(),
            selection_primary_byte_start: selection_primary_byte_range.0,
            selection_primary_byte_end: selection_primary_byte_range.1,
            selection_primary_line_start: selection_primary_line_range.0 + 1,
            selection_primary_line_end: selection_primary_line_range.1 + 1,
            cursor_line: cursor_pos.row + 1,
            cursor_col: cursor_pos.col + 1,
        }
    }

    pub fn to_envs(self: &Self) -> HashMap<&str, String> {
        let mut env = HashMap::new();

        if let Some(path) = &self.path_absolute {
            env.insert(
                "HELIX_DOC_PATH_ABS",
                path.to_str().unwrap_or_else(|| "").to_string(),
            );
        }
        env.insert(
            "HELIX_SEL_BYTE_START",
            format!("{}", self.selection_primary_byte_start).to_string(),
        );
        env.insert(
            "HELIX_SEL_BYTE_END",
            format!("{}", self.selection_primary_byte_end).to_string(),
        );
        env.insert(
            "HELIX_SEL_LINE_START",
            format!("{}", self.selection_primary_line_start).to_string(),
        );
        env.insert(
            "HELIX_SEL_LINE_END",
            format!("{}", self.selection_primary_line_end).to_string(),
        );
        env.insert(
            "HELIX_CURSOR_LINE",
            format!("{}", self.cursor_line).to_string(),
        );
        env.insert(
            "HELIX_CURSOR_COL",
            format!("{}", self.cursor_col).to_string(),
        );
        env
    }
}

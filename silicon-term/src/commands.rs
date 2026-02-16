pub(crate) mod dap;
pub(crate) mod lsp;
pub(crate) mod syntax;
pub(crate) mod typed;
pub(crate) mod insert;
pub(crate) mod editing;
pub(crate) mod movement;
pub(crate) mod selection;

pub use dap::*;
use futures_util::FutureExt;
use silicon_event::status;
use silicon_stdx::{
    path::{self, find_paths},
    rope::{self, RopeSliceExt},
};
use silicon_vcs::{FileChange, Hunk};
pub use lsp::*;
pub use syntax::*;
use tui::{
    text::{Span, Spans},
    widgets::Cell,
};
pub use typed::*;
use editing::*;
use movement::*;
use selection::*;

// Re-export items used by other modules in the crate
pub(crate) use editing::paste_bracketed_value;
pub use editing::Open;
pub use movement::scroll;

use silicon_core::{
    char_idx_at_visual_offset,
    chars::char_is_word,
    command_line::{self, Args},
    comment,
    doc_formatter::TextFormat,
    encoding, find_workspace,
    graphemes::{self, next_grapheme_boundary},
    history::UndoKind,
    increment as core_increment,
    indent::{self as core_indent, IndentStyle},
    line_ending::{get_line_ending_of_str, line_end_char_index},
    match_brackets as core_match_brackets,
    movement::{self as core_movement, move_vertically_visual, Direction},
    object, pos_at_coords,
    regex::{self, Regex},
    search::{self as core_search, CharMatcher},
    selection as core_selection, surround,
    syntax::config::{BlockCommentToken, LanguageServerFeature},
    text_annotations::{Overlay, TextAnnotations},
    textobject,
    unicode::width::UnicodeWidthChar,
    visual_offset_from_block, Deletion, LineEnding, Position, Range, Rope, RopeReader, RopeSlice,
    Selection, SmallVec, Syntax, Tendril, Transaction,
};
use silicon_view::{
    document::{FormatterError, Mode, SCRATCH_BUFFER_NAME},
    editor::Action,
    expansion,
    info::Info,
    input::KeyEvent,
    keyboard::KeyCode,
    theme::Style,
    tree,
    view::View,
    Document, DocumentId, Editor, ViewId,
};

use anyhow::{anyhow, bail, ensure, Context as _};
use arc_swap::access::DynAccess;
use insert::*;
use core_movement::Movement;

use crate::{
    compositor::{self, Component, Compositor},
    filter_picker_entry,
    job::Callback,
    ui::{self, overlay::overlaid, Picker, PickerColumn, Popup, Prompt, PromptEvent},
};

use crate::job::{self, Jobs};
use std::{
    char::{ToLowercase, ToUppercase},
    cmp::Ordering,
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    future::Future,
    io::Read,
    num::NonZeroUsize,
};

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use serde::de::{self, Deserialize, Deserializer};
use url::Url;

use grep_regex::RegexMatcherBuilder;
use grep_searcher::{sinks, BinaryDetection, SearcherBuilder};
use ignore::{DirEntry, WalkBuilder, WalkState};

pub type OnKeyCallback = Box<dyn FnOnce(&mut Context, KeyEvent)>;
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum OnKeyCallbackKind {
    PseudoPending,
    Fallback,
}

pub struct Context<'a> {
    pub register: Option<char>,
    pub count: Option<NonZeroUsize>,
    pub editor: &'a mut Editor,

    pub callback: Vec<crate::compositor::Callback>,
    pub on_next_key_callback: Option<(OnKeyCallback, OnKeyCallbackKind)>,
    pub jobs: &'a mut Jobs,
}

impl Context<'_> {
    /// Push a new component onto the compositor.
    pub fn push_layer(&mut self, component: Box<dyn Component>) {
        self.callback
            .push(Box::new(|compositor: &mut Compositor, _| {
                compositor.push(component)
            }));
    }

    /// Call `replace_or_push` on the Compositor
    pub fn replace_or_push_layer<T: Component>(&mut self, id: &'static str, component: T) {
        self.callback
            .push(Box::new(move |compositor: &mut Compositor, _| {
                compositor.replace_or_push(id, component);
            }));
    }

    #[inline]
    pub fn on_next_key(
        &mut self,
        on_next_key_callback: impl FnOnce(&mut Context, KeyEvent) + 'static,
    ) {
        self.on_next_key_callback = Some((
            Box::new(on_next_key_callback),
            OnKeyCallbackKind::PseudoPending,
        ));
    }

    #[inline]
    pub fn on_next_key_fallback(
        &mut self,
        on_next_key_callback: impl FnOnce(&mut Context, KeyEvent) + 'static,
    ) {
        self.on_next_key_callback =
            Some((Box::new(on_next_key_callback), OnKeyCallbackKind::Fallback));
    }

    #[inline]
    pub fn callback<T, F>(
        &mut self,
        call: impl Future<Output = silicon_lsp::Result<T>> + 'static + Send,
        callback: F,
    ) where
        T: Send + 'static,
        F: FnOnce(&mut Editor, &mut Compositor, T) + Send + 'static,
    {
        self.jobs.callback(make_job_callback(call, callback));
    }

    /// Returns 1 if no explicit count was provided
    #[inline]
    pub fn count(&self) -> usize {
        self.count.map_or(1, |v| v.get())
    }

    /// Waits on all pending jobs, and then tries to flush all pending write
    /// operations for all documents.
    pub fn block_try_flush_writes(&mut self) -> anyhow::Result<()> {
        compositor::Context {
            editor: self.editor,
            jobs: self.jobs,
            scroll: None,
        }
        .block_try_flush_writes()
    }
}

#[inline]
fn make_job_callback<T, F>(
    call: impl Future<Output = silicon_lsp::Result<T>> + 'static + Send,
    callback: F,
) -> std::pin::Pin<Box<impl Future<Output = Result<Callback, anyhow::Error>>>>
where
    T: Send + 'static,
    F: FnOnce(&mut Editor, &mut Compositor, T) + Send + 'static,
{
    Box::pin(async move {
        let response = call.await?;
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                callback(editor, compositor, response)
            },
        ));
        Ok(call)
    })
}

use silicon_view::{align_view, Align};

/// MappableCommands are commands that can be bound to keys, executable in
/// normal, insert or select mode.
///
/// There are three kinds:
///
/// * Static: commands usually bound to keys and used for editing, movement,
///   etc., for example `move_char_left`.
/// * Typable: commands executable from command mode, prefixed with a `:`,
///   for example `:write!`.
/// * Macro: a sequence of keys to execute, for example `@miw`.
#[derive(Clone)]
pub enum MappableCommand {
    Typable {
        name: String,
        args: String,
        doc: String,
    },
    Static {
        name: &'static str,
        fun: fn(cx: &mut Context),
        doc: &'static str,
    },
    Macro {
        name: String,
        keys: Vec<KeyEvent>,
    },
}

macro_rules! static_commands {
    ( $($name:ident, $doc:literal,)* ) => {
        $(
            #[allow(non_upper_case_globals)]
            pub const $name: Self = Self::Static {
                name: stringify!($name),
                fun: $name,
                doc: $doc
            };
        )*

        pub const STATIC_COMMAND_LIST: &'static [Self] = &[
            $( Self::$name, )*
        ];
    }
}

impl MappableCommand {
    pub fn execute(&self, cx: &mut Context) {
        match &self {
            Self::Typable { name, args, doc: _ } => {
                if let Some(command) = typed::TYPABLE_COMMAND_MAP.get(name.as_str()) {
                    let mut cx = compositor::Context {
                        editor: cx.editor,
                        jobs: cx.jobs,
                        scroll: None,
                    };
                    if let Err(e) =
                        typed::execute_command(&mut cx, command, args, PromptEvent::Validate)
                    {
                        cx.editor.set_error(format!("{}", e));
                    }
                } else {
                    cx.editor.set_error(format!("no such command: '{name}'"));
                }
            }
            Self::Static { fun, .. } => (fun)(cx),
            Self::Macro { keys, .. } => {
                // Protect against recursive macros.
                if cx.editor.macro_replaying.contains(&'@') {
                    cx.editor.set_error(
                        "Cannot execute macro because the [@] register is already playing a macro",
                    );
                    return;
                }
                cx.editor.macro_replaying.push('@');
                let keys = keys.clone();
                cx.callback.push(Box::new(move |compositor, cx| {
                    for key in keys.into_iter() {
                        compositor.handle_event(&compositor::Event::Key(key), cx);
                    }
                    cx.editor.macro_replaying.pop();
                }));
            }
        }
    }

    pub fn name(&self) -> &str {
        match &self {
            Self::Typable { name, .. } => name,
            Self::Static { name, .. } => name,
            Self::Macro { name, .. } => name,
        }
    }

    pub fn doc(&self) -> &str {
        match &self {
            Self::Typable { doc, .. } => doc,
            Self::Static { doc, .. } => doc,
            Self::Macro { name, .. } => name,
        }
    }

    #[rustfmt::skip]
    static_commands!(
        no_op, "Do nothing",
        move_char_left, "Move left",
        move_char_right, "Move right",
        move_line_up, "Move up",
        move_line_down, "Move down",
        move_visual_line_up, "Move up",
        move_visual_line_down, "Move down",
        extend_char_left, "Extend left",
        extend_char_right, "Extend right",
        extend_line_up, "Extend up",
        extend_line_down, "Extend down",
        extend_visual_line_up, "Extend up",
        extend_visual_line_down, "Extend down",
        copy_selection_on_next_line, "Copy selection on next line",
        copy_selection_on_prev_line, "Copy selection on previous line",
        move_next_word_start, "Move to start of next word",
        move_prev_word_start, "Move to start of previous word",
        move_next_word_end, "Move to end of next word",
        move_prev_word_end, "Move to end of previous word",
        move_next_long_word_start, "Move to start of next long word",
        move_prev_long_word_start, "Move to start of previous long word",
        move_next_long_word_end, "Move to end of next long word",
        move_prev_long_word_end, "Move to end of previous long word",
        move_next_sub_word_start, "Move to start of next sub word",
        move_prev_sub_word_start, "Move to start of previous sub word",
        move_next_sub_word_end, "Move to end of next sub word",
        move_prev_sub_word_end, "Move to end of previous sub word",
        move_parent_node_end, "Move to end of the parent node",
        move_parent_node_start, "Move to beginning of the parent node",
        extend_next_word_start, "Extend to start of next word",
        extend_prev_word_start, "Extend to start of previous word",
        extend_next_word_end, "Extend to end of next word",
        extend_prev_word_end, "Extend to end of previous word",
        extend_next_long_word_start, "Extend to start of next long word",
        extend_prev_long_word_start, "Extend to start of previous long word",
        extend_next_long_word_end, "Extend to end of next long word",
        extend_prev_long_word_end, "Extend to end of prev long word",
        extend_next_sub_word_start, "Extend to start of next sub word",
        extend_prev_sub_word_start, "Extend to start of previous sub word",
        extend_next_sub_word_end, "Extend to end of next sub word",
        extend_prev_sub_word_end, "Extend to end of prev sub word",
        extend_parent_node_end, "Extend to end of the parent node",
        extend_parent_node_start, "Extend to beginning of the parent node",
        find_till_char, "Move till next occurrence of char",
        find_next_char, "Move to next occurrence of char",
        extend_till_char, "Extend till next occurrence of char",
        extend_next_char, "Extend to next occurrence of char",
        till_prev_char, "Move till previous occurrence of char",
        find_prev_char, "Move to previous occurrence of char",
        extend_till_prev_char, "Extend till previous occurrence of char",
        extend_prev_char, "Extend to previous occurrence of char",
        repeat_last_motion, "Repeat last motion",
        replace, "Replace with new char",
        switch_case, "Switch (toggle) case",
        switch_to_uppercase, "Switch to uppercase",
        switch_to_lowercase, "Switch to lowercase",
        page_up, "Move page up",
        page_down, "Move page down",
        half_page_up, "Move half page up",
        half_page_down, "Move half page down",
        page_cursor_up, "Move page and cursor up",
        page_cursor_down, "Move page and cursor down",
        page_cursor_half_up, "Move page and cursor half up",
        page_cursor_half_down, "Move page and cursor half down",
        select_all, "Select whole document",
        select_regex, "Select all regex matches inside selections",
        split_selection, "Split selections on regex matches",
        split_selection_on_newline, "Split selection on newlines",
        merge_selections, "Merge selections",
        merge_consecutive_selections, "Merge consecutive selections",
        search, "Search for regex pattern",
        rsearch, "Reverse search for regex pattern",
        search_next, "Select next search match",
        search_prev, "Select previous search match",
        extend_search_next, "Add next search match to selection",
        extend_search_prev, "Add previous search match to selection",
        search_selection, "Use current selection as search pattern",
        search_selection_detect_word_boundaries, "Use current selection as the search pattern, automatically wrapping with `\\b` on word boundaries",
        make_search_word_bounded, "Modify current search to make it word bounded",
        global_search, "Global search in workspace folder",
        extend_line, "Select current line, if already selected, extend to another line based on the anchor",
        extend_line_below, "Select current line, if already selected, extend to next line",
        extend_line_above, "Select current line, if already selected, extend to previous line",
        select_line_above, "Select current line, if already selected, extend or shrink line above based on the anchor",
        select_line_below, "Select current line, if already selected, extend or shrink line below based on the anchor",
        extend_to_line_bounds, "Extend selection to line bounds",
        shrink_to_line_bounds, "Shrink selection to line bounds",
        delete_selection, "Delete selection",
        delete_selection_noyank, "Delete selection without yanking",
        change_selection, "Change selection",
        change_selection_noyank, "Change selection without yanking",
        collapse_selection, "Collapse selection into single cursor",
        flip_selections, "Flip selection cursor and anchor",
        ensure_selections_forward, "Ensure all selections face forward",
        insert_mode, "Insert before selection",
        append_mode, "Append after selection",
        command_mode, "Enter command mode",
        file_picker, "Open file picker",
        file_picker_in_current_buffer_directory, "Open file picker at current buffer's directory",
        file_picker_in_current_directory, "Open file picker at current working directory",
        file_explorer, "Open file explorer in workspace root",
        file_explorer_in_current_buffer_directory, "Open file explorer at current buffer's directory",
        file_explorer_in_current_directory, "Open file explorer at current working directory",
        code_action, "Perform code action",
        buffer_picker, "Open buffer picker",
        jumplist_picker, "Open jumplist picker",
        symbol_picker, "Open symbol picker",
        syntax_symbol_picker, "Open symbol picker from syntax information",
        lsp_or_syntax_symbol_picker, "Open symbol picker from LSP or syntax information",
        changed_file_picker, "Open changed file picker",
        select_references_to_symbol_under_cursor, "Select symbol references",
        workspace_symbol_picker, "Open workspace symbol picker",
        syntax_workspace_symbol_picker, "Open workspace symbol picker from syntax information",
        lsp_or_syntax_workspace_symbol_picker, "Open workspace symbol picker from LSP or syntax information",
        diagnostics_picker, "Open diagnostic picker",
        workspace_diagnostics_picker, "Open workspace diagnostic picker",
        last_picker, "Open last picker",
        insert_at_line_start, "Insert at start of line",
        insert_at_line_end, "Insert at end of line",
        open_below, "Open new line below selection",
        open_above, "Open new line above selection",
        normal_mode, "Enter normal mode",
        select_mode, "Enter selection extend mode",
        exit_select_mode, "Exit selection mode",
        goto_definition, "Goto definition",
        goto_declaration, "Goto declaration",
        add_newline_above, "Add newline above",
        add_newline_below, "Add newline below",
        goto_type_definition, "Goto type definition",
        goto_implementation, "Goto implementation",
        goto_file_start, "Goto line number <n> else file start",
        goto_file_end, "Goto file end",
        extend_to_file_start, "Extend to line number<n> else file start",
        extend_to_file_end, "Extend to file end",
        goto_file, "Goto files/URLs in selections",
        goto_file_hsplit, "Goto files in selections (hsplit)",
        goto_file_vsplit, "Goto files in selections (vsplit)",
        goto_reference, "Goto references",
        goto_window_top, "Goto window top",
        goto_window_center, "Goto window center",
        goto_window_bottom, "Goto window bottom",
        goto_last_accessed_file, "Goto last accessed file",
        goto_last_modified_file, "Goto last modified file",
        goto_last_modification, "Goto last modification",
        goto_line, "Goto line",
        goto_last_line, "Goto last line",
        extend_to_last_line, "Extend to last line",
        goto_first_diag, "Goto first diagnostic",
        goto_last_diag, "Goto last diagnostic",
        goto_next_diag, "Goto next diagnostic",
        goto_prev_diag, "Goto previous diagnostic",
        goto_next_change, "Goto next change",
        goto_prev_change, "Goto previous change",
        goto_first_change, "Goto first change",
        goto_last_change, "Goto last change",
        goto_line_start, "Goto line start",
        goto_line_end, "Goto line end",
        goto_column, "Goto column",
        extend_to_column, "Extend to column",
        goto_next_buffer, "Goto next buffer",
        goto_previous_buffer, "Goto previous buffer",
        goto_line_end_newline, "Goto newline at line end",
        goto_first_nonwhitespace, "Goto first non-blank in line",
        trim_selections, "Trim whitespace from selections",
        extend_to_line_start, "Extend to line start",
        extend_to_first_nonwhitespace, "Extend to first non-blank in line",
        extend_to_line_end, "Extend to line end",
        extend_to_line_end_newline, "Extend to line end",
        signature_help, "Show signature help",
        smart_tab, "Insert tab if all cursors have all whitespace to their left; otherwise, run a separate command.",
        insert_tab, "Insert tab char",
        insert_newline, "Insert newline char",
        insert_char_interactive, "Insert an interactively-chosen char",
        append_char_interactive, "Append an interactively-chosen char",
        delete_char_backward, "Delete previous char",
        delete_char_forward, "Delete next char",
        delete_word_backward, "Delete previous word",
        delete_word_forward, "Delete next word",
        kill_to_line_start, "Delete till start of line",
        kill_to_line_end, "Delete till end of line",
        undo, "Undo change",
        redo, "Redo change",
        earlier, "Move backward in history",
        later, "Move forward in history",
        commit_undo_checkpoint, "Commit changes to new checkpoint",
        yank, "Yank selection",
        yank_to_clipboard, "Yank selections to clipboard",
        yank_to_primary_clipboard, "Yank selections to primary clipboard",
        yank_joined, "Join and yank selections",
        yank_joined_to_clipboard, "Join and yank selections to clipboard",
        yank_main_selection_to_clipboard, "Yank main selection to clipboard",
        yank_joined_to_primary_clipboard, "Join and yank selections to primary clipboard",
        yank_main_selection_to_primary_clipboard, "Yank main selection to primary clipboard",
        replace_with_yanked, "Replace with yanked text",
        replace_selections_with_clipboard, "Replace selections by clipboard content",
        replace_selections_with_primary_clipboard, "Replace selections by primary clipboard",
        paste_after, "Paste after selection",
        paste_before, "Paste before selection",
        paste_clipboard_after, "Paste clipboard after selections",
        paste_clipboard_before, "Paste clipboard before selections",
        paste_primary_clipboard_after, "Paste primary clipboard after selections",
        paste_primary_clipboard_before, "Paste primary clipboard before selections",
        indent, "Indent selection",
        unindent, "Unindent selection",
        format_selections, "Format selection",
        join_selections, "Join lines inside selection",
        join_selections_space, "Join lines inside selection and select spaces",
        keep_selections, "Keep selections matching regex",
        remove_selections, "Remove selections matching regex",
        align_selections, "Align selections in column",
        keep_primary_selection, "Keep primary selection",
        remove_primary_selection, "Remove primary selection",
        completion, "Invoke completion popup",
        hover, "Show docs for item under cursor",
        toggle_comments, "Comment/uncomment selections",
        toggle_line_comments, "Line comment/uncomment selections",
        toggle_block_comments, "Block comment/uncomment selections",
        rotate_selections_forward, "Rotate selections forward",
        rotate_selections_backward, "Rotate selections backward",
        rotate_selection_contents_forward, "Rotate selection contents forward",
        rotate_selection_contents_backward, "Rotate selections contents backward",
        reverse_selection_contents, "Reverse selections contents",
        expand_selection, "Expand selection to parent syntax node",
        shrink_selection, "Shrink selection to previously expanded syntax node",
        select_next_sibling, "Select next sibling in the syntax tree",
        select_prev_sibling, "Select previous sibling the in syntax tree",
        select_all_siblings, "Select all siblings of the current node",
        select_all_children, "Select all children of the current node",
        jump_forward, "Jump forward on jumplist",
        jump_backward, "Jump backward on jumplist",
        save_selection, "Save current selection to jumplist",
        jump_view_right, "Jump to right split",
        jump_view_left, "Jump to left split",
        jump_view_up, "Jump to split above",
        jump_view_down, "Jump to split below",
        swap_view_right, "Swap with right split",
        swap_view_left, "Swap with left split",
        swap_view_up, "Swap with split above",
        swap_view_down, "Swap with split below",
        transpose_view, "Transpose splits",
        rotate_view, "Goto next window",
        rotate_view_reverse, "Goto previous window",
        hsplit, "Horizontal bottom split",
        hsplit_new, "Horizontal bottom split scratch buffer",
        vsplit, "Vertical right split",
        vsplit_new, "Vertical right split scratch buffer",
        wclose, "Close window",
        wonly, "Close windows except current",
        select_register, "Select register",
        insert_register, "Insert register",
        copy_between_registers, "Copy between two registers",
        align_view_middle, "Align view middle",
        align_view_top, "Align view top",
        align_view_center, "Align view center",
        align_view_bottom, "Align view bottom",
        scroll_up, "Scroll view up",
        scroll_down, "Scroll view down",
        match_brackets, "Goto matching bracket",
        surround_add, "Surround add",
        surround_replace, "Surround replace",
        surround_delete, "Surround delete",
        select_textobject_around, "Select around object",
        select_textobject_inner, "Select inside object",
        goto_next_function, "Goto next function",
        goto_prev_function, "Goto previous function",
        goto_next_class, "Goto next type definition",
        goto_prev_class, "Goto previous type definition",
        goto_next_parameter, "Goto next parameter",
        goto_prev_parameter, "Goto previous parameter",
        goto_next_comment, "Goto next comment",
        goto_prev_comment, "Goto previous comment",
        goto_next_test, "Goto next test",
        goto_prev_test, "Goto previous test",
        goto_next_xml_element, "Goto next (X)HTML element",
        goto_prev_xml_element, "Goto previous (X)HTML element",
        goto_next_entry, "Goto next pairing",
        goto_prev_entry, "Goto previous pairing",
        goto_next_paragraph, "Goto next paragraph",
        goto_prev_paragraph, "Goto previous paragraph",
        dap_launch, "Launch debug target",
        dap_restart, "Restart debugging session",
        dap_toggle_breakpoint, "Toggle breakpoint",
        dap_continue, "Continue program execution",
        dap_pause, "Pause program execution",
        dap_step_in, "Step in",
        dap_step_out, "Step out",
        dap_next, "Step to next",
        dap_variables, "List variables",
        dap_terminate, "End debug session",
        dap_edit_condition, "Edit breakpoint condition on current line",
        dap_edit_log, "Edit breakpoint log message on current line",
        dap_switch_thread, "Switch current thread",
        dap_switch_stack_frame, "Switch stack frame",
        dap_enable_exceptions, "Enable exception breakpoints",
        dap_disable_exceptions, "Disable exception breakpoints",
        shell_pipe, "Pipe selections through shell command",
        shell_pipe_to, "Pipe selections into shell command ignoring output",
        shell_insert_output, "Insert shell command output before selections",
        shell_append_output, "Append shell command output after selections",
        shell_keep_pipe, "Filter selections with shell predicate",
        suspend, "Suspend and return to shell",
        rename_symbol, "Rename symbol",
        increment, "Increment item under cursor",
        decrement, "Decrement item under cursor",
        record_macro, "Record macro",
        replay_macro, "Replay macro",
        command_palette, "Open command palette",
        goto_word, "Jump to a two-character label",
        extend_to_word, "Extend to a two-character label",
        goto_next_tabstop, "Goto next snippet placeholder",
        goto_prev_tabstop, "Goto next snippet placeholder",
        rotate_selections_first, "Make the first selection your primary one",
        rotate_selections_last, "Make the last selection your primary one",
        toggle_terminal_panel, "Toggle terminal panel",
        new_terminal_tab, "Open new terminal tab",
        run_file, "Run current file",
    );
}

impl fmt::Debug for MappableCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MappableCommand::Static { name, .. } => {
                f.debug_tuple("MappableCommand").field(name).finish()
            }
            MappableCommand::Typable { name, args, .. } => f
                .debug_tuple("MappableCommand")
                .field(name)
                .field(args)
                .finish(),
            MappableCommand::Macro { name, keys, .. } => f
                .debug_tuple("MappableCommand")
                .field(name)
                .field(keys)
                .finish(),
        }
    }
}

impl fmt::Display for MappableCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for MappableCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(suffix) = s.strip_prefix(':') {
            let (name, args, _) = command_line::split(suffix);
            ensure!(!name.is_empty(), "Expected typable command name");
            typed::TYPABLE_COMMAND_MAP
                .get(name)
                .map(|cmd| {
                    let doc = if args.is_empty() {
                        cmd.doc.to_string()
                    } else {
                        format!(":{} {:?}", cmd.name, args)
                    };
                    MappableCommand::Typable {
                        name: cmd.name.to_owned(),
                        doc,
                        args: args.to_string(),
                    }
                })
                .ok_or_else(|| anyhow!("No TypableCommand named '{}'", s))
        } else if let Some(suffix) = s.strip_prefix('@') {
            silicon_view::input::parse_macro(suffix).map(|keys| Self::Macro {
                name: s.to_string(),
                keys,
            })
        } else {
            MappableCommand::STATIC_COMMAND_LIST
                .iter()
                .find(|cmd| cmd.name() == s)
                .cloned()
                .ok_or_else(|| anyhow!("No command named '{}'", s))
        }
    }
}

impl<'de> Deserialize<'de> for MappableCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

impl PartialEq for MappableCommand {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                MappableCommand::Typable {
                    name: first_name,
                    args: first_args,
                    ..
                },
                MappableCommand::Typable {
                    name: second_name,
                    args: second_args,
                    ..
                },
            ) => first_name == second_name && first_args == second_args,
            (
                MappableCommand::Static {
                    name: first_name, ..
                },
                MappableCommand::Static {
                    name: second_name, ..
                },
            ) => first_name == second_name,
            _ => false,
        }
    }
}


fn file_picker(cx: &mut Context) {
    let root = find_workspace().0;
    if !root.exists() {
        cx.editor.set_error("Workspace directory does not exist");
        return;
    }
    let picker = ui::file_picker(cx.editor, root);
    cx.push_layer(Box::new(overlaid(picker)));
}

fn file_picker_in_current_buffer_directory(cx: &mut Context) {
    let doc_dir = doc!(cx.editor)
        .path()
        .and_then(|path| path.parent().map(|path| path.to_path_buf()));

    let path = match doc_dir {
        Some(path) => path,
        None => {
            let cwd = silicon_stdx::env::current_working_dir();
            if !cwd.exists() {
                cx.editor.set_error(
                    "Current buffer has no parent and current working directory does not exist",
                );
                return;
            }
            cx.editor.set_error(
                "Current buffer has no parent, opening file picker in current working directory",
            );
            cwd
        }
    };

    let picker = ui::file_picker(cx.editor, path);
    cx.push_layer(Box::new(overlaid(picker)));
}

fn file_picker_in_current_directory(cx: &mut Context) {
    let cwd = silicon_stdx::env::current_working_dir();
    if !cwd.exists() {
        cx.editor
            .set_error("Current working directory does not exist");
        return;
    }
    let picker = ui::file_picker(cx.editor, cwd);
    cx.push_layer(Box::new(overlaid(picker)));
}

fn file_explorer(cx: &mut Context) {
    let root = find_workspace().0;
    if !root.exists() {
        cx.editor.set_error("Workspace directory does not exist");
        return;
    }

    if let Ok(picker) = ui::file_explorer(root, cx.editor) {
        cx.push_layer(Box::new(overlaid(picker)));
    }
}

fn file_explorer_in_current_buffer_directory(cx: &mut Context) {
    let doc_dir = doc!(cx.editor)
        .path()
        .and_then(|path| path.parent().map(|path| path.to_path_buf()));

    let path = match doc_dir {
        Some(path) => path,
        None => {
            let cwd = silicon_stdx::env::current_working_dir();
            if !cwd.exists() {
                cx.editor.set_error(
                    "Current buffer has no parent and current working directory does not exist",
                );
                return;
            }
            cx.editor.set_error(
                "Current buffer has no parent, opening file explorer in current working directory",
            );
            cwd
        }
    };

    if let Ok(picker) = ui::file_explorer(path, cx.editor) {
        cx.push_layer(Box::new(overlaid(picker)));
    }
}

fn file_explorer_in_current_directory(cx: &mut Context) {
    let cwd = silicon_stdx::env::current_working_dir();
    if !cwd.exists() {
        cx.editor
            .set_error("Current working directory does not exist");
        return;
    }

    if let Ok(picker) = ui::file_explorer(cwd, cx.editor) {
        cx.push_layer(Box::new(overlaid(picker)));
    }
}

fn buffer_picker(cx: &mut Context) {
    let current = view!(cx.editor).doc;

    struct BufferMeta {
        id: DocumentId,
        path: Option<PathBuf>,
        is_modified: bool,
        is_current: bool,
        focused_at: std::time::Instant,
    }

    let new_meta = |doc: &Document| BufferMeta {
        id: doc.id(),
        path: doc.path().cloned(),
        is_modified: doc.is_modified(),
        is_current: doc.id() == current,
        focused_at: doc.focused_at,
    };

    let mut items = cx
        .editor
        .documents
        .values()
        .map(new_meta)
        .collect::<Vec<BufferMeta>>();

    // mru
    items.sort_unstable_by_key(|item| std::cmp::Reverse(item.focused_at));

    let columns = [
        PickerColumn::new("id", |meta: &BufferMeta, _| meta.id.to_string().into()),
        PickerColumn::new("flags", |meta: &BufferMeta, _| {
            let mut flags = String::new();
            if meta.is_modified {
                flags.push('+');
            }
            if meta.is_current {
                flags.push('*');
            }
            flags.into()
        }),
        PickerColumn::new("path", |meta: &BufferMeta, _| {
            let path = meta
                .path
                .as_deref()
                .map(silicon_stdx::path::get_relative_path);
            path.as_deref()
                .and_then(Path::to_str)
                .unwrap_or(SCRATCH_BUFFER_NAME)
                .to_string()
                .into()
        }),
    ];

    let initial_cursor = if cx
        .editor
        .config()
        .buffer_picker
        .start_position
        .is_previous()
        && !items.is_empty()
    {
        1
    } else {
        0
    };

    let picker = Picker::new(columns, 2, items, (), |cx, meta, action| {
        cx.editor.switch(meta.id, action);
    })
    .with_initial_cursor(initial_cursor)
    .with_preview(|editor, meta| {
        let doc = &editor.documents.get(&meta.id)?;
        let lines = doc.selections().values().next().map(|selection| {
            let cursor_line = selection.primary().cursor_line(doc.text().slice(..));
            (cursor_line, cursor_line)
        });
        Some((meta.id.into(), lines))
    });
    cx.push_layer(Box::new(overlaid(picker)));
}

fn jumplist_picker(cx: &mut Context) {
    struct JumpMeta {
        id: DocumentId,
        path: Option<PathBuf>,
        selection: Selection,
        text: String,
        is_current: bool,
    }

    for (view, _) in cx.editor.tree.views_mut() {
        for doc_id in view.jumps.iter().map(|e| e.0).collect::<Vec<_>>().iter() {
            let doc = doc_mut!(cx.editor, doc_id);
            view.sync_changes(doc);
        }
    }

    let new_meta = |view: &View, doc_id: DocumentId, selection: Selection| {
        let doc = &cx.editor.documents.get(&doc_id);
        let text = doc.map_or("".into(), |d| {
            selection
                .fragments(d.text().slice(..))
                .map(Cow::into_owned)
                .collect::<Vec<_>>()
                .join(" ")
        });

        JumpMeta {
            id: doc_id,
            path: doc.and_then(|d| d.path().cloned()),
            selection,
            text,
            is_current: view.doc == doc_id,
        }
    };

    let columns = [
        ui::PickerColumn::new("id", |item: &JumpMeta, _| item.id.to_string().into()),
        ui::PickerColumn::new("path", |item: &JumpMeta, _| {
            let path = item
                .path
                .as_deref()
                .map(silicon_stdx::path::get_relative_path);
            path.as_deref()
                .and_then(Path::to_str)
                .unwrap_or(SCRATCH_BUFFER_NAME)
                .to_string()
                .into()
        }),
        ui::PickerColumn::new("flags", |item: &JumpMeta, _| {
            let mut flags = Vec::new();
            if item.is_current {
                flags.push("*");
            }

            if flags.is_empty() {
                "".into()
            } else {
                format!(" ({})", flags.join("")).into()
            }
        }),
        ui::PickerColumn::new("contents", |item: &JumpMeta, _| item.text.as_str().into()),
    ];

    let picker = Picker::new(
        columns,
        1, // path
        cx.editor.tree.views().flat_map(|(view, _)| {
            view.jumps
                .iter()
                .rev()
                .map(|(doc_id, selection)| new_meta(view, *doc_id, selection.clone()))
        }),
        (),
        |cx, meta, action| {
            cx.editor.switch(meta.id, action);
            let config = cx.editor.config();
            let (view, doc) = (view_mut!(cx.editor), doc_mut!(cx.editor, &meta.id));
            doc.set_selection(view.id, meta.selection.clone());
            if action.align_view(view, doc.id()) {
                view.ensure_cursor_in_view_center(doc, config.scrolloff);
            }
        },
    )
    .with_preview(|editor, meta| {
        let doc = &editor.documents.get(&meta.id)?;
        let line = meta.selection.primary().cursor_line(doc.text().slice(..));
        Some((meta.id.into(), Some((line, line))))
    });
    cx.push_layer(Box::new(overlaid(picker)));
}

fn changed_file_picker(cx: &mut Context) {
    pub struct FileChangeData {
        cwd: PathBuf,
        style_untracked: Style,
        style_modified: Style,
        style_conflict: Style,
        style_deleted: Style,
        style_renamed: Style,
    }

    let cwd = silicon_stdx::env::current_working_dir();
    if !cwd.exists() {
        cx.editor
            .set_error("Current working directory does not exist");
        return;
    }

    let added = cx.editor.theme.get("diff.plus");
    let modified = cx.editor.theme.get("diff.delta");
    let conflict = cx.editor.theme.get("diff.delta.conflict");
    let deleted = cx.editor.theme.get("diff.minus");
    let renamed = cx.editor.theme.get("diff.delta.moved");

    let columns = [
        PickerColumn::new("change", |change: &FileChange, data: &FileChangeData| {
            match change {
                FileChange::Untracked { .. } => Span::styled("+ untracked", data.style_untracked),
                FileChange::Modified { .. } => Span::styled("~ modified", data.style_modified),
                FileChange::Conflict { .. } => Span::styled("x conflict", data.style_conflict),
                FileChange::Deleted { .. } => Span::styled("- deleted", data.style_deleted),
                FileChange::Renamed { .. } => Span::styled("> renamed", data.style_renamed),
            }
            .into()
        }),
        PickerColumn::new("path", |change: &FileChange, data: &FileChangeData| {
            let display_path = |path: &PathBuf| {
                path.strip_prefix(&data.cwd)
                    .unwrap_or(path)
                    .display()
                    .to_string()
            };
            match change {
                FileChange::Untracked { path } => display_path(path),
                FileChange::Modified { path } => display_path(path),
                FileChange::Conflict { path } => display_path(path),
                FileChange::Deleted { path } => display_path(path),
                FileChange::Renamed { from_path, to_path } => {
                    format!("{} -> {}", display_path(from_path), display_path(to_path))
                }
            }
            .into()
        }),
    ];

    let picker = Picker::new(
        columns,
        1, // path
        [],
        FileChangeData {
            cwd: cwd.clone(),
            style_untracked: added,
            style_modified: modified,
            style_conflict: conflict,
            style_deleted: deleted,
            style_renamed: renamed,
        },
        |cx, meta: &FileChange, action| {
            let path_to_open = meta.path();
            if let Err(e) = cx.editor.open(path_to_open, action) {
                let err = if let Some(err) = e.source() {
                    format!("{}", err)
                } else {
                    format!("unable to open \"{}\"", path_to_open.display())
                };
                cx.editor.set_error(err);
            }
        },
    )
    .with_preview(|_editor, meta| Some((meta.path().into(), None)));
    let injector = picker.injector();

    cx.editor
        .diff_providers
        .clone()
        .for_each_changed_file(cwd, move |change| match change {
            Ok(change) => injector.push(change).is_ok(),
            Err(err) => {
                status::report_blocking(err);
                true
            }
        });
    cx.push_layer(Box::new(overlaid(picker)));
}

pub fn command_palette(cx: &mut Context) {
    let register = cx.register;
    let count = cx.count;

    cx.callback.push(Box::new(
        move |compositor: &mut Compositor, cx: &mut compositor::Context| {
            let keymap = compositor.find::<ui::EditorView>().expect("EditorView must exist in compositor").keymaps.map()
                [&cx.editor.mode]
                .reverse_map();

            let commands = MappableCommand::STATIC_COMMAND_LIST.iter().cloned().chain(
                typed::TYPABLE_COMMAND_LIST
                    .iter()
                    .map(|cmd| MappableCommand::Typable {
                        name: cmd.name.to_owned(),
                        args: String::new(),
                        doc: cmd.doc.to_owned(),
                    }),
            );

            let columns = [
                ui::PickerColumn::new("name", |item, _| match item {
                    MappableCommand::Typable { name, .. } => format!(":{name}").into(),
                    MappableCommand::Static { name, .. } => (*name).into(),
                    MappableCommand::Macro { .. } => {
                        unreachable!("macros aren't included in the command palette")
                    }
                }),
                ui::PickerColumn::new(
                    "bindings",
                    |item: &MappableCommand, keymap: &crate::keymap::ReverseKeymap| {
                        keymap
                            .get(item.name())
                            .map(|bindings| {
                                bindings.iter().fold(String::new(), |mut acc, bind| {
                                    if !acc.is_empty() {
                                        acc.push(' ');
                                    }
                                    for key in bind {
                                        acc.push_str(&key.key_sequence_format());
                                    }
                                    acc
                                })
                            })
                            .unwrap_or_default()
                            .into()
                    },
                ),
                ui::PickerColumn::new("doc", |item: &MappableCommand, _| item.doc().into()),
            ];

            let picker = Picker::new(columns, 0, commands, keymap, move |cx, command, _action| {
                let mut ctx = Context {
                    register,
                    count,
                    editor: cx.editor,
                    callback: Vec::new(),
                    on_next_key_callback: None,
                    jobs: cx.jobs,
                };
                let focus = view!(ctx.editor).id;

                command.execute(&mut ctx);

                if ctx.editor.tree.contains(focus) {
                    let config = ctx.editor.config();
                    let mode = ctx.editor.mode();
                    let view = view_mut!(ctx.editor, focus);
                    let doc = doc_mut!(ctx.editor, &view.doc);

                    view.ensure_cursor_in_view(doc, config.scrolloff);

                    if mode != Mode::Insert {
                        doc.append_changes_to_history(view);
                    }
                }
            });
            compositor.push(Box::new(overlaid(picker)));
        },
    ));
}

fn last_picker(cx: &mut Context) {
    // TODO: last picker does not seem to work well with buffer_picker
    cx.callback.push(Box::new(|compositor, cx| {
        if let Some(picker) = compositor.last_picker.take() {
            compositor.push(picker);
        } else {
            cx.editor.set_error("no last picker")
        }
    }));
}

fn normal_mode(cx: &mut Context) {
    cx.editor.enter_normal_mode();
}

// Store a jump on the jumplist.
fn push_jump(view: &mut View, doc: &mut Document) {
    doc.append_changes_to_history(view);
    let jump = (doc.id(), doc.selection(view.id).clone());
    view.jumps.push(jump);
}


fn select_mode(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    // Make sure end-of-document selections are also 1-width.
    // (With the exception of being in an empty document, of course.)
    let selection = doc.selection(view.id).clone().transform(|range| {
        if range.is_empty() && range.head == text.len_chars() {
            Range::new(
                graphemes::prev_grapheme_boundary(text, range.anchor),
                range.head,
            )
        } else {
            range
        }
    });
    doc.set_selection(view.id, selection);

    cx.editor.mode = Mode::Select;
}

fn exit_select_mode(cx: &mut Context) {
    if cx.editor.mode == Mode::Select {
        cx.editor.mode = Mode::Normal;
    }
}


// Undo / Redo

fn undo(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    for _ in 0..count {
        if !doc.undo(view) {
            cx.editor.set_status("Already at oldest change");
            break;
        }
    }
}

fn redo(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    for _ in 0..count {
        if !doc.redo(view) {
            cx.editor.set_status("Already at newest change");
            break;
        }
    }
}

fn earlier(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    for _ in 0..count {
        // rather than doing in batch we do this so get error halfway
        if !doc.earlier(view, UndoKind::Steps(1)) {
            cx.editor.set_status("Already at oldest change");
            break;
        }
    }
}

fn later(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    for _ in 0..count {
        // rather than doing in batch we do this so get error halfway
        if !doc.later(view, UndoKind::Steps(1)) {
            cx.editor.set_status("Already at newest change");
            break;
        }
    }
}

fn commit_undo_checkpoint(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    doc.append_changes_to_history(view);
}


pub fn completion(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let range = doc.selection(view.id).primary();
    let text = doc.text().slice(..);
    let cursor = range.cursor(text);

    cx.editor
        .handlers
        .trigger_completions(cursor, doc.id(), view.id);
}

// comments


fn rotate_view(cx: &mut Context) {
    cx.editor.focus_next()
}

fn rotate_view_reverse(cx: &mut Context) {
    cx.editor.focus_prev()
}

fn jump_view_right(cx: &mut Context) {
    cx.editor.focus_direction(tree::Direction::Right)
}

fn jump_view_left(cx: &mut Context) {
    cx.editor.focus_direction(tree::Direction::Left)
}

fn jump_view_up(cx: &mut Context) {
    cx.editor.focus_direction(tree::Direction::Up)
}

fn jump_view_down(cx: &mut Context) {
    cx.editor.focus_direction(tree::Direction::Down)
}

fn swap_view_right(cx: &mut Context) {
    cx.editor.swap_split_in_direction(tree::Direction::Right)
}

fn swap_view_left(cx: &mut Context) {
    cx.editor.swap_split_in_direction(tree::Direction::Left)
}

fn swap_view_up(cx: &mut Context) {
    cx.editor.swap_split_in_direction(tree::Direction::Up)
}

fn swap_view_down(cx: &mut Context) {
    cx.editor.swap_split_in_direction(tree::Direction::Down)
}

fn transpose_view(cx: &mut Context) {
    cx.editor.transpose_view()
}

/// Open a new split in the given direction specified by the action.
///
/// Maintain the current view (both the cursor's position and view in document).
fn split(editor: &mut Editor, action: Action) {
    let (view, doc) = current!(editor);
    let id = doc.id();
    let selection = doc.selection(view.id).clone();
    let offset = doc.view_offset(view.id);

    editor.switch(id, action);

    // match the selection in the previous view
    let (view, doc) = current!(editor);
    doc.set_selection(view.id, selection);
    // match the view scroll offset (switch doesn't handle this fully
    // since the selection is only matched after the split)
    doc.set_view_offset(view.id, offset);
}

fn hsplit(cx: &mut Context) {
    split(cx.editor, Action::HorizontalSplit);
}

fn hsplit_new(cx: &mut Context) {
    cx.editor.new_file(Action::HorizontalSplit);
}

fn vsplit(cx: &mut Context) {
    split(cx.editor, Action::VerticalSplit);
}

fn vsplit_new(cx: &mut Context) {
    cx.editor.new_file(Action::VerticalSplit);
}

fn wclose(cx: &mut Context) {
    if cx.editor.tree.views().count() == 1 {
        if let Err(err) = typed::buffers_remaining_impl(cx.editor) {
            cx.editor.set_error(err.to_string());
            return;
        }
    }
    let view_id = view!(cx.editor).id;
    // close current split
    cx.editor.close(view_id);
}

fn wonly(cx: &mut Context) {
    let views = cx
        .editor
        .tree
        .views()
        .map(|(v, focus)| (v.id, focus))
        .collect::<Vec<_>>();
    for (view_id, focus) in views {
        if !focus {
            cx.editor.close(view_id);
        }
    }
}

fn select_register(cx: &mut Context) {
    cx.editor.autoinfo = Some(Info::from_registers(
        "Select register",
        &cx.editor.registers,
    ));
    cx.on_next_key(move |cx, event| {
        cx.editor.autoinfo = None;
        if let Some(ch) = event.char() {
            cx.editor.selected_register = Some(ch);
        }
    })
}

fn insert_register(cx: &mut Context) {
    cx.editor.autoinfo = Some(Info::from_registers(
        "Insert register",
        &cx.editor.registers,
    ));
    cx.on_next_key(move |cx, event| {
        cx.editor.autoinfo = None;
        if let Some(ch) = event.char() {
            cx.register = Some(ch);
            paste(
                cx.editor,
                cx.register
                    .unwrap_or(cx.editor.config().default_yank_register),
                Paste::Cursor,
                cx.count(),
            );
        }
    })
}

fn copy_between_registers(cx: &mut Context) {
    cx.editor.autoinfo = Some(Info::from_registers(
        "Copy from register",
        &cx.editor.registers,
    ));
    cx.on_next_key(move |cx, event| {
        cx.editor.autoinfo = None;

        let Some(source) = event.char() else {
            return;
        };

        let Some(values) = cx.editor.registers.read(source, cx.editor) else {
            cx.editor.set_error(format!("register {source} is empty"));
            return;
        };
        let values: Vec<_> = values.map(|value| value.to_string()).collect();

        cx.editor.autoinfo = Some(Info::from_registers(
            "Copy into register",
            &cx.editor.registers,
        ));
        cx.on_next_key(move |cx, event| {
            cx.editor.autoinfo = None;

            let Some(dest) = event.char() else {
                return;
            };

            let n_values = values.len();
            match cx.editor.registers.write(dest, values) {
                Ok(_) => cx.editor.set_status(format!(
                    "yanked {n_values} value{} from register {source} to {dest}",
                    if n_values == 1 { "" } else { "s" }
                )),
                Err(err) => cx.editor.set_error(err.to_string()),
            }
        });
    });
}

fn align_view_top(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    align_view(doc, view, Align::Top);
}

fn align_view_center(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    align_view(doc, view, Align::Center);
}

fn align_view_bottom(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    align_view(doc, view, Align::Bottom);
}

fn align_view_middle(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let inner_width = view.inner_width(doc);
    let text_fmt = doc.text_format(inner_width, None);
    // there is no horizontal position when softwrap is enabled
    if text_fmt.soft_wrap {
        return;
    }
    let doc_text = doc.text().slice(..);
    let pos = doc.selection(view.id).primary().cursor(doc_text);
    let pos = visual_offset_from_block(
        doc_text,
        doc.view_offset(view.id).anchor,
        pos,
        &text_fmt,
        &view.text_annotations(doc, None),
    )
    .0;

    let mut offset = doc.view_offset(view.id);
    offset.horizontal_offset = pos
        .col
        .saturating_sub((view.inner_area(doc).width as usize) / 2);
    doc.set_view_offset(view.id, offset);
}


fn record_macro(cx: &mut Context) {
    if let Some((reg, mut keys)) = cx.editor.macro_recording.take() {
        // Remove the keypress which ends the recording
        keys.pop();
        let s = keys
            .into_iter()
            .map(|key| {
                let s = key.to_string();
                if s.chars().count() == 1 {
                    s
                } else {
                    format!("<{}>", s)
                }
            })
            .collect::<String>();
        match cx.editor.registers.write(reg, vec![s]) {
            Ok(_) => cx
                .editor
                .set_status(format!("Recorded to register [{}]", reg)),
            Err(err) => cx.editor.set_error(err.to_string()),
        }
    } else {
        let reg = cx.register.take().unwrap_or('@');
        cx.editor.macro_recording = Some((reg, Vec::new()));
        cx.editor
            .set_status(format!("Recording to register [{}]", reg));
    }
}

fn replay_macro(cx: &mut Context) {
    let reg = cx.register.unwrap_or('@');

    if cx.editor.macro_replaying.contains(&reg) {
        cx.editor.set_error(format!(
            "Cannot replay from register [{}] because already replaying from same register",
            reg
        ));
        return;
    }

    let keys: Vec<KeyEvent> = if let Some(keys) = cx
        .editor
        .registers
        .read(reg, cx.editor)
        .filter(|values| values.len() == 1)
        .map(|mut values| values.next().unwrap())
    {
        match silicon_view::input::parse_macro(&keys) {
            Ok(keys) => keys,
            Err(err) => {
                cx.editor.set_error(format!("Invalid macro: {}", err));
                return;
            }
        }
    } else {
        cx.editor.set_error(format!("Register [{}] empty", reg));
        return;
    };

    // Once the macro has been fully validated, it's marked as being under replay
    // to ensure we don't fall into infinite recursion.
    cx.editor.macro_replaying.push(reg);

    let count = cx.count();
    cx.callback.push(Box::new(move |compositor, cx| {
        for _ in 0..count {
            for &key in keys.iter() {
                compositor.handle_event(&compositor::Event::Key(key), cx);
            }
        }
        // The macro under replay is cleared at the end of the callback, not in the
        // macro replay context, or it will not correctly protect the user from
        // replaying recursively.
        cx.editor.macro_replaying.pop();
    }));
}


fn lsp_or_syntax_symbol_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);

    if doc
        .language_servers_with_feature(LanguageServerFeature::DocumentSymbols)
        .next()
        .is_some()
    {
        lsp::symbol_picker(cx);
    } else if doc.syntax().is_some() {
        syntax_symbol_picker(cx);
    } else {
        cx.editor
            .set_error("No language server supporting document symbols or syntax info available");
    }
}

fn lsp_or_syntax_workspace_symbol_picker(cx: &mut Context) {
    let doc = doc!(cx.editor);

    if doc
        .language_servers_with_feature(LanguageServerFeature::WorkspaceSymbols)
        .next()
        .is_some()
    {
        lsp::workspace_symbol_picker(cx);
    } else {
        syntax_workspace_symbol_picker(cx);
    }
}

fn toggle_terminal_panel(cx: &mut Context) {
    let callback = async move { Ok(crate::job::Callback::OpenTerminalPanel) };
    cx.jobs.callback(callback);
}

fn new_terminal_tab(cx: &mut Context) {
    let callback = async move { Ok(crate::job::Callback::NewTerminalTab) };
    cx.jobs.callback(callback);
}

/// Built-in default runners for common file extensions.
///
/// For C files, uses multi-file detection matching the Zed run.sh behavior:
/// if only one file in the directory has `main()`, compile all `.c` files together;
/// otherwise compile just the current file.
fn builtin_runner(ext: &str) -> Option<&'static str> {
    match ext {
        "c" => Some(concat!(
            "DIR=\"$(dirname \"{file}\")\" && ",
            "MAIN_COUNT=$(grep -rl '\\bmain\\s*(' \"$DIR\"/*.c 2>/dev/null | wc -l | tr -d ' ') && ",
            "if [ \"$MAIN_COUNT\" -gt 1 ]; then ",
            "clang -std=c11 -Wall -Wextra -o /tmp/{name} \"{file}\" && /tmp/{name}; ",
            "else ",
            "clang -std=c11 -Wall -Wextra -o /tmp/{name} \"$DIR\"/*.c && /tmp/{name}; ",
            "fi",
        )),
        "py" => Some("python3 \"{file}\""),
        "rs" => Some("cargo run"),
        "js" => Some("node \"{file}\""),
        "ts" => Some("npx tsx \"{file}\""),
        "go" => Some("go run \"{file}\""),
        "cs" => Some("cd \"{dir}\" && dotnet run"),
        "sh" => Some("bash \"{file}\""),
        _ => None,
    }
}

/// Expand `{file}`, `{name}`, `{dir}` placeholders in a runner command template.
fn expand_runner_template(template: &str, path: &std::path::Path) -> String {
    let file = path.to_string_lossy();
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();
    let dir = path
        .parent()
        .map(|p| p.to_string_lossy())
        .unwrap_or_default();
    template
        .replace("{file}", &file)
        .replace("{name}", &name)
        .replace("{dir}", &dir)
}

fn run_file(cx: &mut Context) {
    let doc = doc!(cx.editor);
    let path = match doc.path() {
        Some(p) => p.clone(),
        None => {
            cx.editor.set_error("Buffer has no file path");
            return;
        }
    };

    // Save the file before running (matches Zed cmd-r → save then run).
    let doc_id = doc!(cx.editor).id();
    if cx.editor.documents[&doc_id].is_modified() {
        if let Err(err) = cx.editor.save::<std::path::PathBuf>(doc_id, None, false) {
            cx.editor.set_error(format!("Save failed: {err}"));
            return;
        }
    }

    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_string(),
        None => {
            cx.editor.set_error("File has no extension");
            return;
        }
    };

    // Resolve runner: user-defined first, then built-in defaults.
    let template = if let Some(user_cmd) = cx.editor.runners.get(&ext) {
        user_cmd.clone()
    } else if let Some(builtin) = builtin_runner(&ext) {
        builtin.to_string()
    } else {
        cx.editor
            .set_error(format!("No runner for .{ext}"));
        return;
    };

    let expanded = expand_runner_template(&template, &path);
    // Clear terminal before running, matching Zed behavior.
    let cmd = format!("clear && {expanded}");
    // Only pass the shell program (e.g. "/bin/sh"), not "-c" — the terminal
    // spawns an interactive shell and types the command into it.
    let shell_program = cx
        .editor
        .config()
        .shell
        .first()
        .cloned()
        .unwrap_or_else(|| "sh".to_string());
    let shell = vec![shell_program];

    let callback = async move {
        Ok(crate::job::Callback::RunInTerminal { shell, cmd })
    };
    cx.jobs.callback(callback);
}

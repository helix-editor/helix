pub(crate) mod dap;
pub(crate) mod lsp;
pub(crate) mod typed;

pub use dap::*;
use futures_util::FutureExt;
use helix_event::status;
use helix_stdx::{
    path::{self, find_paths},
    rope::{self, RopeSliceExt},
};
use helix_vcs::{FileChange, Hunk};
pub use lsp::*;
use tui::{
    text::{Span, Spans},
    widgets::Cell,
};
pub use typed::*;

use helix_core::{
    char_idx_at_visual_offset,
    chars::char_is_word,
    command_line, comment,
    doc_formatter::TextFormat,
    encoding, find_workspace,
    graphemes::{self, next_grapheme_boundary},
    history::UndoKind,
    increment,
    indent::{self, IndentStyle},
    line_ending::{get_line_ending_of_str, line_end_char_index},
    match_brackets,
    movement::{self, move_vertically_visual, Direction},
    object, pos_at_coords,
    regex::{self, Regex},
    search::{self, CharMatcher},
    selection, surround,
    syntax::config::{BlockCommentToken, LanguageServerFeature},
    text_annotations::{Overlay, TextAnnotations},
    textobject,
    unicode::width::UnicodeWidthChar,
    visual_offset_from_block, Deletion, LineEnding, Position, Range, Rope, RopeReader, RopeSlice,
    Selection, SmallVec, Syntax, Tendril, Transaction,
};
use helix_view::{
    document::{FormatterError, Mode, SCRATCH_BUFFER_NAME},
    editor::Action,
    icons::ICONS,
    info::Info,
    input::KeyEvent,
    keyboard::KeyCode,
    theme::Style,
    tree,
    view::View,
    Document, DocumentId, Editor, ViewId,
};

use anyhow::{anyhow, bail, ensure, Context as _};
use insert::*;
use movement::Movement;

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
        call: impl Future<Output = helix_lsp::Result<T>> + 'static + Send,
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
    call: impl Future<Output = helix_lsp::Result<T>> + 'static + Send,
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

use helix_view::{align_view, Align};

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
        changed_file_picker, "Open changed file picker",
        select_references_to_symbol_under_cursor, "Select symbol references",
        workspace_symbol_picker, "Open workspace symbol picker",
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
            helix_view::input::parse_macro(suffix).map(|keys| Self::Macro {
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

fn no_op(_cx: &mut Context) {}

type MoveFn =
    fn(RopeSlice, Range, Direction, usize, Movement, &TextFormat, &mut TextAnnotations) -> Range;

fn move_impl(cx: &mut Context, move_fn: MoveFn, dir: Direction, behaviour: Movement) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let text_fmt = doc.text_format(view.inner_area(doc).width, None);
    let mut annotations = view.text_annotations(doc, None);

    let selection = doc.selection(view.id).clone().transform(|range| {
        move_fn(
            text,
            range,
            dir,
            count,
            behaviour,
            &text_fmt,
            &mut annotations,
        )
    });
    drop(annotations);
    doc.set_selection(view.id, selection);
}

use helix_core::movement::{move_horizontally, move_vertically};

fn move_char_left(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Backward, Movement::Move)
}

fn move_char_right(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Forward, Movement::Move)
}

fn move_line_up(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Backward, Movement::Move)
}

fn move_line_down(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Forward, Movement::Move)
}

fn move_visual_line_up(cx: &mut Context) {
    move_impl(
        cx,
        move_vertically_visual,
        Direction::Backward,
        Movement::Move,
    )
}

fn move_visual_line_down(cx: &mut Context) {
    move_impl(
        cx,
        move_vertically_visual,
        Direction::Forward,
        Movement::Move,
    )
}

fn extend_char_left(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Backward, Movement::Extend)
}

fn extend_char_right(cx: &mut Context) {
    move_impl(cx, move_horizontally, Direction::Forward, Movement::Extend)
}

fn extend_line_up(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Backward, Movement::Extend)
}

fn extend_line_down(cx: &mut Context) {
    move_impl(cx, move_vertically, Direction::Forward, Movement::Extend)
}

fn extend_visual_line_up(cx: &mut Context) {
    move_impl(
        cx,
        move_vertically_visual,
        Direction::Backward,
        Movement::Extend,
    )
}

fn extend_visual_line_down(cx: &mut Context) {
    move_impl(
        cx,
        move_vertically_visual,
        Direction::Forward,
        Movement::Extend,
    )
}

fn goto_line_end_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let line_start = text.line_to_char(line);

        let pos = graphemes::prev_grapheme_boundary(text, line_end_char_index(&text, line))
            .max(line_start);

        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn goto_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_impl(
        view,
        doc,
        if cx.editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

fn extend_to_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_impl(view, doc, Movement::Extend)
}

fn goto_line_end_newline_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let pos = line_end_char_index(&text, line);

        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn goto_line_end_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_newline_impl(
        view,
        doc,
        if cx.editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

fn extend_to_line_end_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_newline_impl(view, doc, Movement::Extend)
}

fn goto_line_start_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);

        // adjust to start of the line
        let pos = text.line_to_char(line);
        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn goto_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_start_impl(
        view,
        doc,
        if cx.editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

fn goto_next_buffer(cx: &mut Context) {
    goto_buffer(cx.editor, Direction::Forward, cx.count());
}

fn goto_previous_buffer(cx: &mut Context) {
    goto_buffer(cx.editor, Direction::Backward, cx.count());
}

fn goto_buffer(editor: &mut Editor, direction: Direction, count: usize) {
    let current = view!(editor).doc;

    let id = match direction {
        Direction::Forward => {
            let iter = editor.documents.keys();
            // skip 'count' times past current buffer
            iter.cycle().skip_while(|id| *id != &current).nth(count)
        }
        Direction::Backward => {
            let iter = editor.documents.keys();
            // skip 'count' times past current buffer
            iter.rev()
                .cycle()
                .skip_while(|id| *id != &current)
                .nth(count)
        }
    }
    .unwrap();

    let id = *id;

    editor.switch(id, Action::Replace);
}

fn extend_to_line_start(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_start_impl(view, doc, Movement::Extend)
}

fn kill_to_line_start(cx: &mut Context) {
    delete_by_selection_insert_mode(
        cx,
        move |text, range| {
            let line = range.cursor_line(text);
            let first_char = text.line_to_char(line);
            let anchor = range.cursor(text);
            let head = if anchor == first_char && line != 0 {
                // select until previous line
                line_end_char_index(&text, line - 1)
            } else if let Some(pos) = text.line(line).first_non_whitespace_char() {
                if first_char + pos < anchor {
                    // select until first non-blank in line if cursor is after it
                    first_char + pos
                } else {
                    // select until start of line
                    first_char
                }
            } else {
                // select until start of line
                first_char
            };
            (head, anchor)
        },
        Direction::Backward,
    );
}

fn kill_to_line_end(cx: &mut Context) {
    delete_by_selection_insert_mode(
        cx,
        |text, range| {
            let line = range.cursor_line(text);
            let line_end_pos = line_end_char_index(&text, line);
            let pos = range.cursor(text);

            // if the cursor is on the newline char delete that
            if pos == line_end_pos {
                (pos, text.line_to_char(line + 1))
            } else {
                (pos, line_end_pos)
            }
        },
        Direction::Forward,
    );
}

fn goto_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    goto_first_nonwhitespace_impl(
        view,
        doc,
        if cx.editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        },
    )
}

fn extend_to_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_first_nonwhitespace_impl(view, doc, Movement::Extend)
}

fn goto_first_nonwhitespace_impl(view: &mut View, doc: &mut Document, movement: Movement) {
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);

        if let Some(pos) = text.line(line).first_non_whitespace_char() {
            let pos = pos + text.line_to_char(line);
            range.put_cursor(text, pos, movement == Movement::Extend)
        } else {
            range
        }
    });
    doc.set_selection(view.id, selection);
}

fn trim_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let ranges: SmallVec<[Range; 1]> = doc
        .selection(view.id)
        .iter()
        .filter_map(|range| {
            if range.is_empty() || range.slice(text).chars().all(|ch| ch.is_whitespace()) {
                return None;
            }
            let mut start = range.from();
            let mut end = range.to();
            start = movement::skip_while(text, start, |x| x.is_whitespace()).unwrap_or(start);
            end = movement::backwards_skip_while(text, end, |x| x.is_whitespace()).unwrap_or(end);
            Some(Range::new(start, end).with_direction(range.direction()))
        })
        .collect();

    if !ranges.is_empty() {
        let primary = doc.selection(view.id).primary();
        let idx = ranges
            .iter()
            .position(|range| range.overlaps(&primary))
            .unwrap_or(ranges.len() - 1);
        doc.set_selection(view.id, Selection::new(ranges, idx));
    } else {
        collapse_selection(cx);
        keep_primary_selection(cx);
    };
}

// align text in selection
#[allow(deprecated)]
fn align_selections(cx: &mut Context) {
    use helix_core::visual_coords_at_pos;

    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);

    let tab_width = doc.tab_width();
    let mut column_widths: Vec<Vec<_>> = Vec::new();
    let mut last_line = text.len_lines() + 1;
    let mut col = 0;

    for range in selection {
        let coords = visual_coords_at_pos(text, range.head, tab_width);
        let anchor_coords = visual_coords_at_pos(text, range.anchor, tab_width);

        if coords.row != anchor_coords.row {
            cx.editor
                .set_error("align cannot work with multi line selections");
            return;
        }

        col = if coords.row == last_line { col + 1 } else { 0 };

        if col >= column_widths.len() {
            column_widths.push(Vec::new());
        }
        column_widths[col].push((range.from(), coords.col));

        last_line = coords.row;
    }

    let mut changes = Vec::with_capacity(selection.len());

    // Account for changes on each row
    let len = column_widths.first().map(|cols| cols.len()).unwrap_or(0);
    let mut offs = vec![0; len];

    for col in column_widths {
        let max_col = col
            .iter()
            .enumerate()
            .map(|(row, (_, cursor))| *cursor + offs[row])
            .max()
            .unwrap_or(0);

        for (row, (insert_pos, last_col)) in col.into_iter().enumerate() {
            let ins_count = max_col - (last_col + offs[row]);

            if ins_count == 0 {
                continue;
            }

            offs[row] += ins_count;

            changes.push((insert_pos, insert_pos, Some(" ".repeat(ins_count).into())));
        }
    }

    // The changeset has to be sorted
    changes.sort_unstable_by_key(|(from, _, _)| *from);

    let transaction = Transaction::change(doc.text(), changes.into_iter());
    doc.apply(&transaction, view.id);
    exit_select_mode(cx);
}

fn goto_window(cx: &mut Context, align: Align) {
    let count = cx.count() - 1;
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);
    let view_offset = doc.view_offset(view.id);

    let height = view.inner_height();

    // respect user given count if any
    // - 1 so we have at least one gap in the middle.
    // a height of 6 with padding of 3 on each side will keep shifting the view back and forth
    // as we type
    let scrolloff = config.scrolloff.min(height.saturating_sub(1) / 2);

    let last_visual_line = view.last_visual_line(doc);

    let visual_line = match align {
        Align::Top => view_offset.vertical_offset + scrolloff + count,
        Align::Center => view_offset.vertical_offset + (last_visual_line / 2),
        Align::Bottom => {
            view_offset.vertical_offset + last_visual_line.saturating_sub(scrolloff + count)
        }
    };
    let visual_line = visual_line
        .max(view_offset.vertical_offset + scrolloff)
        .min(view_offset.vertical_offset + last_visual_line.saturating_sub(scrolloff));

    let pos = view
        .pos_at_visual_coords(doc, visual_line as u16, 0, false)
        .expect("visual_line was constrained to the view area");

    let text = doc.text().slice(..);
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.put_cursor(text, pos, cx.editor.mode == Mode::Select));
    doc.set_selection(view.id, selection);
}

fn goto_window_top(cx: &mut Context) {
    goto_window(cx, Align::Top)
}

fn goto_window_center(cx: &mut Context) {
    goto_window(cx, Align::Center)
}

fn goto_window_bottom(cx: &mut Context) {
    goto_window(cx, Align::Bottom)
}

fn move_word_impl<F>(cx: &mut Context, move_fn: F)
where
    F: Fn(RopeSlice, Range, usize) -> Range,
{
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| move_fn(text, range, count));
    doc.set_selection(view.id, selection);
}

fn move_next_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_word_start)
}

fn move_prev_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_prev_word_start)
}

fn move_prev_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_prev_word_end)
}

fn move_next_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_word_end)
}

fn move_next_long_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_long_word_start)
}

fn move_prev_long_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_prev_long_word_start)
}

fn move_prev_long_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_prev_long_word_end)
}

fn move_next_long_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_long_word_end)
}

fn move_next_sub_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_sub_word_start)
}

fn move_prev_sub_word_start(cx: &mut Context) {
    move_word_impl(cx, movement::move_prev_sub_word_start)
}

fn move_prev_sub_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_prev_sub_word_end)
}

fn move_next_sub_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_sub_word_end)
}

fn goto_para_impl<F>(cx: &mut Context, move_fn: F)
where
    F: Fn(RopeSlice, Range, usize, Movement) -> Range + 'static,
{
    let count = cx.count();
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);
        let text = doc.text().slice(..);
        let behavior = if editor.mode == Mode::Select {
            Movement::Extend
        } else {
            Movement::Move
        };

        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| move_fn(text, range, count, behavior));
        doc.set_selection(view.id, selection);
    };
    cx.editor.apply_motion(motion)
}

fn goto_prev_paragraph(cx: &mut Context) {
    goto_para_impl(cx, movement::move_prev_paragraph)
}

fn goto_next_paragraph(cx: &mut Context) {
    goto_para_impl(cx, movement::move_next_paragraph)
}

fn goto_file_start(cx: &mut Context) {
    goto_file_start_impl(cx, Movement::Move);
}

fn extend_to_file_start(cx: &mut Context) {
    goto_file_start_impl(cx, Movement::Extend);
}

fn goto_file_start_impl(cx: &mut Context, movement: Movement) {
    if cx.count.is_some() {
        goto_line_impl(cx, movement);
    } else {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, 0, movement == Movement::Extend));
        push_jump(view, doc);
        doc.set_selection(view.id, selection);
    }
}

fn goto_file_end(cx: &mut Context) {
    goto_file_end_impl(cx, Movement::Move);
}

fn extend_to_file_end(cx: &mut Context) {
    goto_file_end_impl(cx, Movement::Extend)
}

fn goto_file_end_impl(cx: &mut Context, movement: Movement) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let pos = doc.text().len_chars();
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.put_cursor(text, pos, movement == Movement::Extend));
    push_jump(view, doc);
    doc.set_selection(view.id, selection);
}

fn goto_file(cx: &mut Context) {
    goto_file_impl(cx, Action::Replace);
}

fn goto_file_hsplit(cx: &mut Context) {
    goto_file_impl(cx, Action::HorizontalSplit);
}

fn goto_file_vsplit(cx: &mut Context) {
    goto_file_impl(cx, Action::VerticalSplit);
}

/// Goto files in selection.
fn goto_file_impl(cx: &mut Context, action: Action) {
    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text().slice(..);
    let selections = doc.selection(view.id);
    let primary = selections.primary();
    let rel_path = doc
        .relative_path()
        .map(|path| path.parent().unwrap().to_path_buf())
        .unwrap_or_default();

    let paths: Vec<_> = if selections.len() == 1 && primary.len() == 1 {
        // Cap the search at roughly 1k bytes around the cursor.
        let lookaround = 1000;
        let pos = text.char_to_byte(primary.cursor(text));
        let search_start = text
            .line_to_byte(text.byte_to_line(pos))
            .max(text.floor_char_boundary(pos.saturating_sub(lookaround)));
        let search_end = text
            .line_to_byte(text.byte_to_line(pos) + 1)
            .min(text.ceil_char_boundary(pos + lookaround));
        let search_range = text.byte_slice(search_start..search_end);
        // we also allow paths that are next to the cursor (can be ambiguous but
        // rarely so in practice) so that gf on quoted/braced path works (not sure about this
        // but apparently that is how gf has worked historically in helix)
        let path = find_paths(search_range, true)
            .take_while(|range| search_start + range.start <= pos + 1)
            .find(|range| pos <= search_start + range.end)
            .map(|range| Cow::from(search_range.byte_slice(range)));
        log::debug!("goto_file auto-detected path: {path:?}");
        let path = path.unwrap_or_else(|| primary.fragment(text));
        vec![path.into_owned()]
    } else {
        // Otherwise use each selection, trimmed.
        selections
            .fragments(text)
            .map(|sel| sel.trim().to_owned())
            .filter(|sel| !sel.is_empty())
            .collect()
    };

    for sel in paths {
        if let Ok(url) = Url::parse(&sel) {
            open_url(cx, url, action);
            continue;
        }

        let path = path::expand(&sel);
        let path = &rel_path.join(path);
        if path.is_dir() {
            let picker = ui::file_picker(cx.editor, path.into());
            cx.push_layer(Box::new(overlaid(picker)));
        } else if let Err(e) = cx.editor.open(path, action) {
            cx.editor.set_error(format!("Open file failed: {:?}", e));
        }
    }
}

/// Opens the given url. If the URL points to a valid textual file it is open in helix.
//  Otherwise, the file is open using external program.
fn open_url(cx: &mut Context, url: Url, action: Action) {
    let doc = doc!(cx.editor);
    let rel_path = doc
        .relative_path()
        .map(|path| path.parent().unwrap().to_path_buf())
        .unwrap_or_default();

    if url.scheme() != "file" {
        return cx.jobs.callback(crate::open_external_url_callback(url));
    }

    let content_type = std::fs::File::open(url.path()).and_then(|file| {
        // Read up to 1kb to detect the content type
        let mut read_buffer = Vec::new();
        let n = file.take(1024).read_to_end(&mut read_buffer)?;
        Ok(content_inspector::inspect(&read_buffer[..n]))
    });

    // we attempt to open binary files - files that can't be open in helix - using external
    // program as well, e.g. pdf files or images
    match content_type {
        Ok(content_inspector::ContentType::BINARY) => {
            cx.jobs.callback(crate::open_external_url_callback(url))
        }
        Ok(_) | Err(_) => {
            let path = &rel_path.join(url.path());
            if path.is_dir() {
                let picker = ui::file_picker(cx.editor, path.into());
                cx.push_layer(Box::new(overlaid(picker)));
            } else if let Err(e) = cx.editor.open(path, action) {
                cx.editor.set_error(format!("Open file failed: {:?}", e));
            }
        }
    }
}

fn extend_word_impl<F>(cx: &mut Context, extend_fn: F)
where
    F: Fn(RopeSlice, Range, usize) -> Range,
{
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let word = extend_fn(text, range, count);
        let pos = word.cursor(text);
        range.put_cursor(text, pos, true)
    });
    doc.set_selection(view.id, selection);
}

fn extend_next_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_word_start)
}

fn extend_prev_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_word_start)
}

fn extend_next_word_end(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_word_end)
}

fn extend_prev_word_end(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_word_end)
}

fn extend_next_long_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_start)
}

fn extend_prev_long_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_long_word_start)
}

fn extend_prev_long_word_end(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_long_word_end)
}

fn extend_next_long_word_end(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_end)
}

fn extend_next_sub_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_sub_word_start)
}

fn extend_prev_sub_word_start(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_sub_word_start)
}

fn extend_prev_sub_word_end(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_sub_word_end)
}

fn extend_next_sub_word_end(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_sub_word_end)
}

/// Separate branch to find_char designed only for `<ret>` char.
//
// This is necessary because the one document can have different line endings inside. And we
// cannot predict what character to find when <ret> is pressed. On the current line it can be `lf`
// but on the next line it can be `crlf`. That's why [`find_char_impl`] cannot be applied here.
fn find_char_line_ending(
    cx: &mut Context,
    count: usize,
    direction: Direction,
    inclusive: bool,
    extend: bool,
) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let cursor = range.cursor(text);
        let cursor_line = range.cursor_line(text);

        // Finding the line where we're going to find <ret>. Depends mostly on
        // `count`, but also takes into account edge cases where we're already at the end
        // of a line or the beginning of a line
        let find_on_line = match direction {
            Direction::Forward => {
                let on_edge = line_end_char_index(&text, cursor_line) == cursor;
                let line = cursor_line + count - 1 + (on_edge as usize);
                if line >= text.len_lines() - 1 {
                    return range;
                } else {
                    line
                }
            }
            Direction::Backward => {
                let on_edge = text.line_to_char(cursor_line) == cursor && !inclusive;
                let line = cursor_line as isize - (count as isize - 1 + on_edge as isize);
                if line <= 0 {
                    return range;
                } else {
                    line as usize
                }
            }
        };

        let pos = match (direction, inclusive) {
            (Direction::Forward, true) => line_end_char_index(&text, find_on_line),
            (Direction::Forward, false) => line_end_char_index(&text, find_on_line) - 1,
            (Direction::Backward, true) => line_end_char_index(&text, find_on_line - 1),
            (Direction::Backward, false) => text.line_to_char(find_on_line),
        };

        if extend {
            range.put_cursor(text, pos, true)
        } else {
            Range::point(range.cursor(text)).put_cursor(text, pos, true)
        }
    });
    doc.set_selection(view.id, selection);
}

fn find_char(cx: &mut Context, direction: Direction, inclusive: bool, extend: bool) {
    // TODO: count is reset to 1 before next key so we move it into the closure here.
    // Would be nice to carry over.
    let count = cx.count();

    // need to wait for next key
    // TODO: should this be done by grapheme rather than char?  For example,
    // we can't properly handle the line-ending CRLF case here in terms of char.
    cx.on_next_key(move |cx, event| {
        let ch = match event {
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                find_char_line_ending(cx, count, direction, inclusive, extend);
                return;
            }

            KeyEvent {
                code: KeyCode::Tab, ..
            } => '\t',

            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => ch,
            _ => return,
        };
        let motion = move |editor: &mut Editor| {
            match direction {
                Direction::Forward => {
                    find_char_impl(editor, &find_next_char_impl, inclusive, extend, ch, count)
                }
                Direction::Backward => {
                    find_char_impl(editor, &find_prev_char_impl, inclusive, extend, ch, count)
                }
            };
        };

        cx.editor.apply_motion(motion);
    })
}

//

#[inline]
fn find_char_impl<F, M: CharMatcher + Clone + Copy>(
    editor: &mut Editor,
    search_fn: &F,
    inclusive: bool,
    extend: bool,
    char_matcher: M,
    count: usize,
) where
    F: Fn(RopeSlice, M, usize, usize, bool) -> Option<usize> + 'static,
{
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        // TODO: use `Range::cursor()` here instead.  However, that works in terms of
        // graphemes, whereas this function doesn't yet.  So we're doing the same logic
        // here, but just in terms of chars instead.
        let search_start_pos = if range.anchor < range.head {
            range.head - 1
        } else {
            range.head
        };

        search_fn(text, char_matcher, search_start_pos, count, inclusive).map_or(range, |pos| {
            if extend {
                range.put_cursor(text, pos, true)
            } else {
                Range::point(range.cursor(text)).put_cursor(text, pos, true)
            }
        })
    });
    doc.set_selection(view.id, selection);
}

fn find_next_char_impl(
    text: RopeSlice,
    ch: char,
    pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    let pos = (pos + 1).min(text.len_chars());
    if inclusive {
        search::find_nth_next(text, ch, pos, n)
    } else {
        let n = match text.get_char(pos) {
            Some(next_ch) if next_ch == ch => n + 1,
            _ => n,
        };
        search::find_nth_next(text, ch, pos, n).map(|n| n.saturating_sub(1))
    }
}

fn find_prev_char_impl(
    text: RopeSlice,
    ch: char,
    pos: usize,
    n: usize,
    inclusive: bool,
) -> Option<usize> {
    if inclusive {
        search::find_nth_prev(text, ch, pos, n)
    } else {
        let n = match text.get_char(pos.saturating_sub(1)) {
            Some(next_ch) if next_ch == ch => n + 1,
            _ => n,
        };
        search::find_nth_prev(text, ch, pos, n).map(|n| (n + 1).min(text.len_chars()))
    }
}

fn find_till_char(cx: &mut Context) {
    find_char(cx, Direction::Forward, false, false);
}

fn find_next_char(cx: &mut Context) {
    find_char(cx, Direction::Forward, true, false)
}

fn extend_till_char(cx: &mut Context) {
    find_char(cx, Direction::Forward, false, true)
}

fn extend_next_char(cx: &mut Context) {
    find_char(cx, Direction::Forward, true, true)
}

fn till_prev_char(cx: &mut Context) {
    find_char(cx, Direction::Backward, false, false)
}

fn find_prev_char(cx: &mut Context) {
    find_char(cx, Direction::Backward, true, false)
}

fn extend_till_prev_char(cx: &mut Context) {
    find_char(cx, Direction::Backward, false, true)
}

fn extend_prev_char(cx: &mut Context) {
    find_char(cx, Direction::Backward, true, true)
}

fn repeat_last_motion(cx: &mut Context) {
    cx.editor.repeat_last_motion(cx.count())
}

fn replace(cx: &mut Context) {
    let mut buf = [0u8; 4]; // To hold utf8 encoded char.

    // need to wait for next key
    cx.on_next_key(move |cx, event| {
        let (view, doc) = current!(cx.editor);
        let ch: Option<&str> = match event {
            KeyEvent {
                code: KeyCode::Char(ch),
                ..
            } => Some(ch.encode_utf8(&mut buf[..])),
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => Some(doc.line_ending.as_str()),
            KeyEvent {
                code: KeyCode::Tab, ..
            } => Some("\t"),
            _ => None,
        };

        let selection = doc.selection(view.id);

        if let Some(ch) = ch {
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                if !range.is_empty() {
                    let text: Tendril = doc
                        .text()
                        .slice(range.from()..range.to())
                        .graphemes()
                        .map(|_g| ch)
                        .collect();
                    (range.from(), range.to(), Some(text))
                } else {
                    // No change.
                    (range.from(), range.to(), None)
                }
            });

            doc.apply(&transaction, view.id);
            exit_select_mode(cx);
        }
    })
}

fn switch_case_impl<F>(cx: &mut Context, change_fn: F)
where
    F: Fn(RopeSlice) -> Tendril,
{
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        let text: Tendril = change_fn(range.slice(doc.text().slice(..)));

        (range.from(), range.to(), Some(text))
    });

    doc.apply(&transaction, view.id);
    exit_select_mode(cx);
}

enum CaseSwitcher {
    Upper(ToUppercase),
    Lower(ToLowercase),
    Keep(Option<char>),
}

impl Iterator for CaseSwitcher {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            CaseSwitcher::Upper(upper) => upper.next(),
            CaseSwitcher::Lower(lower) => lower.next(),
            CaseSwitcher::Keep(ch) => ch.take(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            CaseSwitcher::Upper(upper) => upper.size_hint(),
            CaseSwitcher::Lower(lower) => lower.size_hint(),
            CaseSwitcher::Keep(ch) => {
                let n = if ch.is_some() { 1 } else { 0 };
                (n, Some(n))
            }
        }
    }
}

impl ExactSizeIterator for CaseSwitcher {}

fn switch_case(cx: &mut Context) {
    switch_case_impl(cx, |string| {
        string
            .chars()
            .flat_map(|ch| {
                if ch.is_lowercase() {
                    CaseSwitcher::Upper(ch.to_uppercase())
                } else if ch.is_uppercase() {
                    CaseSwitcher::Lower(ch.to_lowercase())
                } else {
                    CaseSwitcher::Keep(Some(ch))
                }
            })
            .collect()
    });
}

fn switch_to_uppercase(cx: &mut Context) {
    switch_case_impl(cx, |string| {
        string.chunks().map(|chunk| chunk.to_uppercase()).collect()
    });
}

fn switch_to_lowercase(cx: &mut Context) {
    switch_case_impl(cx, |string| {
        string.chunks().map(|chunk| chunk.to_lowercase()).collect()
    });
}

pub fn scroll(cx: &mut Context, offset: usize, direction: Direction, sync_cursor: bool) {
    use Direction::*;
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);
    let mut view_offset = doc.view_offset(view.id);

    let range = doc.selection(view.id).primary();
    let text = doc.text().slice(..);

    let cursor = range.cursor(text);
    let height = view.inner_height();

    let scrolloff = config.scrolloff.min(height.saturating_sub(1) / 2);
    let offset = match direction {
        Forward => offset as isize,
        Backward => -(offset as isize),
    };

    let doc_text = doc.text().slice(..);
    let viewport = view.inner_area(doc);
    let text_fmt = doc.text_format(viewport.width, None);
    (view_offset.anchor, view_offset.vertical_offset) = char_idx_at_visual_offset(
        doc_text,
        view_offset.anchor,
        view_offset.vertical_offset as isize + offset,
        0,
        &text_fmt,
        // &annotations,
        &view.text_annotations(&*doc, None),
    );
    doc.set_view_offset(view.id, view_offset);

    let doc_text = doc.text().slice(..);
    let mut annotations = view.text_annotations(&*doc, None);

    if sync_cursor {
        let movement = match cx.editor.mode {
            Mode::Select => Movement::Extend,
            _ => Movement::Move,
        };
        // TODO: When inline diagnostics gets merged- 1. move_vertically_visual removes
        // line annotations/diagnostics so the cursor may jump further than the view.
        // 2. If the cursor lands on a complete line of virtual text, the cursor will
        // jump a different distance than the view.
        let selection = doc.selection(view.id).clone().transform(|range| {
            move_vertically_visual(
                doc_text,
                range,
                direction,
                offset.unsigned_abs(),
                movement,
                &text_fmt,
                &mut annotations,
            )
        });
        drop(annotations);
        doc.set_selection(view.id, selection);
        return;
    }

    let view_offset = doc.view_offset(view.id);

    let mut head;
    match direction {
        Forward => {
            let off;
            (head, off) = char_idx_at_visual_offset(
                doc_text,
                view_offset.anchor,
                (view_offset.vertical_offset + scrolloff) as isize,
                0,
                &text_fmt,
                &annotations,
            );
            head += (off != 0) as usize;
            if head <= cursor {
                return;
            }
        }
        Backward => {
            head = char_idx_at_visual_offset(
                doc_text,
                view_offset.anchor,
                (view_offset.vertical_offset + height - scrolloff - 1) as isize,
                0,
                &text_fmt,
                &annotations,
            )
            .0;
            if head >= cursor {
                return;
            }
        }
    }

    let anchor = if cx.editor.mode == Mode::Select {
        range.anchor
    } else {
        head
    };

    // replace primary selection with an empty selection at cursor pos
    let prim_sel = Range::new(anchor, head);
    let mut sel = doc.selection(view.id).clone();
    let idx = sel.primary_index();
    sel = sel.replace(idx, prim_sel);
    drop(annotations);
    doc.set_selection(view.id, sel);
}

fn page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Backward, false);
}

fn page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Forward, false);
}

fn half_page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Backward, false);
}

fn half_page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Forward, false);
}

fn page_cursor_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Backward, true);
}

fn page_cursor_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Forward, true);
}

fn page_cursor_half_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Backward, true);
}

fn page_cursor_half_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Forward, true);
}

#[allow(deprecated)]
// currently uses the deprecated `visual_coords_at_pos`/`pos_at_visual_coords` functions
// as this function ignores softwrapping (and virtual text) and instead only cares
// about "text visual position"
//
// TODO: implement a variant of that uses visual lines and respects virtual text
fn copy_selection_on_line(cx: &mut Context, direction: Direction) {
    use helix_core::{pos_at_visual_coords, visual_coords_at_pos};

    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);
    let mut ranges = SmallVec::with_capacity(selection.ranges().len() * (count + 1));
    ranges.extend_from_slice(selection.ranges());
    let mut primary_index = 0;
    for range in selection.iter() {
        let is_primary = *range == selection.primary();

        // The range is always head exclusive
        let (head, anchor) = if range.anchor < range.head {
            (range.head - 1, range.anchor)
        } else {
            (range.head, range.anchor.saturating_sub(1))
        };

        let tab_width = doc.tab_width();

        let head_pos = visual_coords_at_pos(text, head, tab_width);
        let anchor_pos = visual_coords_at_pos(text, anchor, tab_width);

        let height = std::cmp::max(head_pos.row, anchor_pos.row)
            - std::cmp::min(head_pos.row, anchor_pos.row)
            + 1;

        if is_primary {
            primary_index = ranges.len();
        }
        ranges.push(*range);

        let mut sels = 0;
        let mut i = 0;
        while sels < count {
            let offset = (i + 1) * height;

            let anchor_row = match direction {
                Direction::Forward => anchor_pos.row + offset,
                Direction::Backward => anchor_pos.row.saturating_sub(offset),
            };

            let head_row = match direction {
                Direction::Forward => head_pos.row + offset,
                Direction::Backward => head_pos.row.saturating_sub(offset),
            };

            if anchor_row >= text.len_lines() || head_row >= text.len_lines() {
                break;
            }

            let anchor =
                pos_at_visual_coords(text, Position::new(anchor_row, anchor_pos.col), tab_width);
            let head = pos_at_visual_coords(text, Position::new(head_row, head_pos.col), tab_width);

            // skip lines that are too short
            if visual_coords_at_pos(text, anchor, tab_width).col == anchor_pos.col
                && visual_coords_at_pos(text, head, tab_width).col == head_pos.col
            {
                if is_primary {
                    primary_index = ranges.len();
                }
                // This is Range::new(anchor, head), but it will place the cursor on the correct column
                ranges.push(Range::point(anchor).put_cursor(text, head, true));
                sels += 1;
            }

            if anchor_row == 0 && head_row == 0 {
                break;
            }

            i += 1;
        }
    }

    let selection = Selection::new(ranges, primary_index);
    doc.set_selection(view.id, selection);
}

fn copy_selection_on_prev_line(cx: &mut Context) {
    copy_selection_on_line(cx, Direction::Backward)
}

fn copy_selection_on_next_line(cx: &mut Context) {
    copy_selection_on_line(cx, Direction::Forward)
}

fn select_all(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let end = doc.text().len_chars();
    doc.set_selection(view.id, Selection::single(0, end))
}

fn select_regex(cx: &mut Context) {
    let reg = cx.register.unwrap_or('/');
    ui::regex_prompt(
        cx,
        "select:".into(),
        Some(reg),
        ui::completers::none,
        move |cx, regex, event| {
            let (view, doc) = current!(cx.editor);
            if !matches!(event, PromptEvent::Update | PromptEvent::Validate) {
                return;
            }
            let text = doc.text().slice(..);
            if let Some(selection) =
                selection::select_on_matches(text, doc.selection(view.id), &regex)
            {
                doc.set_selection(view.id, selection);
            } else {
                cx.editor.set_error("nothing selected");
            }
        },
    );
}

fn split_selection(cx: &mut Context) {
    let reg = cx.register.unwrap_or('/');
    ui::regex_prompt(
        cx,
        "split:".into(),
        Some(reg),
        ui::completers::none,
        move |cx, regex, event| {
            let (view, doc) = current!(cx.editor);
            if !matches!(event, PromptEvent::Update | PromptEvent::Validate) {
                return;
            }
            let text = doc.text().slice(..);
            let selection = selection::split_on_matches(text, doc.selection(view.id), &regex);
            doc.set_selection(view.id, selection);
        },
    );
}

fn split_selection_on_newline(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = selection::split_on_newline(text, doc.selection(view.id));
    doc.set_selection(view.id, selection);
}

fn merge_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id).clone().merge_ranges();
    doc.set_selection(view.id, selection);
}

fn merge_consecutive_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id).clone().merge_consecutive_ranges();
    doc.set_selection(view.id, selection);
}

#[allow(clippy::too_many_arguments)]
fn search_impl(
    editor: &mut Editor,
    regex: &rope::Regex,
    movement: Movement,
    direction: Direction,
    scrolloff: usize,
    wrap_around: bool,
    show_warnings: bool,
) {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);

    // Get the right side of the primary block cursor for forward search, or the
    // grapheme before the start of the selection for reverse search.
    let start = match direction {
        Direction::Forward => text.char_to_byte(graphemes::ensure_grapheme_boundary_next(
            text,
            selection.primary().to(),
        )),
        Direction::Backward => text.char_to_byte(graphemes::ensure_grapheme_boundary_prev(
            text,
            selection.primary().from(),
        )),
    };

    // A regex::Match returns byte-positions in the str. In the case where we
    // do a reverse search and wraparound to the end, we don't need to search
    // the text before the current cursor position for matches, but by slicing
    // it out, we need to add it back to the position of the selection.
    let doc = doc!(editor).text().slice(..);

    // use find_at to find the next match after the cursor, loop around the end
    // Careful, `Regex` uses `bytes` as offsets, not character indices!
    let mut mat = match direction {
        Direction::Forward => regex.find(doc.regex_input_at_bytes(start..)),
        Direction::Backward => regex.find_iter(doc.regex_input_at_bytes(..start)).last(),
    };

    if mat.is_none() {
        if wrap_around {
            mat = match direction {
                Direction::Forward => regex.find(doc.regex_input()),
                Direction::Backward => regex.find_iter(doc.regex_input_at_bytes(start..)).last(),
            };
        }
        if show_warnings {
            if wrap_around && mat.is_some() {
                editor.set_status("Wrapped around document");
            } else {
                editor.set_error("No more matches");
            }
        }
    }

    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id);

    if let Some(mat) = mat {
        let start = text.byte_to_char(mat.start());
        let end = text.byte_to_char(mat.end());

        if end == 0 {
            // skip empty matches that don't make sense
            return;
        }

        // Determine range direction based on the primary range
        let primary = selection.primary();
        let range = Range::new(start, end).with_direction(primary.direction());

        let selection = match movement {
            Movement::Extend => selection.clone().push(range),
            Movement::Move => selection.clone().replace(selection.primary_index(), range),
        };

        doc.set_selection(view.id, selection);
        view.ensure_cursor_in_view_center(doc, scrolloff);
    };
}

fn search_completions(cx: &mut Context, reg: Option<char>) -> Vec<String> {
    let mut items = reg
        .and_then(|reg| cx.editor.registers.read(reg, cx.editor))
        .map_or(Vec::new(), |reg| reg.take(200).collect());
    items.sort_unstable();
    items.dedup();
    items.into_iter().map(|value| value.to_string()).collect()
}

fn search(cx: &mut Context) {
    searcher(cx, Direction::Forward)
}

fn rsearch(cx: &mut Context) {
    searcher(cx, Direction::Backward)
}

fn searcher(cx: &mut Context, direction: Direction) {
    let reg = cx.register.unwrap_or('/');
    let config = cx.editor.config();
    let scrolloff = config.scrolloff;
    let wrap_around = config.search.wrap_around;
    let movement = if cx.editor.mode() == Mode::Select {
        Movement::Extend
    } else {
        Movement::Move
    };

    // TODO: could probably share with select_on_matches?
    let completions = search_completions(cx, Some(reg));

    ui::regex_prompt(
        cx,
        "search:".into(),
        Some(reg),
        move |_editor: &Editor, input: &str| {
            completions
                .iter()
                .filter(|comp| comp.starts_with(input))
                .map(|comp| (0.., comp.clone().into()))
                .collect()
        },
        move |cx, regex, event| {
            if event == PromptEvent::Validate {
                cx.editor.registers.last_search_register = reg;
            } else if event != PromptEvent::Update {
                return;
            }
            search_impl(
                cx.editor,
                &regex,
                movement,
                direction,
                scrolloff,
                wrap_around,
                false,
            );
        },
    );
}

fn search_next_or_prev_impl(cx: &mut Context, movement: Movement, direction: Direction) {
    let count = cx.count();
    let register = cx
        .register
        .unwrap_or(cx.editor.registers.last_search_register);
    let config = cx.editor.config();
    let scrolloff = config.scrolloff;
    if let Some(query) = cx.editor.registers.first(register, cx.editor) {
        let search_config = &config.search;
        let case_insensitive = if search_config.smart_case {
            !query.chars().any(char::is_uppercase)
        } else {
            false
        };
        let wrap_around = search_config.wrap_around;
        if let Ok(regex) = rope::RegexBuilder::new()
            .syntax(
                rope::Config::new()
                    .case_insensitive(case_insensitive)
                    .multi_line(true),
            )
            .build(&query)
        {
            for _ in 0..count {
                search_impl(
                    cx.editor,
                    &regex,
                    movement,
                    direction,
                    scrolloff,
                    wrap_around,
                    true,
                );
            }
        } else {
            let error = format!("Invalid regex: {}", query);
            cx.editor.set_error(error);
        }
    }
}

fn search_next(cx: &mut Context) {
    search_next_or_prev_impl(cx, Movement::Move, Direction::Forward);
}

fn search_prev(cx: &mut Context) {
    search_next_or_prev_impl(cx, Movement::Move, Direction::Backward);
}
fn extend_search_next(cx: &mut Context) {
    search_next_or_prev_impl(cx, Movement::Extend, Direction::Forward);
}

fn extend_search_prev(cx: &mut Context) {
    search_next_or_prev_impl(cx, Movement::Extend, Direction::Backward);
}

fn search_selection(cx: &mut Context) {
    search_selection_impl(cx, false)
}

fn search_selection_detect_word_boundaries(cx: &mut Context) {
    search_selection_impl(cx, true)
}

fn search_selection_impl(cx: &mut Context, detect_word_boundaries: bool) {
    fn is_at_word_start(text: RopeSlice, index: usize) -> bool {
        // This can happen when the cursor is at the last character in
        // the document +1 (ge + j), in this case text.char(index) will panic as
        // it will index out of bounds. See https://github.com/helix-editor/helix/issues/12609
        if index == text.len_chars() {
            return false;
        }
        let ch = text.char(index);
        if index == 0 {
            return char_is_word(ch);
        }
        let prev_ch = text.char(index - 1);

        !char_is_word(prev_ch) && char_is_word(ch)
    }

    fn is_at_word_end(text: RopeSlice, index: usize) -> bool {
        if index == 0 || index == text.len_chars() {
            return false;
        }
        let ch = text.char(index);
        let prev_ch = text.char(index - 1);

        char_is_word(prev_ch) && !char_is_word(ch)
    }

    let register = cx.register.unwrap_or('/');
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let regex = doc
        .selection(view.id)
        .iter()
        .map(|selection| {
            let add_boundary_prefix =
                detect_word_boundaries && is_at_word_start(text, selection.from());
            let add_boundary_suffix =
                detect_word_boundaries && is_at_word_end(text, selection.to());

            let prefix = if add_boundary_prefix { "\\b" } else { "" };
            let suffix = if add_boundary_suffix { "\\b" } else { "" };

            let word = regex::escape(&selection.fragment(text));
            format!("{}{}{}", prefix, word, suffix)
        })
        .collect::<HashSet<_>>() // Collect into hashset to deduplicate identical regexes
        .into_iter()
        .collect::<Vec<_>>()
        .join("|");

    let msg = format!("register '{}' set to '{}'", register, &regex);
    match cx.editor.registers.push(register, regex) {
        Ok(_) => {
            cx.editor.registers.last_search_register = register;
            cx.editor.set_status(msg)
        }
        Err(err) => cx.editor.set_error(err.to_string()),
    }
}

fn make_search_word_bounded(cx: &mut Context) {
    // Defaults to the active search register instead `/` to be more ergonomic assuming most people
    // would use this command following `search_selection`. This avoids selecting the register
    // twice.
    let register = cx
        .register
        .unwrap_or(cx.editor.registers.last_search_register);
    let regex = match cx.editor.registers.first(register, cx.editor) {
        Some(regex) => regex,
        None => return,
    };
    let start_anchored = regex.starts_with("\\b");
    let end_anchored = regex.ends_with("\\b");

    if start_anchored && end_anchored {
        return;
    }

    let mut new_regex = String::with_capacity(
        regex.len() + if start_anchored { 0 } else { 2 } + if end_anchored { 0 } else { 2 },
    );

    if !start_anchored {
        new_regex.push_str("\\b");
    }
    new_regex.push_str(&regex);
    if !end_anchored {
        new_regex.push_str("\\b");
    }

    let msg = format!("register '{}' set to '{}'", register, &new_regex);
    match cx.editor.registers.push(register, new_regex) {
        Ok(_) => {
            cx.editor.registers.last_search_register = register;
            cx.editor.set_status(msg)
        }
        Err(err) => cx.editor.set_error(err.to_string()),
    }
}

fn global_search(cx: &mut Context) {
    #[derive(Debug)]
    struct FileResult {
        path: PathBuf,
        /// 0 indexed lines
        line_num: usize,
    }

    impl FileResult {
        fn new(path: &Path, line_num: usize) -> Self {
            Self {
                path: path.to_path_buf(),
                line_num,
            }
        }
    }

    struct GlobalSearchConfig {
        smart_case: bool,
        file_picker_config: helix_view::editor::FilePickerConfig,
        directory_style: Style,
        number_style: Style,
        colon_style: Style,
    }

    let config = cx.editor.config();
    let config = GlobalSearchConfig {
        smart_case: config.search.smart_case,
        file_picker_config: config.file_picker.clone(),
        directory_style: cx.editor.theme.get("ui.text.directory"),
        number_style: cx.editor.theme.get("constant.numeric.integer"),
        colon_style: cx.editor.theme.get("punctuation"),
    };

    let columns = [
        PickerColumn::new("path", |item: &FileResult, config: &GlobalSearchConfig| {
            let path = helix_stdx::path::get_relative_path(&item.path);

            let directories = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| format!("{}{}", p.display(), std::path::MAIN_SEPARATOR))
                .unwrap_or_default();

            let filename = item
                .path
                .file_name()
                .expect("global search paths are normalized (can't end in `..`)")
                .to_string_lossy();

            Cell::from(Spans::from(vec![
                Span::styled(directories, config.directory_style),
                Span::raw(filename),
                Span::styled(":", config.colon_style),
                Span::styled((item.line_num + 1).to_string(), config.number_style),
            ]))
        }),
        PickerColumn::hidden("contents"),
    ];

    let get_files = |query: &str,
                     editor: &mut Editor,
                     config: std::sync::Arc<GlobalSearchConfig>,
                     injector: &ui::picker::Injector<_, _>| {
        if query.is_empty() {
            return async { Ok(()) }.boxed();
        }

        let search_root = helix_stdx::env::current_working_dir();
        if !search_root.exists() {
            return async { Err(anyhow::anyhow!("Current working directory does not exist")) }
                .boxed();
        }

        let documents: Vec<_> = editor
            .documents()
            .map(|doc| (doc.path().cloned(), doc.text().to_owned()))
            .collect();

        let matcher = match RegexMatcherBuilder::new()
            .case_smart(config.smart_case)
            .build(query)
        {
            Ok(matcher) => {
                // Clear any "Failed to compile regex" errors out of the statusline.
                editor.clear_status();
                matcher
            }
            Err(err) => {
                log::info!("Failed to compile search pattern in global search: {}", err);
                return async { Err(anyhow::anyhow!("Failed to compile regex")) }.boxed();
            }
        };

        let dedup_symlinks = config.file_picker_config.deduplicate_links;
        let absolute_root = search_root
            .canonicalize()
            .unwrap_or_else(|_| search_root.clone());

        let injector = injector.clone();
        async move {
            let searcher = SearcherBuilder::new()
                .binary_detection(BinaryDetection::quit(b'\x00'))
                .build();
            WalkBuilder::new(search_root)
                .hidden(config.file_picker_config.hidden)
                .parents(config.file_picker_config.parents)
                .ignore(config.file_picker_config.ignore)
                .follow_links(config.file_picker_config.follow_symlinks)
                .git_ignore(config.file_picker_config.git_ignore)
                .git_global(config.file_picker_config.git_global)
                .git_exclude(config.file_picker_config.git_exclude)
                .max_depth(config.file_picker_config.max_depth)
                .filter_entry(move |entry| {
                    filter_picker_entry(entry, &absolute_root, dedup_symlinks)
                })
                .add_custom_ignore_filename(helix_loader::config_dir().join("ignore"))
                .add_custom_ignore_filename(".helix/ignore")
                .build_parallel()
                .run(|| {
                    let mut searcher = searcher.clone();
                    let matcher = matcher.clone();
                    let injector = injector.clone();
                    let documents = &documents;
                    Box::new(move |entry: Result<DirEntry, ignore::Error>| -> WalkState {
                        let entry = match entry {
                            Ok(entry) => entry,
                            Err(_) => return WalkState::Continue,
                        };

                        match entry.file_type() {
                            Some(entry) if entry.is_file() => {}
                            // skip everything else
                            _ => return WalkState::Continue,
                        };

                        let mut stop = false;
                        let sink = sinks::UTF8(|line_num, _line_content| {
                            stop = injector
                                .push(FileResult::new(entry.path(), line_num as usize - 1))
                                .is_err();

                            Ok(!stop)
                        });
                        let doc = documents.iter().find(|&(doc_path, _)| {
                            doc_path
                                .as_ref()
                                .is_some_and(|doc_path| doc_path == entry.path())
                        });

                        let result = if let Some((_, doc)) = doc {
                            // there is already a buffer for this file
                            // search the buffer instead of the file because it's faster
                            // and captures new edits without requiring a save
                            if searcher.multi_line_with_matcher(&matcher) {
                                // in this case a continuous buffer is required
                                // convert the rope to a string
                                let text = doc.to_string();
                                searcher.search_slice(&matcher, text.as_bytes(), sink)
                            } else {
                                searcher.search_reader(
                                    &matcher,
                                    RopeReader::new(doc.slice(..)),
                                    sink,
                                )
                            }
                        } else {
                            searcher.search_path(&matcher, entry.path(), sink)
                        };

                        if let Err(err) = result {
                            log::error!("Global search error: {}, {}", entry.path().display(), err);
                        }
                        if stop {
                            WalkState::Quit
                        } else {
                            WalkState::Continue
                        }
                    })
                });
            Ok(())
        }
        .boxed()
    };

    let reg = cx.register.unwrap_or('/');
    cx.editor.registers.last_search_register = reg;

    let picker = Picker::new(
        columns,
        1, // contents
        [],
        config,
        move |cx, FileResult { path, line_num, .. }, action| {
            let doc = match cx.editor.open(path, action) {
                Ok(id) => doc_mut!(cx.editor, &id),
                Err(e) => {
                    cx.editor
                        .set_error(format!("Failed to open file '{}': {}", path.display(), e));
                    return;
                }
            };

            let line_num = *line_num;
            let view = view_mut!(cx.editor);
            let text = doc.text();
            if line_num >= text.len_lines() {
                cx.editor.set_error(
                    "The line you jumped to does not exist anymore because the file has changed.",
                );
                return;
            }
            let start = text.line_to_char(line_num);
            let end = text.line_to_char((line_num + 1).min(text.len_lines()));

            doc.set_selection(view.id, Selection::single(start, end));
            if action.align_view(view, doc.id()) {
                align_view(doc, view, Align::Center);
            }
        },
    )
    .with_preview(|_editor, FileResult { path, line_num, .. }| {
        Some((path.as_path().into(), Some((*line_num, *line_num))))
    })
    .with_history_register(Some(reg))
    .with_dynamic_query(get_files, Some(275));

    cx.push_layer(Box::new(overlaid(picker)));
}

enum Extend {
    Above,
    Below,
}

fn extend_line(cx: &mut Context) {
    let (view, doc) = current_ref!(cx.editor);
    let extend = match doc.selection(view.id).primary().direction() {
        Direction::Forward => Extend::Below,
        Direction::Backward => Extend::Above,
    };
    extend_line_impl(cx, extend);
}

fn extend_line_below(cx: &mut Context) {
    extend_line_impl(cx, Extend::Below);
}

fn extend_line_above(cx: &mut Context) {
    extend_line_impl(cx, Extend::Above);
}
fn extend_line_impl(cx: &mut Context, extend: Extend) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);

    let text = doc.text();
    let selection = doc.selection(view.id).clone().transform(|range| {
        let (start_line, end_line) = range.line_range(text.slice(..));

        let start = text.line_to_char(start_line);
        let end = text.line_to_char(
            (end_line + 1) // newline of end_line
                .min(text.len_lines()),
        );

        // extend to previous/next line if current line is selected
        let (anchor, head) = if range.from() == start && range.to() == end {
            match extend {
                Extend::Above => (end, text.line_to_char(start_line.saturating_sub(count))),
                Extend::Below => (
                    start,
                    text.line_to_char((end_line + count + 1).min(text.len_lines())),
                ),
            }
        } else {
            match extend {
                Extend::Above => (end, text.line_to_char(start_line.saturating_sub(count - 1))),
                Extend::Below => (
                    start,
                    text.line_to_char((end_line + count).min(text.len_lines())),
                ),
            }
        };

        Range::new(anchor, head)
    });

    doc.set_selection(view.id, selection);
}
fn select_line_below(cx: &mut Context) {
    select_line_impl(cx, Extend::Below);
}
fn select_line_above(cx: &mut Context) {
    select_line_impl(cx, Extend::Above);
}
fn select_line_impl(cx: &mut Context, extend: Extend) {
    let mut count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text();
    let saturating_add = |a: usize, b: usize| (a + b).min(text.len_lines());
    let selection = doc.selection(view.id).clone().transform(|range| {
        let (start_line, end_line) = range.line_range(text.slice(..));
        let start = text.line_to_char(start_line);
        let end = text.line_to_char(saturating_add(end_line, 1));
        let direction = range.direction();

        // Extending to line bounds is counted as one step
        if range.from() != start || range.to() != end {
            count = count.saturating_sub(1)
        }
        let (anchor_line, head_line) = match (&extend, direction) {
            (Extend::Above, Direction::Forward) => (start_line, end_line.saturating_sub(count)),
            (Extend::Above, Direction::Backward) => (end_line, start_line.saturating_sub(count)),
            (Extend::Below, Direction::Forward) => (start_line, saturating_add(end_line, count)),
            (Extend::Below, Direction::Backward) => (end_line, saturating_add(start_line, count)),
        };
        let (anchor, head) = match anchor_line.cmp(&head_line) {
            Ordering::Less => (
                text.line_to_char(anchor_line),
                text.line_to_char(saturating_add(head_line, 1)),
            ),
            Ordering::Equal => match extend {
                Extend::Above => (
                    text.line_to_char(saturating_add(anchor_line, 1)),
                    text.line_to_char(head_line),
                ),
                Extend::Below => (
                    text.line_to_char(head_line),
                    text.line_to_char(saturating_add(anchor_line, 1)),
                ),
            },

            Ordering::Greater => (
                text.line_to_char(saturating_add(anchor_line, 1)),
                text.line_to_char(head_line),
            ),
        };
        Range::new(anchor, head)
    });

    doc.set_selection(view.id, selection);
}

fn extend_to_line_bounds(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    doc.set_selection(
        view.id,
        doc.selection(view.id).clone().transform(|range| {
            let text = doc.text();

            let (start_line, end_line) = range.line_range(text.slice(..));
            let start = text.line_to_char(start_line);
            let end = text.line_to_char((end_line + 1).min(text.len_lines()));

            Range::new(start, end).with_direction(range.direction())
        }),
    );
}

fn shrink_to_line_bounds(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    doc.set_selection(
        view.id,
        doc.selection(view.id).clone().transform(|range| {
            let text = doc.text();

            let (start_line, end_line) = range.line_range(text.slice(..));

            // Do nothing if the selection is within one line to prevent
            // conditional logic for the behavior of this command
            if start_line == end_line {
                return range;
            }

            let mut start = text.line_to_char(start_line);

            // line_to_char gives us the start position of the line, so
            // we need to get the start position of the next line. In
            // the editor, this will correspond to the cursor being on
            // the EOL whitespace character, which is what we want.
            let mut end = text.line_to_char((end_line + 1).min(text.len_lines()));

            if start != range.from() {
                start = text.line_to_char((start_line + 1).min(text.len_lines()));
            }

            if end != range.to() {
                end = text.line_to_char(end_line);
            }

            Range::new(start, end).with_direction(range.direction())
        }),
    );
}

enum Operation {
    Delete,
    Change,
}

fn selection_is_linewise(selection: &Selection, text: &Rope) -> bool {
    selection.ranges().iter().all(|range| {
        let text = text.slice(..);
        if range.slice(text).len_lines() < 2 {
            return false;
        }
        // If the start of the selection is at the start of a line and the end at the end of a line.
        let (start_line, end_line) = range.line_range(text);
        let start = text.line_to_char(start_line);
        let end = text.line_to_char((end_line + 1).min(text.len_lines()));
        start == range.from() && end == range.to()
    })
}

enum YankAction {
    Yank,
    NoYank,
}

fn delete_selection_impl(cx: &mut Context, op: Operation, yank: YankAction) {
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id);
    let only_whole_lines = selection_is_linewise(selection, doc.text());

    if cx.register != Some('_') && matches!(yank, YankAction::Yank) {
        // yank the selection
        let text = doc.text().slice(..);
        let values: Vec<String> = selection.fragments(text).map(Cow::into_owned).collect();
        let reg_name = cx
            .register
            .unwrap_or_else(|| cx.editor.config.load().default_yank_register);
        if let Err(err) = cx.editor.registers.write(reg_name, values) {
            cx.editor.set_error(err.to_string());
            return;
        }
    }

    // delete the selection
    let transaction =
        Transaction::delete_by_selection(doc.text(), selection, |range| (range.from(), range.to()));
    doc.apply(&transaction, view.id);

    match op {
        Operation::Delete => {
            // exit select mode, if currently in select mode
            exit_select_mode(cx);
        }
        Operation::Change => {
            if only_whole_lines {
                open(cx, Open::Above, CommentContinuation::Disabled);
            } else {
                enter_insert_mode(cx);
            }
        }
    }
}

#[inline]
fn delete_by_selection_insert_mode(
    cx: &mut Context,
    mut f: impl FnMut(RopeSlice, &Range) -> Deletion,
    direction: Direction,
) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let mut selection = SmallVec::new();
    let mut insert_newline = false;
    let text_len = text.len_chars();
    let mut transaction =
        Transaction::delete_by_selection(doc.text(), doc.selection(view.id), |range| {
            let (start, end) = f(text, range);
            if direction == Direction::Forward {
                let mut range = *range;
                if range.head > range.anchor {
                    insert_newline |= end == text_len;
                    // move the cursor to the right so that the selection
                    // doesn't shrink when deleting forward (so the text appears to
                    // move to  left)
                    // += 1 is enough here as the range is normalized to grapheme boundaries
                    // later anyway
                    range.head += 1;
                }
                selection.push(range);
            }
            (start, end)
        });

    // in case we delete the last character and the cursor would be moved to the EOF char
    // insert a newline, just like when entering append mode
    if insert_newline {
        transaction = transaction.insert_at_eof(doc.line_ending.as_str().into());
    }

    if direction == Direction::Forward {
        doc.set_selection(
            view.id,
            Selection::new(selection, doc.selection(view.id).primary_index()),
        );
    }
    doc.apply(&transaction, view.id);
}

fn delete_selection(cx: &mut Context) {
    delete_selection_impl(cx, Operation::Delete, YankAction::Yank);
}

fn delete_selection_noyank(cx: &mut Context) {
    delete_selection_impl(cx, Operation::Delete, YankAction::NoYank);
}

fn change_selection(cx: &mut Context) {
    delete_selection_impl(cx, Operation::Change, YankAction::Yank);
}

fn change_selection_noyank(cx: &mut Context) {
    delete_selection_impl(cx, Operation::Change, YankAction::NoYank);
}

fn collapse_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let pos = range.cursor(text);
        Range::new(pos, pos)
    });
    doc.set_selection(view.id, selection);
}

fn flip_selections(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.flip());
    doc.set_selection(view.id, selection);
}

fn ensure_selections_forward(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|r| r.with_direction(Direction::Forward));

    doc.set_selection(view.id, selection);
}

fn enter_insert_mode(cx: &mut Context) {
    cx.editor.mode = Mode::Insert;
}

// inserts at the start of each selection
fn insert_mode(cx: &mut Context) {
    enter_insert_mode(cx);
    let (view, doc) = current!(cx.editor);

    log::trace!(
        "entering insert mode with sel: {:?}, text: {:?}",
        doc.selection(view.id),
        doc.text().to_string()
    );

    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| Range::new(range.to(), range.from()));

    doc.set_selection(view.id, selection);
}

// inserts at the end of each selection
fn append_mode(cx: &mut Context) {
    enter_insert_mode(cx);
    let (view, doc) = current!(cx.editor);
    doc.restore_cursor = true;
    let text = doc.text().slice(..);

    // Make sure there's room at the end of the document if the last
    // selection butts up against it.
    let end = text.len_chars();
    let last_range = doc
        .selection(view.id)
        .iter()
        .last()
        .expect("selection should always have at least one range");
    if !last_range.is_empty() && last_range.to() == end {
        let transaction = Transaction::change(
            doc.text(),
            [(end, end, Some(doc.line_ending.as_str().into()))].into_iter(),
        );
        doc.apply(&transaction, view.id);
    }

    let selection = doc.selection(view.id).clone().transform(|range| {
        Range::new(
            range.from(),
            graphemes::next_grapheme_boundary(doc.text().slice(..), range.to()),
        )
    });
    doc.set_selection(view.id, selection);
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
            cx.editor.set_error("current buffer has no path or parent");
            return;
        }
    };

    let picker = ui::file_picker(cx.editor, path);
    cx.push_layer(Box::new(overlaid(picker)));
}

fn file_picker_in_current_directory(cx: &mut Context) {
    let cwd = helix_stdx::env::current_working_dir();
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
            let cwd = helix_stdx::env::current_working_dir();
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
    let cwd = helix_stdx::env::current_working_dir();
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
                .map(helix_stdx::path::get_relative_path);

            let name = path
                .as_deref()
                .and_then(Path::to_str)
                .unwrap_or(SCRATCH_BUFFER_NAME);
            let icons = ICONS.load();

            let mut spans = Vec::with_capacity(2);

            if let Some(icon) = icons
                .mime()
                .get(path.as_ref().map(|path| path.to_path_buf()).as_ref(), None)
            {
                if let Some(color) = icon.color() {
                    spans.push(Span::styled(
                        format!("{}  ", icon.glyph()),
                        Style::default().fg(color),
                    ));
                } else {
                    spans.push(Span::raw(format!("{}  ", icon.glyph())));
                }
            }

            spans.push(Span::raw(name.to_string()));

            Spans::from(spans).into()
        }),
    ];
    let picker = Picker::new(columns, 2, items, (), |cx, meta, action| {
        cx.editor.switch(meta.id, action);
    })
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
                .map(helix_stdx::path::get_relative_path);

            let name = path
                .as_deref()
                .and_then(Path::to_str)
                .unwrap_or(SCRATCH_BUFFER_NAME);
            let icons = ICONS.load();

            let mut spans = Vec::with_capacity(2);

            if let Some(icon) = icons
                .mime()
                .get(path.as_ref().map(|path| path.to_path_buf()).as_ref(), None)
            {
                if let Some(color) = icon.color() {
                    spans.push(Span::styled(
                        format!("{}  ", icon.glyph()),
                        Style::default().fg(color),
                    ));
                } else {
                    spans.push(Span::raw(format!("{}  ", icon.glyph())));
                }
            }

            spans.push(Span::raw(name.to_string()));

            Spans::from(spans).into()
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

    let cwd = helix_stdx::env::current_working_dir();
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
            let icons = ICONS.load();
            match change {
                FileChange::Untracked { .. } => Span::styled(
                    format!("{}  untracked", icons.vcs().added()),
                    data.style_untracked,
                ),
                FileChange::Modified { .. } => Span::styled(
                    format!("{}  modified", icons.vcs().modified()),
                    data.style_modified,
                ),
                FileChange::Conflict { .. } => Span::styled(
                    format!("{}  conflict", icons.vcs().conflict()),
                    data.style_conflict,
                ),
                FileChange::Deleted { .. } => Span::styled(
                    format!("{}  deleted", icons.vcs().removed()),
                    data.style_deleted,
                ),
                FileChange::Renamed { .. } => Span::styled(
                    format!("{}  renamed", icons.vcs().renamed()),
                    data.style_renamed,
                ),
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
            let keymap = compositor.find::<ui::EditorView>().unwrap().keymaps.map()
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

/// Fallback position to use for [`insert_with_indent`].
enum IndentFallbackPos {
    LineStart,
    LineEnd,
}

// `I` inserts at the first nonwhitespace character of each line with a selection.
// If the line is empty, automatically indent.
fn insert_at_line_start(cx: &mut Context) {
    insert_with_indent(cx, IndentFallbackPos::LineStart);
}

// `A` inserts at the end of each line with a selection.
// If the line is empty, automatically indent.
fn insert_at_line_end(cx: &mut Context) {
    insert_with_indent(cx, IndentFallbackPos::LineEnd);
}

// Enter insert mode and auto-indent the current line if it is empty.
// If the line is not empty, move the cursor to the specified fallback position.
fn insert_with_indent(cx: &mut Context, cursor_fallback: IndentFallbackPos) {
    enter_insert_mode(cx);

    let (view, doc) = current!(cx.editor);
    let loader = cx.editor.syn_loader.load();

    let text = doc.text().slice(..);
    let contents = doc.text();
    let selection = doc.selection(view.id);

    let syntax = doc.syntax();
    let tab_width = doc.tab_width();

    let mut ranges = SmallVec::with_capacity(selection.len());
    let mut offs = 0;

    let mut transaction = Transaction::change_by_selection(contents, selection, |range| {
        let cursor_line = range.cursor_line(text);
        let cursor_line_start = text.line_to_char(cursor_line);

        if line_end_char_index(&text, cursor_line) == cursor_line_start {
            // line is empty => auto indent
            let line_end_index = cursor_line_start;

            let indent = indent::indent_for_newline(
                &loader,
                syntax,
                &doc.config.load().indent_heuristic,
                &doc.indent_style,
                tab_width,
                text,
                cursor_line,
                line_end_index,
                cursor_line,
            );

            // calculate new selection ranges
            let pos = offs + cursor_line_start;
            let indent_width = indent.chars().count();
            ranges.push(Range::point(pos + indent_width));
            offs += indent_width;

            (line_end_index, line_end_index, Some(indent.into()))
        } else {
            // move cursor to the fallback position
            let pos = match cursor_fallback {
                IndentFallbackPos::LineStart => text
                    .line(cursor_line)
                    .first_non_whitespace_char()
                    .map(|ws_offset| ws_offset + cursor_line_start)
                    .unwrap_or(cursor_line_start),
                IndentFallbackPos::LineEnd => line_end_char_index(&text, cursor_line),
            };

            ranges.push(range.put_cursor(text, pos + offs, cx.editor.mode == Mode::Select));

            (cursor_line_start, cursor_line_start, None)
        }
    });

    transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));
    doc.apply(&transaction, view.id);
}

// Creates an LspCallback that waits for formatting changes to be computed. When they're done,
// it applies them, but only if the doc hasn't changed.
//
// TODO: provide some way to cancel this, probably as part of a more general job cancellation
// scheme
async fn make_format_callback(
    doc_id: DocumentId,
    doc_version: i32,
    view_id: ViewId,
    format: impl Future<Output = Result<Transaction, FormatterError>> + Send + 'static,
    write: Option<(Option<PathBuf>, bool)>,
) -> anyhow::Result<job::Callback> {
    let format = format.await;

    let call: job::Callback = Callback::Editor(Box::new(move |editor| {
        if !editor.documents.contains_key(&doc_id) || !editor.tree.contains(view_id) {
            return;
        }

        let scrolloff = editor.config().scrolloff;
        let doc = doc_mut!(editor, &doc_id);
        let view = view_mut!(editor, view_id);

        match format {
            Ok(format) => {
                if doc.version() == doc_version {
                    doc.apply(&format, view.id);
                    doc.append_changes_to_history(view);
                    doc.detect_indent_and_line_ending();
                    view.ensure_cursor_in_view(doc, scrolloff);
                } else {
                    log::info!("discarded formatting changes because the document changed");
                }
            }
            Err(err) => {
                if write.is_none() {
                    editor.set_error(err.to_string());
                    return;
                }
                log::info!("failed to format '{}': {err}", doc.display_name());
            }
        }

        if let Some((path, force)) = write {
            let id = doc.id();
            if let Err(err) = editor.save(id, path, force) {
                editor.set_error(format!("Error saving: {}", err));
            }
        }
    }));

    Ok(call)
}

#[derive(PartialEq, Eq)]
pub enum Open {
    Below,
    Above,
}

#[derive(PartialEq)]
pub enum CommentContinuation {
    Enabled,
    Disabled,
}

fn open(cx: &mut Context, open: Open, comment_continuation: CommentContinuation) {
    let count = cx.count();
    enter_insert_mode(cx);
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);
    let loader = cx.editor.syn_loader.load();

    let text = doc.text().slice(..);
    let contents = doc.text();
    let selection = doc.selection(view.id);
    let mut offs = 0;

    let mut ranges = SmallVec::with_capacity(selection.len());

    let continue_comment_tokens =
        if comment_continuation == CommentContinuation::Enabled && config.continue_comments {
            doc.language_config()
                .and_then(|config| config.comment_tokens.as_ref())
        } else {
            None
        };

    let mut transaction = Transaction::change_by_selection(contents, selection, |range| {
        // the line number, where the cursor is currently
        let curr_line_num = text.char_to_line(match open {
            Open::Below => graphemes::prev_grapheme_boundary(text, range.to()),
            Open::Above => range.from(),
        });

        // the next line number, where the cursor will be, after finishing the transaction
        let next_new_line_num = match open {
            Open::Below => curr_line_num + 1,
            Open::Above => curr_line_num,
        };

        let above_next_new_line_num = next_new_line_num.saturating_sub(1);

        let continue_comment_token = continue_comment_tokens
            .and_then(|tokens| comment::get_comment_token(text, tokens, curr_line_num));

        // Index to insert newlines after, as well as the char width
        // to use to compensate for those inserted newlines.
        let (above_next_line_end_index, above_next_line_end_width) = if next_new_line_num == 0 {
            (0, 0)
        } else {
            (
                line_end_char_index(&text, above_next_new_line_num),
                doc.line_ending.len_chars(),
            )
        };

        let line = text.line(curr_line_num);
        let indent = match line.first_non_whitespace_char() {
            Some(pos) if continue_comment_token.is_some() => line.slice(..pos).to_string(),
            _ => indent::indent_for_newline(
                &loader,
                doc.syntax(),
                &config.indent_heuristic,
                &doc.indent_style,
                doc.tab_width(),
                text,
                above_next_new_line_num,
                above_next_line_end_index,
                curr_line_num,
            ),
        };

        let indent_len = indent.len();
        let mut text = String::with_capacity(1 + indent_len);

        if open == Open::Above && next_new_line_num == 0 {
            text.push_str(&indent);
            if let Some(token) = continue_comment_token {
                text.push_str(token);
                text.push(' ');
            }
            text.push_str(doc.line_ending.as_str());
        } else {
            text.push_str(doc.line_ending.as_str());
            text.push_str(&indent);

            if let Some(token) = continue_comment_token {
                text.push_str(token);
                text.push(' ');
            }
        }

        let text = text.repeat(count);

        // calculate new selection ranges
        let pos = offs + above_next_line_end_index + above_next_line_end_width;
        let comment_len = continue_comment_token
            .map(|token| token.len() + 1) // `+ 1` for the extra space added
            .unwrap_or_default();
        for i in 0..count {
            // pos                     -> beginning of reference line,
            // + (i * (line_ending_len + indent_len + comment_len)) -> beginning of i'th line from pos (possibly including comment token)
            // + indent_len + comment_len ->        -> indent for i'th line
            ranges.push(Range::point(
                pos + (i * (doc.line_ending.len_chars() + indent_len + comment_len))
                    + indent_len
                    + comment_len,
            ));
        }

        // update the offset for the next range
        offs += text.chars().count();

        (
            above_next_line_end_index,
            above_next_line_end_index,
            Some(text.into()),
        )
    });

    transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));

    doc.apply(&transaction, view.id);
}

// o inserts a new line after each line with a selection
fn open_below(cx: &mut Context) {
    open(cx, Open::Below, CommentContinuation::Enabled)
}

// O inserts a new line before each line with a selection
fn open_above(cx: &mut Context) {
    open(cx, Open::Above, CommentContinuation::Enabled)
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

fn goto_line(cx: &mut Context) {
    goto_line_impl(cx, Movement::Move);
}

fn goto_line_impl(cx: &mut Context, movement: Movement) {
    if cx.count.is_some() {
        let (view, doc) = current!(cx.editor);
        push_jump(view, doc);

        goto_line_without_jumplist(cx.editor, cx.count, movement);
    }
}

fn goto_line_without_jumplist(
    editor: &mut Editor,
    count: Option<NonZeroUsize>,
    movement: Movement,
) {
    if let Some(count) = count {
        let (view, doc) = current!(editor);
        let text = doc.text().slice(..);
        let max_line = if text.line(text.len_lines() - 1).len_chars() == 0 {
            // If the last line is blank, don't jump to it.
            text.len_lines().saturating_sub(2)
        } else {
            text.len_lines() - 1
        };
        let line_idx = std::cmp::min(count.get() - 1, max_line);
        let pos = text.line_to_char(line_idx);
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, pos, movement == Movement::Extend));

        doc.set_selection(view.id, selection);
    }
}

fn goto_last_line(cx: &mut Context) {
    goto_last_line_impl(cx, Movement::Move)
}

fn extend_to_last_line(cx: &mut Context) {
    goto_last_line_impl(cx, Movement::Extend)
}

fn goto_last_line_impl(cx: &mut Context, movement: Movement) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let line_idx = if text.line(text.len_lines() - 1).len_chars() == 0 {
        // If the last line is blank, don't jump to it.
        text.len_lines().saturating_sub(2)
    } else {
        text.len_lines() - 1
    };
    let pos = text.line_to_char(line_idx);
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.put_cursor(text, pos, movement == Movement::Extend));

    push_jump(view, doc);
    doc.set_selection(view.id, selection);
}

fn goto_column(cx: &mut Context) {
    goto_column_impl(cx, Movement::Move);
}

fn extend_to_column(cx: &mut Context) {
    goto_column_impl(cx, Movement::Extend);
}

fn goto_column_impl(cx: &mut Context, movement: Movement) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let line_start = text.line_to_char(line);
        let line_end = line_end_char_index(&text, line);
        let pos = graphemes::nth_next_grapheme_boundary(text, line_start, count - 1).min(line_end);
        range.put_cursor(text, pos, movement == Movement::Extend)
    });
    doc.set_selection(view.id, selection);
}

fn goto_last_accessed_file(cx: &mut Context) {
    let view = view_mut!(cx.editor);
    if let Some(alt) = view.docs_access_history.pop() {
        cx.editor.switch(alt, Action::Replace);
    } else {
        cx.editor.set_error("no last accessed buffer")
    }
}

fn goto_last_modification(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let pos = doc.history.get_mut().last_edit_pos();
    let text = doc.text().slice(..);
    if let Some(pos) = pos {
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, pos, cx.editor.mode == Mode::Select));
        doc.set_selection(view.id, selection);
    }
}

fn goto_last_modified_file(cx: &mut Context) {
    let view = view!(cx.editor);
    let alternate_file = view
        .last_modified_docs
        .into_iter()
        .flatten()
        .find(|&id| id != view.doc);
    if let Some(alt) = alternate_file {
        cx.editor.switch(alt, Action::Replace);
    } else {
        cx.editor.set_error("no last modified buffer")
    }
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

fn goto_first_diag(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = match doc.diagnostics().first() {
        Some(diag) => Selection::single(diag.range.start, diag.range.end),
        None => return,
    };
    doc.set_selection(view.id, selection);
    view.diagnostics_handler
        .immediately_show_diagnostic(doc, view.id);
}

fn goto_last_diag(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = match doc.diagnostics().last() {
        Some(diag) => Selection::single(diag.range.start, diag.range.end),
        None => return,
    };
    doc.set_selection(view.id, selection);
    view.diagnostics_handler
        .immediately_show_diagnostic(doc, view.id);
}

fn goto_next_diag(cx: &mut Context) {
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        let cursor_pos = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));

        let diag = doc
            .diagnostics()
            .iter()
            .find(|diag| diag.range.start > cursor_pos);

        let selection = match diag {
            Some(diag) => Selection::single(diag.range.start, diag.range.end),
            None => return,
        };
        doc.set_selection(view.id, selection);
        view.diagnostics_handler
            .immediately_show_diagnostic(doc, view.id);
    };

    cx.editor.apply_motion(motion);
}

fn goto_prev_diag(cx: &mut Context) {
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        let cursor_pos = doc
            .selection(view.id)
            .primary()
            .cursor(doc.text().slice(..));

        let diag = doc
            .diagnostics()
            .iter()
            .rev()
            .find(|diag| diag.range.start < cursor_pos);

        let selection = match diag {
            // NOTE: the selection is reversed because we're jumping to the
            // previous diagnostic.
            Some(diag) => Selection::single(diag.range.end, diag.range.start),
            None => return,
        };
        doc.set_selection(view.id, selection);
        view.diagnostics_handler
            .immediately_show_diagnostic(doc, view.id);
    };
    cx.editor.apply_motion(motion)
}

fn goto_first_change(cx: &mut Context) {
    goto_first_change_impl(cx, false);
}

fn goto_last_change(cx: &mut Context) {
    goto_first_change_impl(cx, true);
}

fn goto_first_change_impl(cx: &mut Context, reverse: bool) {
    let editor = &mut cx.editor;
    let (view, doc) = current!(editor);
    if let Some(handle) = doc.diff_handle() {
        let hunk = {
            let diff = handle.load();
            let idx = if reverse {
                diff.len().saturating_sub(1)
            } else {
                0
            };
            diff.nth_hunk(idx)
        };
        if hunk != Hunk::NONE {
            let range = hunk_range(hunk, doc.text().slice(..));
            doc.set_selection(view.id, Selection::single(range.anchor, range.head));
        }
    }
}

fn goto_next_change(cx: &mut Context) {
    goto_next_change_impl(cx, Direction::Forward)
}

fn goto_prev_change(cx: &mut Context) {
    goto_next_change_impl(cx, Direction::Backward)
}

fn goto_next_change_impl(cx: &mut Context, direction: Direction) {
    let count = cx.count() as u32 - 1;
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);
        let doc_text = doc.text().slice(..);
        let diff_handle = if let Some(diff_handle) = doc.diff_handle() {
            diff_handle
        } else {
            editor.set_status("Diff is not available in current buffer");
            return;
        };

        let selection = doc.selection(view.id).clone().transform(|range| {
            let cursor_line = range.cursor_line(doc_text) as u32;

            let diff = diff_handle.load();
            let hunk_idx = match direction {
                Direction::Forward => diff
                    .next_hunk(cursor_line)
                    .map(|idx| (idx + count).min(diff.len() - 1)),
                Direction::Backward => diff
                    .prev_hunk(cursor_line)
                    .map(|idx| idx.saturating_sub(count)),
            };
            let Some(hunk_idx) = hunk_idx else {
                return range;
            };
            let hunk = diff.nth_hunk(hunk_idx);
            let new_range = hunk_range(hunk, doc_text);
            if editor.mode == Mode::Select {
                let head = if new_range.head < range.anchor {
                    new_range.anchor
                } else {
                    new_range.head
                };

                Range::new(range.anchor, head)
            } else {
                new_range.with_direction(direction)
            }
        });

        doc.set_selection(view.id, selection)
    };
    cx.editor.apply_motion(motion);
}

/// Returns the [Range] for a [Hunk] in the given text.
/// Additions and modifications cover the added and modified ranges.
/// Deletions are represented as the point at the start of the deletion hunk.
fn hunk_range(hunk: Hunk, text: RopeSlice) -> Range {
    let anchor = text.line_to_char(hunk.after.start as usize);
    let head = if hunk.after.is_empty() {
        anchor + 1
    } else {
        text.line_to_char(hunk.after.end as usize)
    };

    Range::new(anchor, head)
}

pub mod insert {
    use crate::events::PostInsertChar;

    use super::*;
    pub type Hook = fn(&Rope, &Selection, char) -> Option<Transaction>;

    /// Exclude the cursor in range.
    fn exclude_cursor(text: RopeSlice, range: Range, cursor: Range) -> Range {
        if range.to() == cursor.to() && text.len_chars() != cursor.to() {
            Range::new(
                range.from(),
                graphemes::prev_grapheme_boundary(text, cursor.to()),
            )
        } else {
            range
        }
    }

    // The default insert hook: simply insert the character
    #[allow(clippy::unnecessary_wraps)] // need to use Option<> because of the Hook signature
    fn insert(doc: &Rope, selection: &Selection, ch: char) -> Option<Transaction> {
        let cursors = selection.clone().cursors(doc.slice(..));
        let mut t = Tendril::new();
        t.push(ch);
        let transaction = Transaction::insert(doc, &cursors, t);
        Some(transaction)
    }

    use helix_core::auto_pairs;
    use helix_view::editor::SmartTabConfig;

    pub fn insert_char(cx: &mut Context, c: char) {
        let (view, doc) = current_ref!(cx.editor);
        let text = doc.text();
        let selection = doc.selection(view.id);
        let auto_pairs = doc.auto_pairs(cx.editor);

        let transaction = auto_pairs
            .as_ref()
            .and_then(|ap| auto_pairs::hook(text, selection, c, ap))
            .or_else(|| insert(text, selection, c));

        let (view, doc) = current!(cx.editor);
        if let Some(t) = transaction {
            doc.apply(&t, view.id);
        }

        helix_event::dispatch(PostInsertChar { c, cx });
    }

    pub fn smart_tab(cx: &mut Context) {
        let (view, doc) = current_ref!(cx.editor);
        let view_id = view.id;

        if matches!(
            cx.editor.config().smart_tab,
            Some(SmartTabConfig { enable: true, .. })
        ) {
            let cursors_after_whitespace = doc.selection(view_id).ranges().iter().all(|range| {
                let cursor = range.cursor(doc.text().slice(..));
                let current_line_num = doc.text().char_to_line(cursor);
                let current_line_start = doc.text().line_to_char(current_line_num);
                let left = doc.text().slice(current_line_start..cursor);
                left.chars().all(|c| c.is_whitespace())
            });

            if !cursors_after_whitespace {
                if doc.active_snippet.is_some() {
                    goto_next_tabstop(cx);
                } else {
                    move_parent_node_end(cx);
                }
                return;
            }
        }

        insert_tab(cx);
    }

    pub fn insert_tab(cx: &mut Context) {
        let (view, doc) = current!(cx.editor);
        // TODO: round out to nearest indentation level (for example a line with 3 spaces should
        // indent by one to reach 4 spaces).

        let indent = Tendril::from(doc.indent_style.as_str());
        let transaction = Transaction::insert(
            doc.text(),
            &doc.selection(view.id).clone().cursors(doc.text().slice(..)),
            indent,
        );
        doc.apply(&transaction, view.id);
    }

    pub fn insert_newline(cx: &mut Context) {
        let config = cx.editor.config();
        let (view, doc) = current_ref!(cx.editor);
        let loader = cx.editor.syn_loader.load();
        let text = doc.text().slice(..);
        let line_ending = doc.line_ending.as_str();

        let contents = doc.text();
        let selection = doc.selection(view.id);
        let mut ranges = SmallVec::with_capacity(selection.len());

        // TODO: this is annoying, but we need to do it to properly calculate pos after edits
        let mut global_offs = 0;
        let mut new_text = String::new();

        let continue_comment_tokens = if config.continue_comments {
            doc.language_config()
                .and_then(|config| config.comment_tokens.as_ref())
        } else {
            None
        };

        let mut last_pos = 0;
        let mut transaction = Transaction::change_by_selection(contents, selection, |range| {
            // Tracks the number of trailing whitespace characters deleted by this selection.
            let mut chars_deleted = 0;
            let pos = range.cursor(text);

            let prev = if pos == 0 {
                ' '
            } else {
                contents.char(pos - 1)
            };
            let curr = contents.get_char(pos).unwrap_or(' ');

            let current_line = text.char_to_line(pos);
            let line_start = text.line_to_char(current_line);

            let continue_comment_token = continue_comment_tokens
                .and_then(|tokens| comment::get_comment_token(text, tokens, current_line));

            let (from, to, local_offs) = if let Some(idx) =
                text.slice(line_start..pos).last_non_whitespace_char()
            {
                let first_trailing_whitespace_char = (line_start + idx + 1).clamp(last_pos, pos);
                last_pos = pos;
                let line = text.line(current_line);

                let indent = match line.first_non_whitespace_char() {
                    Some(pos) if continue_comment_token.is_some() => line.slice(..pos).to_string(),
                    _ => indent::indent_for_newline(
                        &loader,
                        doc.syntax(),
                        &config.indent_heuristic,
                        &doc.indent_style,
                        doc.tab_width(),
                        text,
                        current_line,
                        pos,
                        current_line,
                    ),
                };

                // If we are between pairs (such as brackets), we want to
                // insert an additional line which is indented one level
                // more and place the cursor there
                let on_auto_pair = doc
                    .auto_pairs(cx.editor)
                    .and_then(|pairs| pairs.get(prev))
                    .is_some_and(|pair| pair.open == prev && pair.close == curr);

                let local_offs = if let Some(token) = continue_comment_token {
                    new_text.reserve_exact(line_ending.len() + indent.len() + token.len() + 1);
                    new_text.push_str(line_ending);
                    new_text.push_str(&indent);
                    new_text.push_str(token);
                    new_text.push(' ');
                    new_text.chars().count()
                } else if on_auto_pair {
                    // line where the cursor will be
                    let inner_indent = indent.clone() + doc.indent_style.as_str();
                    new_text
                        .reserve_exact(line_ending.len() * 2 + indent.len() + inner_indent.len());
                    new_text.push_str(line_ending);
                    new_text.push_str(&inner_indent);

                    // line where the matching pair will be
                    let local_offs = new_text.chars().count();
                    new_text.push_str(line_ending);
                    new_text.push_str(&indent);

                    local_offs
                } else {
                    new_text.reserve_exact(line_ending.len() + indent.len());
                    new_text.push_str(line_ending);
                    new_text.push_str(&indent);

                    new_text.chars().count()
                };

                // Note that `first_trailing_whitespace_char` is at least `pos` so this unsigned
                // subtraction cannot underflow.
                chars_deleted = pos - first_trailing_whitespace_char;

                (
                    first_trailing_whitespace_char,
                    pos,
                    local_offs as isize - chars_deleted as isize,
                )
            } else {
                // If the current line is all whitespace, insert a line ending at the beginning of
                // the current line. This makes the current line empty and the new line contain the
                // indentation of the old line.
                new_text.push_str(line_ending);

                (line_start, line_start, new_text.chars().count() as isize)
            };

            let new_range = if range.cursor(text) > range.anchor {
                // when appending, extend the range by local_offs
                Range::new(
                    (range.anchor as isize + global_offs) as usize,
                    (range.head as isize + local_offs + global_offs) as usize,
                )
            } else {
                // when inserting, slide the range by local_offs
                Range::new(
                    (range.anchor as isize + local_offs + global_offs) as usize,
                    (range.head as isize + local_offs + global_offs) as usize,
                )
            };

            // TODO: range replace or extend
            // range.replace(|range| range.is_empty(), head); -> fn extend if cond true, new head pos
            // can be used with cx.mode to do replace or extend on most changes
            ranges.push(new_range);
            global_offs += new_text.chars().count() as isize - chars_deleted as isize;
            let tendril = Tendril::from(&new_text);
            new_text.clear();

            (from, to, Some(tendril))
        });

        transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));

        let (view, doc) = current!(cx.editor);
        doc.apply(&transaction, view.id);
    }

    pub fn delete_char_backward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current_ref!(cx.editor);
        let text = doc.text().slice(..);
        let tab_width = doc.tab_width();
        let indent_width = doc.indent_width();
        let auto_pairs = doc.auto_pairs(cx.editor);

        let transaction =
            Transaction::delete_by_selection(doc.text(), doc.selection(view.id), |range| {
                let pos = range.cursor(text);
                if pos == 0 {
                    return (pos, pos);
                }
                let line_start_pos = text.line_to_char(range.cursor_line(text));
                // consider to delete by indent level if all characters before `pos` are indent units.
                let fragment = Cow::from(text.slice(line_start_pos..pos));
                if !fragment.is_empty() && fragment.chars().all(|ch| ch == ' ' || ch == '\t') {
                    if text.get_char(pos.saturating_sub(1)) == Some('\t') {
                        // fast path, delete one char
                        (graphemes::nth_prev_grapheme_boundary(text, pos, 1), pos)
                    } else {
                        let width: usize = fragment
                            .chars()
                            .map(|ch| {
                                if ch == '\t' {
                                    tab_width
                                } else {
                                    // it can be none if it still meet control characters other than '\t'
                                    // here just set the width to 1 (or some value better?).
                                    ch.width().unwrap_or(1)
                                }
                            })
                            .sum();
                        let mut drop = width % indent_width; // round down to nearest unit
                        if drop == 0 {
                            drop = indent_width
                        }; // if it's already at a unit, consume a whole unit
                        let mut chars = fragment.chars().rev();
                        let mut start = pos;
                        for _ in 0..drop {
                            // delete up to `drop` spaces
                            match chars.next() {
                                Some(' ') => start -= 1,
                                _ => break,
                            }
                        }
                        (start, pos) // delete!
                    }
                } else {
                    match (
                        text.get_char(pos.saturating_sub(1)),
                        text.get_char(pos),
                        auto_pairs,
                    ) {
                        (Some(_x), Some(_y), Some(ap))
                            if range.is_single_grapheme(text)
                                && ap.get(_x).is_some()
                                && ap.get(_x).unwrap().open == _x
                                && ap.get(_x).unwrap().close == _y =>
                        // delete both autopaired characters
                        {
                            (
                                graphemes::nth_prev_grapheme_boundary(text, pos, count),
                                graphemes::nth_next_grapheme_boundary(text, pos, count),
                            )
                        }
                        _ =>
                        // delete 1 char
                        {
                            (graphemes::nth_prev_grapheme_boundary(text, pos, count), pos)
                        }
                    }
                }
            });
        let (view, doc) = current!(cx.editor);
        doc.apply(&transaction, view.id);
    }

    pub fn delete_char_forward(cx: &mut Context) {
        let count = cx.count();
        delete_by_selection_insert_mode(
            cx,
            |text, range| {
                let pos = range.cursor(text);
                (pos, graphemes::nth_next_grapheme_boundary(text, pos, count))
            },
            Direction::Forward,
        )
    }

    pub fn delete_word_backward(cx: &mut Context) {
        let count = cx.count();
        delete_by_selection_insert_mode(
            cx,
            |text, range| {
                let anchor = movement::move_prev_word_start(text, *range, count).from();
                let next = Range::new(anchor, range.cursor(text));
                let range = exclude_cursor(text, next, *range);
                (range.from(), range.to())
            },
            Direction::Backward,
        );
    }

    pub fn delete_word_forward(cx: &mut Context) {
        let count = cx.count();
        delete_by_selection_insert_mode(
            cx,
            |text, range| {
                let head = movement::move_next_word_end(text, *range, count).to();
                (range.cursor(text), head)
            },
            Direction::Forward,
        );
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

// Yank / Paste

fn yank(cx: &mut Context) {
    yank_impl(
        cx.editor,
        cx.register
            .unwrap_or(cx.editor.config().default_yank_register),
    );
    exit_select_mode(cx);
}

fn yank_to_clipboard(cx: &mut Context) {
    yank_impl(cx.editor, '+');
    exit_select_mode(cx);
}

fn yank_to_primary_clipboard(cx: &mut Context) {
    yank_impl(cx.editor, '*');
    exit_select_mode(cx);
}

fn yank_impl(editor: &mut Editor, register: char) {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(text)
        .map(Cow::into_owned)
        .collect();
    let selections = values.len();

    match editor.registers.write(register, values) {
        Ok(_) => editor.set_status(format!(
            "yanked {selections} selection{} to register {register}",
            if selections == 1 { "" } else { "s" }
        )),
        Err(err) => editor.set_error(err.to_string()),
    }
}

fn yank_joined_impl(editor: &mut Editor, separator: &str, register: char) {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id);
    let selections = selection.len();
    let joined = selection
        .fragments(text)
        .fold(String::new(), |mut acc, fragment| {
            if !acc.is_empty() {
                acc.push_str(separator);
            }
            acc.push_str(&fragment);
            acc
        });

    match editor.registers.write(register, vec![joined]) {
        Ok(_) => editor.set_status(format!(
            "joined and yanked {selections} selection{} to register {register}",
            if selections == 1 { "" } else { "s" }
        )),
        Err(err) => editor.set_error(err.to_string()),
    }
}

fn yank_joined(cx: &mut Context) {
    let separator = doc!(cx.editor).line_ending.as_str();
    yank_joined_impl(
        cx.editor,
        separator,
        cx.register
            .unwrap_or(cx.editor.config().default_yank_register),
    );
    exit_select_mode(cx);
}

fn yank_joined_to_clipboard(cx: &mut Context) {
    let line_ending = doc!(cx.editor).line_ending;
    yank_joined_impl(cx.editor, line_ending.as_str(), '+');
    exit_select_mode(cx);
}

fn yank_joined_to_primary_clipboard(cx: &mut Context) {
    let line_ending = doc!(cx.editor).line_ending;
    yank_joined_impl(cx.editor, line_ending.as_str(), '*');
    exit_select_mode(cx);
}

fn yank_primary_selection_impl(editor: &mut Editor, register: char) {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).primary().fragment(text).to_string();

    match editor.registers.write(register, vec![selection]) {
        Ok(_) => editor.set_status(format!("yanked primary selection to register {register}",)),
        Err(err) => editor.set_error(err.to_string()),
    }
}

fn yank_main_selection_to_clipboard(cx: &mut Context) {
    yank_primary_selection_impl(cx.editor, '+');
    exit_select_mode(cx);
}

fn yank_main_selection_to_primary_clipboard(cx: &mut Context) {
    yank_primary_selection_impl(cx.editor, '*');
    exit_select_mode(cx);
}

#[derive(Copy, Clone)]
enum Paste {
    Before,
    After,
    Cursor,
}

static LINE_ENDING_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\r\n|\r|\n").unwrap());

fn paste_impl(
    values: &[String],
    doc: &mut Document,
    view: &mut View,
    action: Paste,
    count: usize,
    mode: Mode,
) {
    if values.is_empty() {
        return;
    }

    if mode == Mode::Insert {
        doc.append_changes_to_history(view);
    }

    // if any of values ends with a line ending, it's linewise paste
    let linewise = values
        .iter()
        .any(|value| get_line_ending_of_str(value).is_some());

    let map_value = |value| {
        let value = LINE_ENDING_REGEX.replace_all(value, doc.line_ending.as_str());
        let mut out = Tendril::from(value.as_ref());
        for _ in 1..count {
            out.push_str(&value);
        }
        out
    };

    let repeat = std::iter::repeat(
        // `values` is asserted to have at least one entry above.
        map_value(values.last().unwrap()),
    );

    let mut values = values.iter().map(|value| map_value(value)).chain(repeat);

    let text = doc.text();
    let selection = doc.selection(view.id);

    let mut offset = 0;
    let mut ranges = SmallVec::with_capacity(selection.len());

    let mut transaction = Transaction::change_by_selection(text, selection, |range| {
        let pos = match (action, linewise) {
            // paste linewise before
            (Paste::Before, true) => text.line_to_char(text.char_to_line(range.from())),
            // paste linewise after
            (Paste::After, true) => {
                let line = range.line_range(text.slice(..)).1;
                text.line_to_char((line + 1).min(text.len_lines()))
            }
            // paste insert
            (Paste::Before, false) => range.from(),
            // paste append
            (Paste::After, false) => range.to(),
            // paste at cursor
            (Paste::Cursor, _) => range.cursor(text.slice(..)),
        };

        let value = values.next();

        let value_len = value
            .as_ref()
            .map(|content| content.chars().count())
            .unwrap_or_default();
        let anchor = offset + pos;

        let new_range = Range::new(anchor, anchor + value_len).with_direction(range.direction());
        ranges.push(new_range);
        offset += value_len;

        (pos, pos, value)
    });

    if mode == Mode::Normal {
        transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));
    }

    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view);
}

pub(crate) fn paste_bracketed_value(cx: &mut Context, contents: String) {
    let count = cx.count();
    let paste = match cx.editor.mode {
        Mode::Insert | Mode::Select => Paste::Cursor,
        Mode::Normal => Paste::Before,
    };
    let (view, doc) = current!(cx.editor);
    paste_impl(&[contents], doc, view, paste, count, cx.editor.mode);
    exit_select_mode(cx);
}

fn paste_clipboard_after(cx: &mut Context) {
    paste(cx.editor, '+', Paste::After, cx.count());
    exit_select_mode(cx);
}

fn paste_clipboard_before(cx: &mut Context) {
    paste(cx.editor, '+', Paste::Before, cx.count());
    exit_select_mode(cx);
}

fn paste_primary_clipboard_after(cx: &mut Context) {
    paste(cx.editor, '*', Paste::After, cx.count());
    exit_select_mode(cx);
}

fn paste_primary_clipboard_before(cx: &mut Context) {
    paste(cx.editor, '*', Paste::Before, cx.count());
    exit_select_mode(cx);
}

fn replace_with_yanked(cx: &mut Context) {
    replace_with_yanked_impl(
        cx.editor,
        cx.register
            .unwrap_or(cx.editor.config().default_yank_register),
        cx.count(),
    );
    exit_select_mode(cx);
}

fn replace_with_yanked_impl(editor: &mut Editor, register: char, count: usize) {
    let Some(values) = editor
        .registers
        .read(register, editor)
        .filter(|values| values.len() > 0)
    else {
        return;
    };
    let scrolloff = editor.config().scrolloff;
    let (view, doc) = current_ref!(editor);

    let map_value = |value: &Cow<str>| {
        let value = LINE_ENDING_REGEX.replace_all(value, doc.line_ending.as_str());
        let mut out = Tendril::from(value.as_ref());
        for _ in 1..count {
            out.push_str(&value);
        }
        out
    };
    let mut values_rev = values.rev().peekable();
    // `values` is asserted to have at least one entry above.
    let last = values_rev.peek().unwrap();
    let repeat = std::iter::repeat(map_value(last));
    let mut values = values_rev
        .rev()
        .map(|value| map_value(&value))
        .chain(repeat);
    let selection = doc.selection(view.id);
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        if !range.is_empty() {
            (range.from(), range.to(), Some(values.next().unwrap()))
        } else {
            (range.from(), range.to(), None)
        }
    });
    drop(values);

    let (view, doc) = current!(editor);
    doc.apply(&transaction, view.id);
    doc.append_changes_to_history(view);
    view.ensure_cursor_in_view(doc, scrolloff);
}

fn replace_selections_with_clipboard(cx: &mut Context) {
    replace_with_yanked_impl(cx.editor, '+', cx.count());
    exit_select_mode(cx);
}

fn replace_selections_with_primary_clipboard(cx: &mut Context) {
    replace_with_yanked_impl(cx.editor, '*', cx.count());
    exit_select_mode(cx);
}

fn paste(editor: &mut Editor, register: char, pos: Paste, count: usize) {
    let Some(values) = editor.registers.read(register, editor) else {
        return;
    };
    let values: Vec<_> = values.map(|value| value.to_string()).collect();

    let (view, doc) = current!(editor);
    paste_impl(&values, doc, view, pos, count, editor.mode);
}

fn paste_after(cx: &mut Context) {
    paste(
        cx.editor,
        cx.register
            .unwrap_or(cx.editor.config().default_yank_register),
        Paste::After,
        cx.count(),
    );
    exit_select_mode(cx);
}

fn paste_before(cx: &mut Context) {
    paste(
        cx.editor,
        cx.register
            .unwrap_or(cx.editor.config().default_yank_register),
        Paste::Before,
        cx.count(),
    );
    exit_select_mode(cx);
}

fn get_lines(doc: &Document, view_id: ViewId) -> Vec<usize> {
    let mut lines = Vec::new();

    // Get all line numbers
    for range in doc.selection(view_id) {
        let (start, end) = range.line_range(doc.text().slice(..));

        for line in start..=end {
            lines.push(line)
        }
    }
    lines.sort_unstable(); // sorting by usize so _unstable is preferred
    lines.dedup();
    lines
}

fn indent(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let lines = get_lines(doc, view.id);

    // Indent by one level
    let indent = Tendril::from(doc.indent_style.as_str().repeat(count));

    let transaction = Transaction::change(
        doc.text(),
        lines.into_iter().filter_map(|line| {
            let is_blank = doc.text().line(line).chunks().all(|s| s.trim().is_empty());
            if is_blank {
                return None;
            }
            let pos = doc.text().line_to_char(line);
            Some((pos, pos, Some(indent.clone())))
        }),
    );
    doc.apply(&transaction, view.id);
    exit_select_mode(cx);
}

fn unindent(cx: &mut Context) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let lines = get_lines(doc, view.id);
    let mut changes = Vec::with_capacity(lines.len());
    let tab_width = doc.tab_width();
    let indent_width = count * doc.indent_width();

    for line_idx in lines {
        let line = doc.text().line(line_idx);
        let mut width = 0;
        let mut pos = 0;

        for ch in line.chars() {
            match ch {
                ' ' => width += 1,
                '\t' => width = (width / tab_width + 1) * tab_width,
                _ => break,
            }

            pos += 1;

            if width >= indent_width {
                break;
            }
        }

        // now delete from start to first non-blank
        if pos > 0 {
            let start = doc.text().line_to_char(line_idx);
            changes.push((start, start + pos, None))
        }
    }

    let transaction = Transaction::change(doc.text(), changes.into_iter());

    doc.apply(&transaction, view.id);
    exit_select_mode(cx);
}

fn format_selections(cx: &mut Context) {
    use helix_lsp::{lsp, util::range_to_lsp_range};

    let (view, doc) = current!(cx.editor);
    let view_id = view.id;

    // via lsp if available
    // TODO: else via tree-sitter indentation calculations

    if doc.selection(view_id).len() != 1 {
        cx.editor
            .set_error("format_selections only supports a single selection for now");
        return;
    }

    // TODO extra LanguageServerFeature::FormatSelections?
    // maybe such that LanguageServerFeature::Format contains it as well
    let Some(language_server) = doc
        .language_servers_with_feature(LanguageServerFeature::Format)
        .find(|ls| {
            matches!(
                ls.capabilities().document_range_formatting_provider,
                Some(lsp::OneOf::Left(true) | lsp::OneOf::Right(_))
            )
        })
    else {
        cx.editor
            .set_error("No configured language server supports range formatting");
        return;
    };

    let offset_encoding = language_server.offset_encoding();
    let ranges: Vec<lsp::Range> = doc
        .selection(view_id)
        .iter()
        .map(|range| range_to_lsp_range(doc.text(), *range, offset_encoding))
        .collect();

    // TODO: handle fails
    // TODO: concurrent map over all ranges

    let range = ranges[0];

    let future = language_server
        .text_document_range_formatting(
            doc.identifier(),
            range,
            lsp::FormattingOptions {
                tab_size: doc.tab_width() as u32,
                insert_spaces: matches!(doc.indent_style, IndentStyle::Spaces(_)),
                ..Default::default()
            },
            None,
        )
        .unwrap();

    let edits = tokio::task::block_in_place(|| helix_lsp::block_on(future))
        .ok()
        .flatten()
        .unwrap_or_default();

    let transaction =
        helix_lsp::util::generate_transaction_from_edits(doc.text(), edits, offset_encoding);

    doc.apply(&transaction, view_id);
}

fn join_selections_impl(cx: &mut Context, select_space: bool) {
    use movement::skip_while;
    let (view, doc) = current!(cx.editor);
    let text = doc.text();
    let slice = text.slice(..);

    let comment_tokens = doc
        .language_config()
        .and_then(|config| config.comment_tokens.as_deref())
        .unwrap_or(&[]);
    // Sort by length to handle Rust's /// vs //
    let mut comment_tokens: Vec<&str> = comment_tokens.iter().map(|x| x.as_str()).collect();
    comment_tokens.sort_unstable_by_key(|x| std::cmp::Reverse(x.len()));

    let mut changes = Vec::new();

    for selection in doc.selection(view.id) {
        let (start, mut end) = selection.line_range(slice);
        if start == end {
            end = (end + 1).min(text.len_lines() - 1);
        }
        let lines = start..end;

        changes.reserve(lines.len());

        let first_line_idx = slice.line_to_char(start);
        let first_line_idx = skip_while(slice, first_line_idx, |ch| matches!(ch, ' ' | '\t'))
            .unwrap_or(first_line_idx);
        let first_line = slice.slice(first_line_idx..);
        let mut current_comment_token = comment_tokens
            .iter()
            .find(|token| first_line.starts_with(token));

        for line in lines {
            let start = line_end_char_index(&slice, line);
            let mut end = text.line_to_char(line + 1);
            end = skip_while(slice, end, |ch| matches!(ch, ' ' | '\t')).unwrap_or(end);
            let slice_from_end = slice.slice(end..);
            if let Some(token) = comment_tokens
                .iter()
                .find(|token| slice_from_end.starts_with(token))
            {
                if Some(token) == current_comment_token {
                    end += token.chars().count();
                    end = skip_while(slice, end, |ch| matches!(ch, ' ' | '\t')).unwrap_or(end);
                } else {
                    // update current token, but don't delete this one.
                    current_comment_token = Some(token);
                }
            }

            let separator = if end == line_end_char_index(&slice, line + 1) {
                // the joining line contains only space-characters => don't include a whitespace when joining
                None
            } else {
                Some(Tendril::from(" "))
            };
            changes.push((start, end, separator));
        }
    }

    // nothing to do, bail out early to avoid crashes later
    if changes.is_empty() {
        return;
    }

    changes.sort_unstable_by_key(|(from, _to, _text)| *from);
    changes.dedup();

    // select inserted spaces
    let transaction = if select_space {
        let mut offset: usize = 0;
        let ranges: SmallVec<_> = changes
            .iter()
            .filter_map(|change| {
                if change.2.is_some() {
                    let range = Range::point(change.0 - offset);
                    offset += change.1 - change.0 - 1; // -1 adjusts for the replacement of the range by a space
                    Some(range)
                } else {
                    offset += change.1 - change.0;
                    None
                }
            })
            .collect();
        let t = Transaction::change(text, changes.into_iter());
        if ranges.is_empty() {
            t
        } else {
            let selection = Selection::new(ranges, 0);
            t.with_selection(selection)
        }
    } else {
        Transaction::change(text, changes.into_iter())
    };

    doc.apply(&transaction, view.id);
}

fn keep_or_remove_selections_impl(cx: &mut Context, remove: bool) {
    // keep or remove selections matching regex
    let reg = cx.register.unwrap_or('/');
    ui::regex_prompt(
        cx,
        if remove { "remove:" } else { "keep:" }.into(),
        Some(reg),
        ui::completers::none,
        move |cx, regex, event| {
            let (view, doc) = current!(cx.editor);
            if !matches!(event, PromptEvent::Update | PromptEvent::Validate) {
                return;
            }
            let text = doc.text().slice(..);

            if let Some(selection) =
                selection::keep_or_remove_matches(text, doc.selection(view.id), &regex, remove)
            {
                doc.set_selection(view.id, selection);
            } else {
                cx.editor.set_error("no selections remaining");
            }
        },
    )
}

fn join_selections(cx: &mut Context) {
    join_selections_impl(cx, false)
}

fn join_selections_space(cx: &mut Context) {
    join_selections_impl(cx, true)
}

fn keep_selections(cx: &mut Context) {
    keep_or_remove_selections_impl(cx, false)
}

fn remove_selections(cx: &mut Context) {
    keep_or_remove_selections_impl(cx, true)
}

fn keep_primary_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    // TODO: handle count

    let range = doc.selection(view.id).primary();
    doc.set_selection(view.id, Selection::single(range.anchor, range.head));
}

fn remove_primary_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    // TODO: handle count

    let selection = doc.selection(view.id);
    if selection.len() == 1 {
        cx.editor.set_error("no selections remaining");
        return;
    }
    let index = selection.primary_index();
    let selection = selection.clone().remove(index);

    doc.set_selection(view.id, selection);
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
type CommentTransactionFn = fn(
    line_token: Option<&str>,
    block_tokens: Option<&[BlockCommentToken]>,
    doc: &Rope,
    selection: &Selection,
) -> Transaction;

fn toggle_comments_impl(cx: &mut Context, comment_transaction: CommentTransactionFn) {
    let (view, doc) = current!(cx.editor);
    let line_token: Option<&str> = doc
        .language_config()
        .and_then(|lc| lc.comment_tokens.as_ref())
        .and_then(|tc| tc.first())
        .map(|tc| tc.as_str());
    let block_tokens: Option<&[BlockCommentToken]> = doc
        .language_config()
        .and_then(|lc| lc.block_comment_tokens.as_ref())
        .map(|tc| &tc[..]);

    let transaction =
        comment_transaction(line_token, block_tokens, doc.text(), doc.selection(view.id));

    doc.apply(&transaction, view.id);
    exit_select_mode(cx);
}

/// commenting behavior:
/// 1. only line comment tokens -> line comment
/// 2. each line block commented -> uncomment all lines
/// 3. whole selection block commented -> uncomment selection
/// 4. all lines not commented and block tokens -> comment uncommented lines
/// 5. no comment tokens and not block commented -> line comment
fn toggle_comments(cx: &mut Context) {
    toggle_comments_impl(cx, |line_token, block_tokens, doc, selection| {
        let text = doc.slice(..);

        // only have line comment tokens
        if line_token.is_some() && block_tokens.is_none() {
            return comment::toggle_line_comments(doc, selection, line_token);
        }

        let split_lines = comment::split_lines_of_selection(text, selection);

        let default_block_tokens = &[BlockCommentToken::default()];
        let block_comment_tokens = block_tokens.unwrap_or(default_block_tokens);

        let (line_commented, line_comment_changes) =
            comment::find_block_comments(block_comment_tokens, text, &split_lines);

        // block commented by line would also be block commented so check this first
        if line_commented {
            return comment::create_block_comment_transaction(
                doc,
                &split_lines,
                line_commented,
                line_comment_changes,
            )
            .0;
        }

        let (block_commented, comment_changes) =
            comment::find_block_comments(block_comment_tokens, text, selection);

        // check if selection has block comments
        if block_commented {
            return comment::create_block_comment_transaction(
                doc,
                selection,
                block_commented,
                comment_changes,
            )
            .0;
        }

        // not commented and only have block comment tokens
        if line_token.is_none() && block_tokens.is_some() {
            return comment::create_block_comment_transaction(
                doc,
                &split_lines,
                line_commented,
                line_comment_changes,
            )
            .0;
        }

        // not block commented at all and don't have any tokens
        comment::toggle_line_comments(doc, selection, line_token)
    })
}

fn toggle_line_comments(cx: &mut Context) {
    toggle_comments_impl(cx, |line_token, block_tokens, doc, selection| {
        if line_token.is_none() && block_tokens.is_some() {
            let default_block_tokens = &[BlockCommentToken::default()];
            let block_comment_tokens = block_tokens.unwrap_or(default_block_tokens);
            comment::toggle_block_comments(
                doc,
                &comment::split_lines_of_selection(doc.slice(..), selection),
                block_comment_tokens,
            )
        } else {
            comment::toggle_line_comments(doc, selection, line_token)
        }
    });
}

fn toggle_block_comments(cx: &mut Context) {
    toggle_comments_impl(cx, |line_token, block_tokens, doc, selection| {
        if line_token.is_some() && block_tokens.is_none() {
            comment::toggle_line_comments(doc, selection, line_token)
        } else {
            let default_block_tokens = &[BlockCommentToken::default()];
            let block_comment_tokens = block_tokens.unwrap_or(default_block_tokens);
            comment::toggle_block_comments(doc, selection, block_comment_tokens)
        }
    });
}

fn rotate_selections(cx: &mut Context, direction: Direction) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let mut selection = doc.selection(view.id).clone();
    let index = selection.primary_index();
    let len = selection.len();
    selection.set_primary_index(match direction {
        Direction::Forward => (index + count) % len,
        Direction::Backward => (index + (len.saturating_sub(count) % len)) % len,
    });
    doc.set_selection(view.id, selection);
}
fn rotate_selections_forward(cx: &mut Context) {
    rotate_selections(cx, Direction::Forward)
}
fn rotate_selections_backward(cx: &mut Context) {
    rotate_selections(cx, Direction::Backward)
}

fn rotate_selections_first(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let mut selection = doc.selection(view.id).clone();
    selection.set_primary_index(0);
    doc.set_selection(view.id, selection);
}

fn rotate_selections_last(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let mut selection = doc.selection(view.id).clone();
    let len = selection.len();
    selection.set_primary_index(len - 1);
    doc.set_selection(view.id, selection);
}

enum ReorderStrategy {
    RotateForward,
    RotateBackward,
    Reverse,
}

fn reorder_selection_contents(cx: &mut Context, strategy: ReorderStrategy) {
    let count = cx.count;
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id);
    let mut fragments: Vec<_> = selection
        .slices(text)
        .map(|fragment| fragment.chunks().collect())
        .collect();

    let group = count
        .map(|count| count.get())
        .unwrap_or(fragments.len()) // default to rotating everything as one group
        .min(fragments.len());

    for chunk in fragments.chunks_mut(group) {
        // TODO: also modify main index
        match strategy {
            ReorderStrategy::RotateForward => chunk.rotate_right(1),
            ReorderStrategy::RotateBackward => chunk.rotate_left(1),
            ReorderStrategy::Reverse => chunk.reverse(),
        };
    }

    let transaction = Transaction::change(
        doc.text(),
        selection
            .ranges()
            .iter()
            .zip(fragments)
            .map(|(range, fragment)| (range.from(), range.to(), Some(fragment))),
    );

    doc.apply(&transaction, view.id);
}

fn rotate_selection_contents_forward(cx: &mut Context) {
    reorder_selection_contents(cx, ReorderStrategy::RotateForward)
}
fn rotate_selection_contents_backward(cx: &mut Context) {
    reorder_selection_contents(cx, ReorderStrategy::RotateBackward)
}
fn reverse_selection_contents(cx: &mut Context) {
    reorder_selection_contents(cx, ReorderStrategy::Reverse)
}

// tree sitter node selection

fn expand_selection(cx: &mut Context) {
    let motion = |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);

            let current_selection = doc.selection(view.id);
            let selection = object::expand_selection(syntax, text, current_selection.clone());

            // check if selection is different from the last one
            if *current_selection != selection {
                // save current selection so it can be restored using shrink_selection
                view.object_selections.push(current_selection.clone());

                doc.set_selection(view.id, selection);
            }
        }
    };
    cx.editor.apply_motion(motion);
}

fn shrink_selection(cx: &mut Context) {
    let motion = |editor: &mut Editor| {
        let (view, doc) = current!(editor);
        let current_selection = doc.selection(view.id);
        // try to restore previous selection
        if let Some(prev_selection) = view.object_selections.pop() {
            if current_selection.contains(&prev_selection) {
                doc.set_selection(view.id, prev_selection);
                return;
            } else {
                // clear existing selection as they can't be shrunk to anyway
                view.object_selections.clear();
            }
        }
        // if not previous selection, shrink to first child
        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            let selection = object::shrink_selection(syntax, text, current_selection.clone());
            doc.set_selection(view.id, selection);
        }
    };
    cx.editor.apply_motion(motion);
}

fn select_sibling_impl<F>(cx: &mut Context, sibling_fn: F)
where
    F: Fn(&helix_core::Syntax, RopeSlice, Selection) -> Selection + 'static,
{
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            let current_selection = doc.selection(view.id);
            let selection = sibling_fn(syntax, text, current_selection.clone());
            doc.set_selection(view.id, selection);
        }
    };
    cx.editor.apply_motion(motion);
}

fn select_next_sibling(cx: &mut Context) {
    select_sibling_impl(cx, object::select_next_sibling)
}

fn select_prev_sibling(cx: &mut Context) {
    select_sibling_impl(cx, object::select_prev_sibling)
}

fn move_node_bound_impl(cx: &mut Context, dir: Direction, movement: Movement) {
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            let current_selection = doc.selection(view.id);

            let selection = movement::move_parent_node_end(
                syntax,
                text,
                current_selection.clone(),
                dir,
                movement,
            );

            doc.set_selection(view.id, selection);
        }
    };

    cx.editor.apply_motion(motion);
}

pub fn move_parent_node_end(cx: &mut Context) {
    move_node_bound_impl(cx, Direction::Forward, Movement::Move)
}

pub fn move_parent_node_start(cx: &mut Context) {
    move_node_bound_impl(cx, Direction::Backward, Movement::Move)
}

pub fn extend_parent_node_end(cx: &mut Context) {
    move_node_bound_impl(cx, Direction::Forward, Movement::Extend)
}

pub fn extend_parent_node_start(cx: &mut Context) {
    move_node_bound_impl(cx, Direction::Backward, Movement::Extend)
}

fn select_all_impl<F>(editor: &mut Editor, select_fn: F)
where
    F: Fn(&Syntax, RopeSlice, Selection) -> Selection,
{
    let (view, doc) = current!(editor);

    if let Some(syntax) = doc.syntax() {
        let text = doc.text().slice(..);
        let current_selection = doc.selection(view.id);
        let selection = select_fn(syntax, text, current_selection.clone());
        doc.set_selection(view.id, selection);
    }
}

fn select_all_siblings(cx: &mut Context) {
    let motion = |editor: &mut Editor| {
        select_all_impl(editor, object::select_all_siblings);
    };

    cx.editor.apply_motion(motion);
}

fn select_all_children(cx: &mut Context) {
    let motion = |editor: &mut Editor| {
        select_all_impl(editor, object::select_all_children);
    };

    cx.editor.apply_motion(motion);
}

fn match_brackets(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let is_select = cx.editor.mode == Mode::Select;
    let text = doc.text();
    let text_slice = text.slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let pos = range.cursor(text_slice);
        if let Some(matched_pos) = doc.syntax().map_or_else(
            || match_brackets::find_matching_bracket_plaintext(text.slice(..), pos),
            |syntax| match_brackets::find_matching_bracket_fuzzy(syntax, text.slice(..), pos),
        ) {
            range.put_cursor(text_slice, matched_pos, is_select)
        } else {
            range
        }
    });

    doc.set_selection(view.id, selection);
}

//

fn jump_forward(cx: &mut Context) {
    let count = cx.count();
    let config = cx.editor.config();
    let view = view_mut!(cx.editor);
    let doc_id = view.doc;

    if let Some((id, selection)) = view.jumps.forward(count) {
        view.doc = *id;
        let selection = selection.clone();
        let (view, doc) = current!(cx.editor); // refetch doc

        if doc.id() != doc_id {
            view.add_to_history(doc_id);
        }

        doc.set_selection(view.id, selection);
        // Document we switch to might not have been opened in the view before
        doc.ensure_view_init(view.id);
        view.ensure_cursor_in_view_center(doc, config.scrolloff);
    };
}

fn jump_backward(cx: &mut Context) {
    let count = cx.count();
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);
    let doc_id = doc.id();

    if let Some((id, selection)) = view.jumps.backward(view.id, doc, count) {
        view.doc = *id;
        let selection = selection.clone();
        let (view, doc) = current!(cx.editor); // refetch doc

        if doc.id() != doc_id {
            view.add_to_history(doc_id);
        }

        doc.set_selection(view.id, selection);
        // Document we switch to might not have been opened in the view before
        doc.ensure_view_init(view.id);
        view.ensure_cursor_in_view_center(doc, config.scrolloff);
    };
}

fn save_selection(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    push_jump(view, doc);
    cx.editor.set_status("Selection saved to jumplist");
}

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

fn scroll_up(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Backward, false);
}

fn scroll_down(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Forward, false);
}

fn goto_ts_object_impl(cx: &mut Context, object: &'static str, direction: Direction) {
    let count = cx.count();
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);
        let loader = editor.syn_loader.load();
        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            let root = syntax.tree().root_node();

            let selection = doc.selection(view.id).clone().transform(|range| {
                let new_range = movement::goto_treesitter_object(
                    text, range, object, direction, &root, syntax, &loader, count,
                );

                if editor.mode == Mode::Select {
                    let head = if new_range.head < range.anchor {
                        new_range.anchor
                    } else {
                        new_range.head
                    };

                    Range::new(range.anchor, head)
                } else {
                    new_range.with_direction(direction)
                }
            });

            doc.set_selection(view.id, selection);
        } else {
            editor.set_status("Syntax-tree is not available in current buffer");
        }
    };
    cx.editor.apply_motion(motion);
}

fn goto_next_function(cx: &mut Context) {
    goto_ts_object_impl(cx, "function", Direction::Forward)
}

fn goto_prev_function(cx: &mut Context) {
    goto_ts_object_impl(cx, "function", Direction::Backward)
}

fn goto_next_class(cx: &mut Context) {
    goto_ts_object_impl(cx, "class", Direction::Forward)
}

fn goto_prev_class(cx: &mut Context) {
    goto_ts_object_impl(cx, "class", Direction::Backward)
}

fn goto_next_parameter(cx: &mut Context) {
    goto_ts_object_impl(cx, "parameter", Direction::Forward)
}

fn goto_prev_parameter(cx: &mut Context) {
    goto_ts_object_impl(cx, "parameter", Direction::Backward)
}

fn goto_next_comment(cx: &mut Context) {
    goto_ts_object_impl(cx, "comment", Direction::Forward)
}

fn goto_prev_comment(cx: &mut Context) {
    goto_ts_object_impl(cx, "comment", Direction::Backward)
}

fn goto_next_test(cx: &mut Context) {
    goto_ts_object_impl(cx, "test", Direction::Forward)
}

fn goto_prev_test(cx: &mut Context) {
    goto_ts_object_impl(cx, "test", Direction::Backward)
}

fn goto_next_entry(cx: &mut Context) {
    goto_ts_object_impl(cx, "entry", Direction::Forward)
}

fn goto_prev_entry(cx: &mut Context) {
    goto_ts_object_impl(cx, "entry", Direction::Backward)
}

fn select_textobject_around(cx: &mut Context) {
    select_textobject(cx, textobject::TextObject::Around);
}

fn select_textobject_inner(cx: &mut Context) {
    select_textobject(cx, textobject::TextObject::Inside);
}

fn select_textobject(cx: &mut Context, objtype: textobject::TextObject) {
    let count = cx.count();

    cx.on_next_key(move |cx, event| {
        cx.editor.autoinfo = None;
        if let Some(ch) = event.char() {
            let textobject = move |editor: &mut Editor| {
                let (view, doc) = current!(editor);
                let loader = editor.syn_loader.load();
                let text = doc.text().slice(..);

                let textobject_treesitter = |obj_name: &str, range: Range| -> Range {
                    let Some(syntax) = doc.syntax() else {
                        return range;
                    };
                    textobject::textobject_treesitter(
                        text, range, objtype, obj_name, syntax, &loader, count,
                    )
                };

                if ch == 'g' && doc.diff_handle().is_none() {
                    editor.set_status("Diff is not available in current buffer");
                    return;
                }

                let textobject_change = |range: Range| -> Range {
                    let diff_handle = doc.diff_handle().unwrap();
                    let diff = diff_handle.load();
                    let line = range.cursor_line(text);
                    let hunk_idx = if let Some(hunk_idx) = diff.hunk_at(line as u32, false) {
                        hunk_idx
                    } else {
                        return range;
                    };
                    let hunk = diff.nth_hunk(hunk_idx).after;

                    let start = text.line_to_char(hunk.start as usize);
                    let end = text.line_to_char(hunk.end as usize);
                    Range::new(start, end).with_direction(range.direction())
                };

                let selection = doc.selection(view.id).clone().transform(|range| {
                    match ch {
                        'w' => textobject::textobject_word(text, range, objtype, count, false),
                        'W' => textobject::textobject_word(text, range, objtype, count, true),
                        't' => textobject_treesitter("class", range),
                        'f' => textobject_treesitter("function", range),
                        'a' => textobject_treesitter("parameter", range),
                        'c' => textobject_treesitter("comment", range),
                        'T' => textobject_treesitter("test", range),
                        'e' => textobject_treesitter("entry", range),
                        'p' => textobject::textobject_paragraph(text, range, objtype, count),
                        'm' => textobject::textobject_pair_surround_closest(
                            doc.syntax(),
                            text,
                            range,
                            objtype,
                            count,
                        ),
                        'g' => textobject_change(range),
                        // TODO: cancel new ranges if inconsistent surround matches across lines
                        ch if !ch.is_ascii_alphanumeric() => textobject::textobject_pair_surround(
                            doc.syntax(),
                            text,
                            range,
                            objtype,
                            ch,
                            count,
                        ),
                        _ => range,
                    }
                });
                doc.set_selection(view.id, selection);
            };
            cx.editor.apply_motion(textobject);
        }
    });

    let title = match objtype {
        textobject::TextObject::Inside => "Match inside",
        textobject::TextObject::Around => "Match around",
        _ => return,
    };
    let help_text = [
        ("w", "Word"),
        ("W", "WORD"),
        ("p", "Paragraph"),
        ("t", "Type definition (tree-sitter)"),
        ("f", "Function (tree-sitter)"),
        ("a", "Argument/parameter (tree-sitter)"),
        ("c", "Comment (tree-sitter)"),
        ("T", "Test (tree-sitter)"),
        ("e", "Data structure entry (tree-sitter)"),
        ("m", "Closest surrounding pair (tree-sitter)"),
        ("g", "Change"),
        (" ", "... or any character acting as a pair"),
    ];

    cx.editor.autoinfo = Some(Info::new(title, &help_text));
}

static SURROUND_HELP_TEXT: [(&str, &str); 6] = [
    ("m", "Nearest matching pair"),
    ("( or )", "Parentheses"),
    ("{ or }", "Curly braces"),
    ("< or >", "Angled brackets"),
    ("[ or ]", "Square brackets"),
    (" ", "... or any character"),
];

fn surround_add(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        cx.editor.autoinfo = None;
        let (view, doc) = current!(cx.editor);
        // surround_len is the number of new characters being added.
        let (open, close, surround_len) = match event.char() {
            Some(ch) => {
                let (o, c) = match_brackets::get_pair(ch);
                let mut open = Tendril::new();
                open.push(o);
                let mut close = Tendril::new();
                close.push(c);
                (open, close, 2)
            }
            None if event.code == KeyCode::Enter => (
                doc.line_ending.as_str().into(),
                doc.line_ending.as_str().into(),
                2 * doc.line_ending.len_chars(),
            ),
            None => return,
        };

        let selection = doc.selection(view.id);
        let mut changes = Vec::with_capacity(selection.len() * 2);
        let mut ranges = SmallVec::with_capacity(selection.len());
        let mut offs = 0;

        for range in selection.iter() {
            changes.push((range.from(), range.from(), Some(open.clone())));
            changes.push((range.to(), range.to(), Some(close.clone())));

            ranges.push(
                Range::new(offs + range.from(), offs + range.to() + surround_len)
                    .with_direction(range.direction()),
            );

            offs += surround_len;
        }

        let transaction = Transaction::change(doc.text(), changes.into_iter())
            .with_selection(Selection::new(ranges, selection.primary_index()));
        doc.apply(&transaction, view.id);
        exit_select_mode(cx);
    });

    cx.editor.autoinfo = Some(Info::new(
        "Surround selections with",
        &SURROUND_HELP_TEXT[1..],
    ));
}

fn surround_replace(cx: &mut Context) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        cx.editor.autoinfo = None;
        let surround_ch = match event.char() {
            Some('m') => None, // m selects the closest surround pair
            Some(ch) => Some(ch),
            None => return,
        };
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id);

        let change_pos =
            match surround::get_surround_pos(doc.syntax(), text, selection, surround_ch, count) {
                Ok(c) => c,
                Err(err) => {
                    cx.editor.set_error(err.to_string());
                    return;
                }
            };

        let selection = selection.clone();
        let ranges: SmallVec<[Range; 1]> = change_pos.iter().map(|&p| Range::point(p)).collect();
        doc.set_selection(
            view.id,
            Selection::new(ranges, selection.primary_index() * 2),
        );

        cx.on_next_key(move |cx, event| {
            cx.editor.autoinfo = None;
            let (view, doc) = current!(cx.editor);
            let to = match event.char() {
                Some(to) => to,
                None => return doc.set_selection(view.id, selection),
            };
            let (open, close) = match_brackets::get_pair(to);

            // the changeset has to be sorted to allow nested surrounds
            let mut sorted_pos: Vec<(usize, char)> = Vec::new();
            for p in change_pos.chunks(2) {
                sorted_pos.push((p[0], open));
                sorted_pos.push((p[1], close));
            }
            sorted_pos.sort_unstable();

            let transaction = Transaction::change(
                doc.text(),
                sorted_pos.iter().map(|&pos| {
                    let mut t = Tendril::new();
                    t.push(pos.1);
                    (pos.0, pos.0 + 1, Some(t))
                }),
            );
            doc.set_selection(view.id, selection);
            doc.apply(&transaction, view.id);
            exit_select_mode(cx);
        });

        cx.editor.autoinfo = Some(Info::new(
            "Replace with a pair of",
            &SURROUND_HELP_TEXT[1..],
        ));
    });

    cx.editor.autoinfo = Some(Info::new(
        "Replace surrounding pair of",
        &SURROUND_HELP_TEXT,
    ));
}

fn surround_delete(cx: &mut Context) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        cx.editor.autoinfo = None;
        let surround_ch = match event.char() {
            Some('m') => None, // m selects the closest surround pair
            Some(ch) => Some(ch),
            None => return,
        };
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id);

        let mut change_pos =
            match surround::get_surround_pos(doc.syntax(), text, selection, surround_ch, count) {
                Ok(c) => c,
                Err(err) => {
                    cx.editor.set_error(err.to_string());
                    return;
                }
            };
        change_pos.sort_unstable(); // the changeset has to be sorted to allow nested surrounds
        let transaction =
            Transaction::change(doc.text(), change_pos.into_iter().map(|p| (p, p + 1, None)));
        doc.apply(&transaction, view.id);
        exit_select_mode(cx);
    });

    cx.editor.autoinfo = Some(Info::new("Delete surrounding pair of", &SURROUND_HELP_TEXT));
}

#[derive(Eq, PartialEq)]
enum ShellBehavior {
    Replace,
    Ignore,
    Insert,
    Append,
}

fn shell_pipe(cx: &mut Context) {
    shell_prompt(cx, "pipe:".into(), ShellBehavior::Replace);
}

fn shell_pipe_to(cx: &mut Context) {
    shell_prompt(cx, "pipe-to:".into(), ShellBehavior::Ignore);
}

fn shell_insert_output(cx: &mut Context) {
    shell_prompt(cx, "insert-output:".into(), ShellBehavior::Insert);
}

fn shell_append_output(cx: &mut Context) {
    shell_prompt(cx, "append-output:".into(), ShellBehavior::Append);
}

fn shell_keep_pipe(cx: &mut Context) {
    ui::prompt(
        cx,
        "keep-pipe:".into(),
        Some('|'),
        ui::completers::none,
        move |cx, input: &str, event: PromptEvent| {
            let shell = &cx.editor.config().shell;
            if event != PromptEvent::Validate {
                return;
            }
            if input.is_empty() {
                return;
            }
            let (view, doc) = current!(cx.editor);
            let selection = doc.selection(view.id);

            let mut ranges = SmallVec::with_capacity(selection.len());
            let old_index = selection.primary_index();
            let mut index: Option<usize> = None;
            let text = doc.text().slice(..);

            for (i, range) in selection.ranges().iter().enumerate() {
                let fragment = range.slice(text);
                if let Err(err) = shell_impl(shell, input, Some(fragment.into())) {
                    log::debug!("Shell command failed: {}", err);
                } else {
                    ranges.push(*range);
                    if i >= old_index && index.is_none() {
                        index = Some(ranges.len() - 1);
                    }
                }
            }

            if ranges.is_empty() {
                cx.editor.set_error("No selections remaining");
                return;
            }

            let index = index.unwrap_or_else(|| ranges.len() - 1);
            doc.set_selection(view.id, Selection::new(ranges, index));
        },
    );
}

fn shell_impl(shell: &[String], cmd: &str, input: Option<Rope>) -> anyhow::Result<Tendril> {
    tokio::task::block_in_place(|| helix_lsp::block_on(shell_impl_async(shell, cmd, input)))
}

async fn shell_impl_async(
    shell: &[String],
    cmd: &str,
    input: Option<Rope>,
) -> anyhow::Result<Tendril> {
    use std::process::Stdio;
    use tokio::process::Command;
    ensure!(!shell.is_empty(), "No shell set");

    let mut process = Command::new(&shell[0]);
    process
        .args(&shell[1..])
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if input.is_some() || cfg!(windows) {
        process.stdin(Stdio::piped());
    } else {
        process.stdin(Stdio::null());
    }

    let mut process = match process.spawn() {
        Ok(process) => process,
        Err(e) => {
            log::error!("Failed to start shell: {}", e);
            return Err(e.into());
        }
    };
    let output = if let Some(mut stdin) = process.stdin.take() {
        let input_task = tokio::spawn(async move {
            if let Some(input) = input {
                helix_view::document::to_writer(&mut stdin, (encoding::UTF_8, false), &input)
                    .await?;
            }
            anyhow::Ok(())
        });
        let (output, _) = tokio::join! {
            process.wait_with_output(),
            input_task,
        };
        output?
    } else {
        // Process has no stdin, so we just take the output
        process.wait_with_output().await?
    };

    let output = if !output.status.success() {
        if output.stderr.is_empty() {
            match output.status.code() {
                Some(exit_code) => bail!("Shell command failed: status {}", exit_code),
                None => bail!("Shell command failed"),
            }
        }
        String::from_utf8_lossy(&output.stderr)
        // Prioritize `stderr` output over `stdout`
    } else if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::debug!("Command printed to stderr: {stderr}");
        stderr
    } else {
        String::from_utf8_lossy(&output.stdout)
    };

    Ok(Tendril::from(output))
}

fn shell(cx: &mut compositor::Context, cmd: &str, behavior: &ShellBehavior) {
    let pipe = match behavior {
        ShellBehavior::Replace | ShellBehavior::Ignore => true,
        ShellBehavior::Insert | ShellBehavior::Append => false,
    };

    let config = cx.editor.config();
    let shell = &config.shell;
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);

    let mut changes = Vec::with_capacity(selection.len());
    let mut ranges = SmallVec::with_capacity(selection.len());
    let text = doc.text().slice(..);

    let mut shell_output: Option<Tendril> = None;
    let mut offset = 0isize;
    for range in selection.ranges() {
        let output = if let Some(output) = shell_output.as_ref() {
            output.clone()
        } else {
            let input = range.slice(text);
            match shell_impl(shell, cmd, pipe.then(|| input.into())) {
                Ok(mut output) => {
                    if !input.ends_with("\n") && output.ends_with('\n') {
                        output.pop();
                        if output.ends_with('\r') {
                            output.pop();
                        }
                    }

                    if !pipe {
                        shell_output = Some(output.clone());
                    }
                    output
                }
                Err(err) => {
                    cx.editor.set_error(err.to_string());
                    return;
                }
            }
        };

        let output_len = output.chars().count();

        let (from, to, deleted_len) = match behavior {
            ShellBehavior::Replace => (range.from(), range.to(), range.len()),
            ShellBehavior::Insert => (range.from(), range.from(), 0),
            ShellBehavior::Append => (range.to(), range.to(), 0),
            _ => (range.from(), range.from(), 0),
        };

        // These `usize`s cannot underflow because selection ranges cannot overlap.
        let anchor = to
            .checked_add_signed(offset)
            .expect("Selection ranges cannot overlap")
            .checked_sub(deleted_len)
            .expect("Selection ranges cannot overlap");
        let new_range = Range::new(anchor, anchor + output_len).with_direction(range.direction());
        ranges.push(new_range);
        offset = offset
            .checked_add_unsigned(output_len)
            .expect("Selection ranges cannot overlap")
            .checked_sub_unsigned(deleted_len)
            .expect("Selection ranges cannot overlap");

        changes.push((from, to, Some(output)));
    }

    if behavior != &ShellBehavior::Ignore {
        let transaction = Transaction::change(doc.text(), changes.into_iter())
            .with_selection(Selection::new(ranges, selection.primary_index()));
        doc.apply(&transaction, view.id);
        doc.append_changes_to_history(view);
    }

    // after replace cursor may be out of bounds, do this to
    // make sure cursor is in view and update scroll as well
    view.ensure_cursor_in_view(doc, config.scrolloff);
}

fn shell_prompt(cx: &mut Context, prompt: Cow<'static, str>, behavior: ShellBehavior) {
    ui::prompt(
        cx,
        prompt,
        Some('|'),
        ui::completers::shell,
        move |cx, input: &str, event: PromptEvent| {
            if event != PromptEvent::Validate {
                return;
            }
            if input.is_empty() {
                return;
            }

            shell(cx, input, &behavior);
        },
    );
}

fn suspend(_cx: &mut Context) {
    #[cfg(not(windows))]
    {
        _cx.block_try_flush_writes().ok();
        signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
    }
}

fn add_newline_above(cx: &mut Context) {
    add_newline_impl(cx, Open::Above);
}

fn add_newline_below(cx: &mut Context) {
    add_newline_impl(cx, Open::Below)
}

fn add_newline_impl(cx: &mut Context, open: Open) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);
    let text = doc.text();
    let slice = text.slice(..);

    let changes = selection.into_iter().map(|range| {
        let (start, end) = range.line_range(slice);
        let line = match open {
            Open::Above => start,
            Open::Below => end + 1,
        };
        let pos = text.line_to_char(line);
        (
            pos,
            pos,
            Some(doc.line_ending.as_str().repeat(count).into()),
        )
    });

    let transaction = Transaction::change(text, changes);
    doc.apply(&transaction, view.id);
}

enum IncrementDirection {
    Increase,
    Decrease,
}

/// Increment objects within selections by count.
fn increment(cx: &mut Context) {
    increment_impl(cx, IncrementDirection::Increase);
}

/// Decrement objects within selections by count.
fn decrement(cx: &mut Context) {
    increment_impl(cx, IncrementDirection::Decrease);
}

/// Increment objects within selections by `amount`.
/// A negative `amount` will decrement objects within selections.
fn increment_impl(cx: &mut Context, increment_direction: IncrementDirection) {
    let sign = match increment_direction {
        IncrementDirection::Increase => 1,
        IncrementDirection::Decrease => -1,
    };
    let mut amount = sign * cx.count() as i64;
    // If the register is `#` then increase or decrease the `amount` by 1 per element
    let increase_by = if cx.register == Some('#') { sign } else { 0 };

    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id);
    let text = doc.text().slice(..);

    let mut new_selection_ranges = SmallVec::new();
    let mut cumulative_length_diff: i128 = 0;
    let mut changes = vec![];

    for range in selection {
        let selected_text: Cow<str> = range.fragment(text);
        let new_from = ((range.from() as i128) + cumulative_length_diff) as usize;
        let incremented = [increment::integer, increment::date_time]
            .iter()
            .find_map(|incrementor| incrementor(selected_text.as_ref(), amount));

        amount += increase_by;

        match incremented {
            None => {
                let new_range = Range::new(
                    new_from,
                    (range.to() as i128 + cumulative_length_diff) as usize,
                );
                new_selection_ranges.push(new_range);
            }
            Some(new_text) => {
                let new_range = Range::new(new_from, new_from + new_text.len());
                cumulative_length_diff += new_text.len() as i128 - selected_text.len() as i128;
                new_selection_ranges.push(new_range);
                changes.push((range.from(), range.to(), Some(new_text.into())));
            }
        }
    }

    if !changes.is_empty() {
        let new_selection = Selection::new(new_selection_ranges, selection.primary_index());
        let transaction = Transaction::change(doc.text(), changes.into_iter());
        let transaction = transaction.with_selection(new_selection);
        doc.apply(&transaction, view.id);
        exit_select_mode(cx);
    }
}

fn goto_next_tabstop(cx: &mut Context) {
    goto_next_tabstop_impl(cx, Direction::Forward)
}

fn goto_prev_tabstop(cx: &mut Context) {
    goto_next_tabstop_impl(cx, Direction::Backward)
}

fn goto_next_tabstop_impl(cx: &mut Context, direction: Direction) {
    let (view, doc) = current!(cx.editor);
    let view_id = view.id;
    let Some(mut snippet) = doc.active_snippet.take() else {
        cx.editor.set_error("no snippet is currently active");
        return;
    };
    let tabstop = match direction {
        Direction::Forward => Some(snippet.next_tabstop(doc.selection(view_id))),
        Direction::Backward => snippet
            .prev_tabstop(doc.selection(view_id))
            .map(|selection| (selection, false)),
    };
    let Some((selection, last_tabstop)) = tabstop else {
        return;
    };
    doc.set_selection(view_id, selection);
    if !last_tabstop {
        doc.active_snippet = Some(snippet)
    }
    if cx.editor.mode() == Mode::Insert {
        cx.on_next_key_fallback(|cx, key| {
            if let Some(c) = key.char() {
                let (view, doc) = current!(cx.editor);
                if let Some(snippet) = &doc.active_snippet {
                    doc.apply(&snippet.delete_placeholder(doc.text()), view.id);
                }
                insert_char(cx, c);
            }
        })
    }
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
        match helix_view::input::parse_macro(&keys) {
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

fn goto_word(cx: &mut Context) {
    jump_to_word(cx, Movement::Move)
}

fn extend_to_word(cx: &mut Context) {
    jump_to_word(cx, Movement::Extend)
}

fn jump_to_label(cx: &mut Context, labels: Vec<Range>, behaviour: Movement) {
    let doc = doc!(cx.editor);
    let alphabet = &cx.editor.config().jump_label_alphabet;
    if labels.is_empty() {
        return;
    }
    let alphabet_char = |i| {
        let mut res = Tendril::new();
        res.push(alphabet[i]);
        res
    };

    // Add label for each jump candidate to the View as virtual text.
    let text = doc.text().slice(..);
    let mut overlays: Vec<_> = labels
        .iter()
        .enumerate()
        .flat_map(|(i, range)| {
            [
                Overlay::new(range.from(), alphabet_char(i / alphabet.len())),
                Overlay::new(
                    graphemes::next_grapheme_boundary(text, range.from()),
                    alphabet_char(i % alphabet.len()),
                ),
            ]
        })
        .collect();
    overlays.sort_unstable_by_key(|overlay| overlay.char_idx);
    let (view, doc) = current!(cx.editor);
    doc.set_jump_labels(view.id, overlays);

    // Accept two characters matching a visible label. Jump to the candidate
    // for that label if it exists.
    let primary_selection = doc.selection(view.id).primary();
    let view = view.id;
    let doc = doc.id();
    cx.on_next_key(move |cx, event| {
        let alphabet = &cx.editor.config().jump_label_alphabet;
        let Some(i) = event
            .char()
            .filter(|_| event.modifiers.is_empty())
            .and_then(|ch| alphabet.iter().position(|&it| it == ch))
        else {
            doc_mut!(cx.editor, &doc).remove_jump_labels(view);
            return;
        };
        let outer = i * alphabet.len();
        // Bail if the given character cannot be a jump label.
        if outer > labels.len() {
            doc_mut!(cx.editor, &doc).remove_jump_labels(view);
            return;
        }
        cx.on_next_key(move |cx, event| {
            doc_mut!(cx.editor, &doc).remove_jump_labels(view);
            let alphabet = &cx.editor.config().jump_label_alphabet;
            let Some(inner) = event
                .char()
                .filter(|_| event.modifiers.is_empty())
                .and_then(|ch| alphabet.iter().position(|&it| it == ch))
            else {
                return;
            };
            if let Some(mut range) = labels.get(outer + inner).copied() {
                range = if behaviour == Movement::Extend {
                    let anchor = if range.anchor < range.head {
                        let from = primary_selection.from();
                        if range.anchor < from {
                            range.anchor
                        } else {
                            from
                        }
                    } else {
                        let to = primary_selection.to();
                        if range.anchor > to {
                            range.anchor
                        } else {
                            to
                        }
                    };
                    Range::new(anchor, range.head)
                } else {
                    range.with_direction(Direction::Forward)
                };
                doc_mut!(cx.editor, &doc).set_selection(view, range.into());
            }
        });
    });
}

fn jump_to_word(cx: &mut Context, behaviour: Movement) {
    // Calculate the jump candidates: ranges for any visible words with two or
    // more characters.
    let alphabet = &cx.editor.config().jump_label_alphabet;
    if alphabet.is_empty() {
        return;
    }

    let jump_label_limit = alphabet.len() * alphabet.len();
    let mut words = Vec::with_capacity(jump_label_limit);
    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text().slice(..);

    // This is not necessarily exact if there is virtual text like soft wrap.
    // It's ok though because the extra jump labels will not be rendered.
    let start = text.line_to_char(text.char_to_line(doc.view_offset(view.id).anchor));
    let end = text.line_to_char(view.estimate_last_doc_line(doc) + 1);

    let primary_selection = doc.selection(view.id).primary();
    let cursor = primary_selection.cursor(text);
    let mut cursor_fwd = Range::point(cursor);
    let mut cursor_rev = Range::point(cursor);
    if text.get_char(cursor).is_some_and(|c| !c.is_whitespace()) {
        let cursor_word_end = movement::move_next_word_end(text, cursor_fwd, 1);
        //  single grapheme words need a special case
        if cursor_word_end.anchor == cursor {
            cursor_fwd = cursor_word_end;
        }
        let cursor_word_start = movement::move_prev_word_start(text, cursor_rev, 1);
        if cursor_word_start.anchor == next_grapheme_boundary(text, cursor) {
            cursor_rev = cursor_word_start;
        }
    }
    'outer: loop {
        let mut changed = false;
        while cursor_fwd.head < end {
            cursor_fwd = movement::move_next_word_end(text, cursor_fwd, 1);
            // The cursor is on a word that is atleast two graphemes long and
            // madeup of word characters. The latter condition is needed because
            // move_next_word_end simply treats a sequence of characters from
            // the same char class as a word so `=<` would also count as a word.
            let add_label = text
                .slice(..cursor_fwd.head)
                .graphemes_rev()
                .take(2)
                .take_while(|g| g.chars().all(char_is_word))
                .count()
                == 2;
            if !add_label {
                continue;
            }
            changed = true;
            // skip any leading whitespace
            cursor_fwd.anchor += text
                .chars_at(cursor_fwd.anchor)
                .take_while(|&c| !char_is_word(c))
                .count();
            words.push(cursor_fwd);
            if words.len() == jump_label_limit {
                break 'outer;
            }
            break;
        }
        while cursor_rev.head > start {
            cursor_rev = movement::move_prev_word_start(text, cursor_rev, 1);
            // The cursor is on a word that is atleast two graphemes long and
            // madeup of word characters. The latter condition is needed because
            // move_prev_word_start simply treats a sequence of characters from
            // the same char class as a word so `=<` would also count as a word.
            let add_label = text
                .slice(cursor_rev.head..)
                .graphemes()
                .take(2)
                .take_while(|g| g.chars().all(char_is_word))
                .count()
                == 2;
            if !add_label {
                continue;
            }
            changed = true;
            cursor_rev.anchor -= text
                .chars_at(cursor_rev.anchor)
                .reversed()
                .take_while(|&c| !char_is_word(c))
                .count();
            words.push(cursor_rev);
            if words.len() == jump_label_limit {
                break 'outer;
            }
            break;
        }
        if !changed {
            break;
        }
    }
    jump_to_label(cx, words, behaviour)
}

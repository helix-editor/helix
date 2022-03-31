pub(crate) mod dap;
pub(crate) mod lsp;
pub(crate) mod typed;

pub use dap::*;
use helix_vcs::Hunk;
pub use lsp::*;
use tokio::sync::oneshot;
use tui::widgets::Row;
pub use typed::*;

use helix_core::{
    char_idx_at_visual_offset, comment,
    doc_formatter::TextFormat,
    encoding, find_first_non_whitespace_char, find_workspace, graphemes,
    history::UndoKind,
    increment, indent,
    indent::IndentStyle,
    line_ending::{get_line_ending_of_str, line_end_char_index, str_is_line_ending},
    match_brackets,
    movement::{self, move_vertically_visual, Direction},
    object, pos_at_coords,
    regex::{self, Regex, RegexBuilder},
    search::{self, CharMatcher},
    selection, shellwords, surround,
    text_annotations::TextAnnotations,
    textobject,
    tree_sitter::Node,
    unicode::width::UnicodeWidthChar,
    visual_offset_from_block, LineEnding, Position, Range, Rope, RopeGraphemes, RopeSlice,
    Selection, SmallVec, Tendril, Transaction,
};
use helix_view::{
    clipboard::ClipboardType,
    document::{FormatterError, Mode, SCRATCH_BUFFER_NAME},
    editor::{Action, Motion},
    info::Info,
    input::KeyEvent,
    keyboard::KeyCode,
    tree,
    view::View,
    Document, DocumentId, Editor, Theme, ViewId,
};

use anyhow::{anyhow, bail, ensure, Context as _};
use fuzzy_matcher::FuzzyMatcher;
use insert::*;
use movement::Movement;

use crate::{
    args,
    compositor::{self, Component, Compositor},
    filter_picker_entry,
    job::Callback,
    keymap::ReverseKeymap,
    ui::{
        self, editor::InsertEvent, lsp::SignatureHelp, overlay::overlaid, FilePicker, Picker,
        Popup, Prompt, PromptEvent,
    },
};

use crate::job::{self, Jobs};
use futures_util::StreamExt;
use std::{collections::HashMap, fmt, future::Future};
use std::{collections::HashSet, num::NonZeroUsize};

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use serde::de::{self, Deserialize, Deserializer};

use grep_regex::RegexMatcherBuilder;
use grep_searcher::{sinks, BinaryDetection, SearcherBuilder};
use ignore::{DirEntry, WalkBuilder, WalkState};
use tokio_stream::wrappers::UnboundedReceiverStream;

pub type OnKeyCallback = Box<dyn FnOnce(&mut Context, KeyEvent)>;

pub struct Context<'a> {
    pub register: Option<char>,
    pub count: Option<NonZeroUsize>,
    pub editor: &'a mut Editor,

    pub callback: Option<crate::compositor::Callback>,
    pub on_next_key_callback: Option<OnKeyCallback>,
    pub jobs: &'a mut Jobs,
}

impl<'a> Context<'a> {
    /// Push a new component onto the compositor.
    pub fn push_layer(&mut self, component: Box<dyn Component>) {
        self.callback = Some(Box::new(|compositor: &mut Compositor, _| {
            compositor.push(component)
        }));
    }

    #[inline]
    pub fn on_next_key(
        &mut self,
        on_next_key_callback: impl FnOnce(&mut Context, KeyEvent) + 'static,
    ) {
        self.on_next_key_callback = Some(Box::new(on_next_key_callback));
    }

    #[inline]
    pub fn callback<T, F>(
        &mut self,
        call: impl Future<Output = helix_lsp::Result<serde_json::Value>> + 'static + Send,
        callback: F,
    ) where
        T: for<'de> serde::Deserialize<'de> + Send + 'static,
        F: FnOnce(&mut Editor, &mut Compositor, T) + Send + 'static,
    {
        self.jobs.callback(make_job_callback(call, callback));
    }

    /// Returns 1 if no explicit count was provided
    #[inline]
    pub fn count(&self) -> usize {
        self.count.map_or(1, |v| v.get())
    }
}

#[inline]
fn make_job_callback<T, F>(
    call: impl Future<Output = helix_lsp::Result<serde_json::Value>> + 'static + Send,
    callback: F,
) -> std::pin::Pin<Box<impl Future<Output = Result<Callback, anyhow::Error>>>>
where
    T: for<'de> serde::Deserialize<'de> + Send + 'static,
    F: FnOnce(&mut Editor, &mut Compositor, T) + Send + 'static,
{
    Box::pin(async move {
        let json = call.await?;
        let response = serde_json::from_value(json)?;
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                callback(editor, compositor, response)
            },
        ));
        Ok(call)
    })
}

use helix_view::{align_view, Align};

/// A MappableCommand is either a static command like "jump_view_up" or a Typable command like
/// :format. It causes a side-effect on the state (usually by creating and applying a transaction).
/// Both of these types of commands can be mapped with keybindings in the config.toml.
#[derive(Clone)]
pub enum MappableCommand {
    Typable {
        name: String,
        args: Vec<String>,
        doc: String,
    },
    Static {
        name: &'static str,
        fun: fn(cx: &mut Context),
        doc: &'static str,
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
                let args: Vec<Cow<str>> = args.iter().map(Cow::from).collect();
                if let Some(command) = typed::TYPABLE_COMMAND_MAP.get(name.as_str()) {
                    let mut cx = compositor::Context {
                        editor: cx.editor,
                        jobs: cx.jobs,
                        scroll: None,
                    };
                    if let Err(e) = (command.fun)(&mut cx, &args[..], PromptEvent::Validate) {
                        cx.editor.set_error(format!("{}", e));
                    }
                }
            }
            Self::Static { fun, .. } => (fun)(cx),
        }
    }

    pub fn name(&self) -> &str {
        match &self {
            Self::Typable { name, .. } => name,
            Self::Static { name, .. } => name,
        }
    }

    pub fn doc(&self) -> &str {
        match &self {
            Self::Typable { doc, .. } => doc,
            Self::Static { doc, .. } => doc,
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
        extend_next_word_start, "Extend to start of next word",
        extend_prev_word_start, "Extend to start of previous word",
        extend_next_word_end, "Extend to end of next word",
        extend_prev_word_end, "Extend to end of previous word",
        extend_next_long_word_start, "Extend to start of next long word",
        extend_prev_long_word_start, "Extend to start of previous long word",
        extend_next_long_word_end, "Extend to end of next long word",
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
        select_all, "Select whole document",
        select_regex, "Select all regex matches inside selections",
        split_selection, "Split selections on regex matches",
        split_selection_on_newline, "Split selection on newlines",
        merge_consecutive_selections, "Merge consecutive selections",
        search, "Search for regex pattern",
        rsearch, "Reverse search for regex pattern",
        search_next, "Select next search match",
        search_prev, "Select previous search match",
        extend_search_next, "Add next search match to selection",
        extend_search_prev, "Add previous search match to selection",
        search_selection, "Use current selection as search pattern",
        make_search_word_bounded, "Modify current search to make it word bounded",
        global_search, "Global search in workspace folder",
        extend_line, "Select current line, if already selected, extend to another line based on the anchor",
        extend_line_below, "Select current line, if already selected, extend to next line",
        extend_line_above, "Select current line, if already selected, extend to previous line",
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
        file_picker_in_current_buffer_directory, "Open file picker at current buffers's directory",
        file_picker_in_current_directory, "Open file picker at current working directory",
        code_action, "Perform code action",
        buffer_picker, "Open buffer picker",
        jumplist_picker, "Open jumplist picker",
        symbol_picker, "Open symbol picker",
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
        goto_file, "Goto files in selection",
        goto_file_hsplit, "Goto files in selection (hsplit)",
        goto_file_vsplit, "Goto files in selection (vsplit)",
        goto_reference, "Goto references",
        goto_window_top, "Goto window top",
        goto_window_center, "Goto window center",
        goto_window_bottom, "Goto window bottom",
        goto_last_accessed_file, "Goto last accessed file",
        goto_last_modified_file, "Goto last modified file",
        goto_last_modification, "Goto last modification",
        goto_line, "Goto line",
        goto_last_line, "Goto last line",
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
        goto_next_buffer, "Goto next buffer",
        goto_previous_buffer, "Goto previous buffer",
        goto_line_end_newline, "Goto newline at line end",
        goto_first_nonwhitespace, "Goto first non-blank in line",
        trim_selections, "Trim whitespace from selections",
        extend_to_line_start, "Extend to line start",
        extend_to_line_end, "Extend to line end",
        extend_to_line_end_newline, "Extend to line end",
        signature_help, "Show signature help",
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
        rotate_selections_forward, "Rotate selections forward",
        rotate_selections_backward, "Rotate selections backward",
        rotate_selection_contents_forward, "Rotate selection contents forward",
        rotate_selection_contents_backward, "Rotate selections contents backward",
        expand_selection, "Expand selection to parent syntax node",
        shrink_selection, "Shrink selection to previously expanded syntax node",
        select_next_sibling, "Select next sibling in syntax tree",
        select_prev_sibling, "Select previous sibling in syntax tree",
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
            let mut typable_command = suffix.split(' ').map(|arg| arg.trim());
            let name = typable_command
                .next()
                .ok_or_else(|| anyhow!("Expected typable command name"))?;
            let args = typable_command
                .map(|s| s.to_owned())
                .collect::<Vec<String>>();
            typed::TYPABLE_COMMAND_MAP
                .get(name)
                .map(|cmd| MappableCommand::Typable {
                    name: cmd.name.to_owned(),
                    doc: format!(":{} {:?}", cmd.name, args),
                    args,
                })
                .ok_or_else(|| anyhow!("No TypableCommand named '{}'", s))
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
    goto_buffer(cx.editor, Direction::Forward);
}

fn goto_previous_buffer(cx: &mut Context) {
    goto_buffer(cx.editor, Direction::Backward);
}

fn goto_buffer(editor: &mut Editor, direction: Direction) {
    let current = view!(editor).doc;

    let id = match direction {
        Direction::Forward => {
            let iter = editor.documents.keys();
            let mut iter = iter.skip_while(|id| *id != &current);
            iter.next(); // skip current item
            iter.next().or_else(|| editor.documents.keys().next())
        }
        Direction::Backward => {
            let iter = editor.documents.keys();
            let mut iter = iter.rev().skip_while(|id| *id != &current);
            iter.next(); // skip current item
            iter.next().or_else(|| editor.documents.keys().rev().next())
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
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let first_char = text.line_to_char(line);
        let anchor = range.cursor(text);
        let head = if anchor == first_char && line != 0 {
            // select until previous line
            line_end_char_index(&text, line - 1)
        } else if let Some(pos) = find_first_non_whitespace_char(text.line(line)) {
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
        Range::new(head, anchor)
    });
    delete_selection_insert_mode(doc, view, &selection);

    lsp::signature_help_impl(cx, SignatureHelpInvoked::Automatic);
}

fn kill_to_line_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let line_end_pos = line_end_char_index(&text, line);
        let pos = range.cursor(text);

        let mut new_range = range.put_cursor(text, line_end_pos, true);
        // don't want to remove the line separator itself if the cursor doesn't reach the end of line.
        if pos != line_end_pos {
            new_range.head = line_end_pos;
        }
        new_range
    });
    delete_selection_insert_mode(doc, view, &selection);

    lsp::signature_help_impl(cx, SignatureHelpInvoked::Automatic);
}

fn goto_first_nonwhitespace(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);

        if let Some(pos) = find_first_non_whitespace_char(text.line(line)) {
            let pos = pos + text.line_to_char(line);
            range.put_cursor(text, pos, cx.editor.mode == Mode::Select)
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
}

fn goto_window(cx: &mut Context, align: Align) {
    let count = cx.count() - 1;
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);

    let height = view.inner_height();

    // respect user given count if any
    // - 1 so we have at least one gap in the middle.
    // a height of 6 with padding of 3 on each side will keep shifting the view back and forth
    // as we type
    let scrolloff = config.scrolloff.min(height.saturating_sub(1) / 2);

    let last_visual_line = view.last_visual_line(doc);

    let visual_line = match align {
        Align::Top => view.offset.vertical_offset + scrolloff + count,
        Align::Center => view.offset.vertical_offset + (last_visual_line / 2),
        Align::Bottom => {
            view.offset.vertical_offset + last_visual_line.saturating_sub(scrolloff + count)
        }
    };
    let visual_line = visual_line
        .max(view.offset.vertical_offset + scrolloff)
        .min(view.offset.vertical_offset + last_visual_line.saturating_sub(scrolloff));

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

fn move_next_long_word_end(cx: &mut Context) {
    move_word_impl(cx, movement::move_next_long_word_end)
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
    motion(cx.editor);
    cx.editor.last_motion = Some(Motion(Box::new(motion)));
}

fn goto_prev_paragraph(cx: &mut Context) {
    goto_para_impl(cx, movement::move_prev_paragraph)
}

fn goto_next_paragraph(cx: &mut Context) {
    goto_para_impl(cx, movement::move_next_paragraph)
}

fn goto_file_start(cx: &mut Context) {
    if cx.count.is_some() {
        goto_line(cx);
    } else {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let selection = doc
            .selection(view.id)
            .clone()
            .transform(|range| range.put_cursor(text, 0, cx.editor.mode == Mode::Select));
        push_jump(view, doc);
        doc.set_selection(view.id, selection);
    }
}

fn goto_file_end(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let pos = doc.text().len_chars();
    let selection = doc
        .selection(view.id)
        .clone()
        .transform(|range| range.put_cursor(text, pos, cx.editor.mode == Mode::Select));
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
    let text = doc.text();
    let selections = doc.selection(view.id);
    let mut paths: Vec<_> = selections
        .iter()
        .map(|r| text.slice(r.from()..r.to()).to_string())
        .collect();
    let primary = selections.primary();
    // Checks whether there is only one selection with a width of 1
    if selections.len() == 1 && primary.len() == 1 {
        let count = cx.count();
        let text_slice = text.slice(..);
        // In this case it selects the WORD under the cursor
        let current_word = textobject::textobject_word(
            text_slice,
            primary,
            textobject::TextObject::Inside,
            count,
            true,
        );
        // Trims some surrounding chars so that the actual file is opened.
        let surrounding_chars: &[_] = &['\'', '"', '(', ')'];
        paths.clear();
        paths.push(
            current_word
                .fragment(text_slice)
                .trim_matches(surrounding_chars)
                .to_string(),
        );
    }
    for sel in paths {
        let p = sel.trim();
        if !p.is_empty() {
            if let Err(e) = cx.editor.open(&PathBuf::from(p), action) {
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

fn extend_next_long_word_end(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_end)
}

fn will_find_char<F>(cx: &mut Context, search_fn: F, inclusive: bool, extend: bool)
where
    F: Fn(RopeSlice, char, usize, usize, bool) -> Option<usize> + 'static,
{
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
            } =>
            // TODO: this isn't quite correct when CRLF is involved.
            // This hack will work in most cases, since documents don't
            // usually mix line endings.  But we should fix it eventually
            // anyway.
            {
                doc!(cx.editor).line_ending.as_str().chars().next().unwrap()
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

        find_char_impl(cx.editor, &search_fn, inclusive, extend, ch, count);
        cx.editor.last_motion = Some(Motion(Box::new(move |editor: &mut Editor| {
            find_char_impl(editor, &search_fn, inclusive, true, ch, 1);
        })));
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
    will_find_char(cx, find_next_char_impl, false, false)
}

fn find_next_char(cx: &mut Context) {
    will_find_char(cx, find_next_char_impl, true, false)
}

fn extend_till_char(cx: &mut Context) {
    will_find_char(cx, find_next_char_impl, false, true)
}

fn extend_next_char(cx: &mut Context) {
    will_find_char(cx, find_next_char_impl, true, true)
}

fn till_prev_char(cx: &mut Context) {
    will_find_char(cx, find_prev_char_impl, false, false)
}

fn find_prev_char(cx: &mut Context) {
    will_find_char(cx, find_prev_char_impl, true, false)
}

fn extend_till_prev_char(cx: &mut Context) {
    will_find_char(cx, find_prev_char_impl, false, true)
}

fn extend_prev_char(cx: &mut Context) {
    will_find_char(cx, find_prev_char_impl, true, true)
}

fn repeat_last_motion(cx: &mut Context) {
    let count = cx.count();
    let last_motion = cx.editor.last_motion.take();
    if let Some(m) = &last_motion {
        for _ in 0..count {
            m.run(cx.editor);
        }
        cx.editor.last_motion = last_motion;
    }
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
                    let text: String =
                        RopeGraphemes::new(doc.text().slice(range.from()..range.to()))
                            .map(|g| {
                                let cow: Cow<str> = g.into();
                                if str_is_line_ending(&cow) {
                                    cow
                                } else {
                                    ch.into()
                                }
                            })
                            .collect();

                    (range.from(), range.to(), Some(text.into()))
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
}

fn switch_case(cx: &mut Context) {
    switch_case_impl(cx, |string| {
        string
            .chars()
            .flat_map(|ch| {
                if ch.is_lowercase() {
                    ch.to_uppercase().collect()
                } else if ch.is_uppercase() {
                    ch.to_lowercase().collect()
                } else {
                    vec![ch]
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

pub fn scroll(cx: &mut Context, offset: usize, direction: Direction) {
    use Direction::*;
    let config = cx.editor.config();
    let (view, doc) = current!(cx.editor);

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
    let annotations = view.text_annotations(doc, None);
    (view.offset.anchor, view.offset.vertical_offset) = char_idx_at_visual_offset(
        doc_text,
        view.offset.anchor,
        view.offset.vertical_offset as isize + offset,
        0,
        &text_fmt,
        &annotations,
    );

    let mut head;
    match direction {
        Forward => {
            let off;
            (head, off) = char_idx_at_visual_offset(
                doc_text,
                view.offset.anchor,
                (view.offset.vertical_offset + scrolloff) as isize,
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
                view.offset.anchor,
                (view.offset.vertical_offset + height - scrolloff - 1) as isize,
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
    doc.set_selection(view.id, sel);
}

fn page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Backward);
}

fn page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height();
    scroll(cx, offset, Direction::Forward);
}

fn half_page_up(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Backward);
}

fn half_page_down(cx: &mut Context) {
    let view = view!(cx.editor);
    let offset = view.inner_height() / 2;
    scroll(cx, offset, Direction::Forward);
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
        move |editor, regex, event| {
            let (view, doc) = current!(editor);
            if !matches!(event, PromptEvent::Update | PromptEvent::Validate) {
                return;
            }
            let text = doc.text().slice(..);
            if let Some(selection) =
                selection::select_on_matches(text, doc.selection(view.id), &regex)
            {
                doc.set_selection(view.id, selection);
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
        move |editor, regex, event| {
            let (view, doc) = current!(editor);
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
    // only compile the regex once
    #[allow(clippy::trivial_regex)]
    static REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"\r\n|[\n\r\u{000B}\u{000C}\u{0085}\u{2028}\u{2029}]").unwrap());
    let selection = selection::split_on_matches(text, doc.selection(view.id), &REGEX);
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
    contents: &str,
    regex: &Regex,
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
    let mut offset = 0;

    // use find_at to find the next match after the cursor, loop around the end
    // Careful, `Regex` uses `bytes` as offsets, not character indices!
    let mut mat = match direction {
        Direction::Forward => regex.find_at(contents, start),
        Direction::Backward => regex.find_iter(&contents[..start]).last(),
    };

    if mat.is_none() {
        if wrap_around {
            mat = match direction {
                Direction::Forward => regex.find(contents),
                Direction::Backward => {
                    offset = start;
                    regex.find_iter(&contents[start..]).last()
                }
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
        let start = text.byte_to_char(mat.start() + offset);
        let end = text.byte_to_char(mat.end() + offset);

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
        .and_then(|reg| cx.editor.registers.get(reg))
        .map_or(Vec::new(), |reg| reg.read().iter().take(200).collect());
    items.sort_unstable();
    items.dedup();
    items.into_iter().cloned().collect()
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

    let doc = doc!(cx.editor);

    // TODO: could probably share with select_on_matches?

    // HAXX: sadly we can't avoid allocating a single string for the whole buffer since we can't
    // feed chunks into the regex yet
    let contents = doc.text().slice(..).to_string();
    let completions = search_completions(cx, Some(reg));

    ui::regex_prompt(
        cx,
        "search:".into(),
        Some(reg),
        move |_editor: &Editor, input: &str| {
            completions
                .iter()
                .filter(|comp| comp.starts_with(input))
                .map(|comp| (0.., std::borrow::Cow::Owned(comp.clone())))
                .collect()
        },
        move |editor, regex, event| {
            if !matches!(event, PromptEvent::Update | PromptEvent::Validate) {
                return;
            }
            search_impl(
                editor,
                &contents,
                &regex,
                Movement::Move,
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
    let config = cx.editor.config();
    let scrolloff = config.scrolloff;
    let (_, doc) = current!(cx.editor);
    let registers = &cx.editor.registers;
    if let Some(query) = registers.read('/').and_then(|query| query.last()) {
        let contents = doc.text().slice(..).to_string();
        let search_config = &config.search;
        let case_insensitive = if search_config.smart_case {
            !query.chars().any(char::is_uppercase)
        } else {
            false
        };
        let wrap_around = search_config.wrap_around;
        if let Ok(regex) = RegexBuilder::new(query)
            .case_insensitive(case_insensitive)
            .multi_line(true)
            .build()
        {
            for _ in 0..count {
                search_impl(
                    cx.editor,
                    &contents,
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
    let (view, doc) = current!(cx.editor);
    let contents = doc.text().slice(..);

    let regex = doc
        .selection(view.id)
        .iter()
        .map(|selection| regex::escape(&selection.fragment(contents)))
        .collect::<HashSet<_>>() // Collect into hashset to deduplicate identical regexes
        .into_iter()
        .collect::<Vec<_>>()
        .join("|");

    let msg = format!("register '{}' set to '{}'", '/', &regex);
    cx.editor.registers.push('/', regex);
    cx.editor.set_status(msg);
}

fn make_search_word_bounded(cx: &mut Context) {
    let regex = match cx.editor.registers.last('/') {
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
    new_regex.push_str(regex);
    if !end_anchored {
        new_regex.push_str("\\b");
    }

    let msg = format!("register '{}' set to '{}'", '/', &new_regex);
    cx.editor.registers.push('/', new_regex);
    cx.editor.set_status(msg);
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

    impl ui::menu::Item for FileResult {
        type Data = Option<PathBuf>;

        fn format(&self, current_path: &Self::Data, _theme: Option<&Theme>) -> Row {
            let relative_path = helix_core::path::get_relative_path(&self.path)
                .to_string_lossy()
                .into_owned();
            if current_path
                .as_ref()
                .map(|p| p == &self.path)
                .unwrap_or(false)
            {
                format!("{} (*)", relative_path).into()
            } else {
                relative_path.into()
            }
        }
    }

    let (all_matches_sx, all_matches_rx) = tokio::sync::mpsc::unbounded_channel::<FileResult>();
    let config = cx.editor.config();
    let smart_case = config.search.smart_case;
    let file_picker_config = config.file_picker.clone();

    let reg = cx.register.unwrap_or('/');

    let completions = search_completions(cx, Some(reg));
    ui::regex_prompt(
        cx,
        "global-search:".into(),
        Some(reg),
        move |_editor: &Editor, input: &str| {
            completions
                .iter()
                .filter(|comp| comp.starts_with(input))
                .map(|comp| (0.., std::borrow::Cow::Owned(comp.clone())))
                .collect()
        },
        move |_editor, regex, event| {
            if event != PromptEvent::Validate {
                return;
            }

            if let Ok(matcher) = RegexMatcherBuilder::new()
                .case_smart(smart_case)
                .build(regex.as_str())
            {
                let searcher = SearcherBuilder::new()
                    .binary_detection(BinaryDetection::quit(b'\x00'))
                    .build();

                let search_root = std::env::current_dir()
                    .expect("Global search error: Failed to get current dir");
                let dedup_symlinks = file_picker_config.deduplicate_links;
                let absolute_root = search_root
                    .canonicalize()
                    .unwrap_or_else(|_| search_root.clone());

                WalkBuilder::new(search_root)
                    .hidden(file_picker_config.hidden)
                    .parents(file_picker_config.parents)
                    .ignore(file_picker_config.ignore)
                    .follow_links(file_picker_config.follow_symlinks)
                    .git_ignore(file_picker_config.git_ignore)
                    .git_global(file_picker_config.git_global)
                    .git_exclude(file_picker_config.git_exclude)
                    .max_depth(file_picker_config.max_depth)
                    .filter_entry(move |entry| {
                        filter_picker_entry(entry, &absolute_root, dedup_symlinks)
                    })
                    .build_parallel()
                    .run(|| {
                        let mut searcher = searcher.clone();
                        let matcher = matcher.clone();
                        let all_matches_sx = all_matches_sx.clone();
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

                            let result = searcher.search_path(
                                &matcher,
                                entry.path(),
                                sinks::UTF8(|line_num, _| {
                                    all_matches_sx
                                        .send(FileResult::new(entry.path(), line_num as usize - 1))
                                        .unwrap();

                                    Ok(true)
                                }),
                            );

                            if let Err(err) = result {
                                log::error!(
                                    "Global search error: {}, {}",
                                    entry.path().display(),
                                    err
                                );
                            }
                            WalkState::Continue
                        })
                    });
            } else {
                // Otherwise do nothing
                // log::warn!("Global Search Invalid Pattern")
            }
        },
    );

    let current_path = doc_mut!(cx.editor).path().cloned();

    let show_picker = async move {
        let all_matches: Vec<FileResult> =
            UnboundedReceiverStream::new(all_matches_rx).collect().await;
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                if all_matches.is_empty() {
                    editor.set_status("No matches found");
                    return;
                }

                let picker = FilePicker::new(
                    all_matches,
                    current_path,
                    move |cx, FileResult { path, line_num }, action| {
                        match cx.editor.open(path, action) {
                            Ok(_) => {}
                            Err(e) => {
                                cx.editor.set_error(format!(
                                    "Failed to open file '{}': {}",
                                    path.display(),
                                    e
                                ));
                                return;
                            }
                        }

                        let line_num = *line_num;
                        let (view, doc) = current!(cx.editor);
                        let text = doc.text();
                        if line_num >= text.len_lines() {
                            cx.editor.set_error("The line you jumped to does not exist anymore because the file has changed.");
                            return;
                        }
                        let start = text.line_to_char(line_num);
                        let end = text.line_to_char((line_num + 1).min(text.len_lines()));

                        doc.set_selection(view.id, Selection::single(start, end));
                        align_view(doc, view, Align::Center);
                    },
                    |_editor, FileResult { path, line_num }| {
                        Some((path.clone().into(), Some((*line_num, *line_num))))
                    },
                );
                compositor.push(Box::new(overlaid(picker)));
            },
        ));
        Ok(call)
    };
    cx.jobs.callback(show_picker);
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

fn delete_selection_impl(cx: &mut Context, op: Operation) {
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id);

    if cx.register != Some('_') {
        // first yank the selection
        let text = doc.text().slice(..);
        let values: Vec<String> = selection.fragments(text).map(Cow::into_owned).collect();
        let reg_name = cx.register.unwrap_or('"');
        cx.editor.registers.write(reg_name, values);
    };

    // then delete
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        (range.from(), range.to(), None)
    });
    doc.apply(&transaction, view.id);

    match op {
        Operation::Delete => {
            // exit select mode, if currently in select mode
            exit_select_mode(cx);
        }
        Operation::Change => {
            enter_insert_mode(cx);
        }
    }
}

#[inline]
fn delete_selection_insert_mode(doc: &mut Document, view: &mut View, selection: &Selection) {
    let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
        (range.from(), range.to(), None)
    });
    doc.apply(&transaction, view.id);
}

fn delete_selection(cx: &mut Context) {
    delete_selection_impl(cx, Operation::Delete);
}

fn delete_selection_noyank(cx: &mut Context) {
    cx.register = Some('_');
    delete_selection_impl(cx, Operation::Delete);
}

fn change_selection(cx: &mut Context) {
    delete_selection_impl(cx, Operation::Change);
}

fn change_selection_noyank(cx: &mut Context) {
    cx.register = Some('_');
    delete_selection_impl(cx, Operation::Change);
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
    let picker = ui::file_picker(root, &cx.editor.config());
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

    let picker = ui::file_picker(path, &cx.editor.config());
    cx.push_layer(Box::new(overlaid(picker)));
}
fn file_picker_in_current_directory(cx: &mut Context) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("./"));
    let picker = ui::file_picker(cwd, &cx.editor.config());
    cx.push_layer(Box::new(overlaid(picker)));
}

fn buffer_picker(cx: &mut Context) {
    let current = view!(cx.editor).doc;

    struct BufferMeta {
        id: DocumentId,
        path: Option<PathBuf>,
        is_modified: bool,
        is_current: bool,
    }

    impl ui::menu::Item for BufferMeta {
        type Data = ();

        fn format(&self, _data: &Self::Data, _theme: Option<&Theme>) -> Row {
            let path = self
                .path
                .as_deref()
                .map(helix_core::path::get_relative_path);
            let path = match path.as_deref().and_then(Path::to_str) {
                Some(path) => path,
                None => SCRATCH_BUFFER_NAME,
            };

            let mut flags = String::new();
            if self.is_modified {
                flags.push('+');
            }
            if self.is_current {
                flags.push('*');
            }

            Row::new([self.id.to_string(), flags, path.to_string()])
        }
    }

    let new_meta = |doc: &Document| BufferMeta {
        id: doc.id(),
        path: doc.path().cloned(),
        is_modified: doc.is_modified(),
        is_current: doc.id() == current,
    };

    let picker = FilePicker::new(
        cx.editor
            .documents
            .values()
            .map(|doc| new_meta(doc))
            .collect(),
        (),
        |cx, meta, action| {
            cx.editor.switch(meta.id, action);
        },
        |editor, meta| {
            let doc = &editor.documents.get(&meta.id)?;
            let &view_id = doc.selections().keys().next()?;
            let line = doc
                .selection(view_id)
                .primary()
                .cursor_line(doc.text().slice(..));
            Some((meta.id.into(), Some((line, line))))
        },
    );
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

    impl ui::menu::Item for JumpMeta {
        type Data = ();

        fn format(&self, _data: &Self::Data, _theme: Option<&Theme>) -> Row {
            let path = self
                .path
                .as_deref()
                .map(helix_core::path::get_relative_path);
            let path = match path.as_deref().and_then(Path::to_str) {
                Some(path) => path,
                None => SCRATCH_BUFFER_NAME,
            };

            let mut flags = Vec::new();
            if self.is_current {
                flags.push("*");
            }

            let flag = if flags.is_empty() {
                "".into()
            } else {
                format!(" ({})", flags.join(""))
            };
            format!("{} {}{} {}", self.id, path, flag, self.text).into()
        }
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

    let picker = FilePicker::new(
        cx.editor
            .tree
            .views()
            .flat_map(|(view, _)| {
                view.jumps
                    .iter()
                    .map(|(doc_id, selection)| new_meta(view, *doc_id, selection.clone()))
            })
            .collect(),
        (),
        |cx, meta, action| {
            cx.editor.switch(meta.id, action);
            let config = cx.editor.config();
            let (view, doc) = current!(cx.editor);
            doc.set_selection(view.id, meta.selection.clone());
            view.ensure_cursor_in_view_center(doc, config.scrolloff);
        },
        |editor, meta| {
            let doc = &editor.documents.get(&meta.id)?;
            let line = meta.selection.primary().cursor_line(doc.text().slice(..));
            Some((meta.path.clone()?.into(), Some((line, line))))
        },
    );
    cx.push_layer(Box::new(overlaid(picker)));
}

impl ui::menu::Item for MappableCommand {
    type Data = ReverseKeymap;

    fn format(&self, keymap: &Self::Data, _theme: Option<&Theme>) -> Row {
        let fmt_binding = |bindings: &Vec<Vec<KeyEvent>>| -> String {
            bindings.iter().fold(String::new(), |mut acc, bind| {
                if !acc.is_empty() {
                    acc.push(' ');
                }
                for key in bind {
                    acc.push_str(&key.key_sequence_format());
                }
                acc
            })
        };

        match self {
            MappableCommand::Typable { doc, name, .. } => match keymap.get(name as &String) {
                Some(bindings) => format!("{} ({}) [:{}]", doc, fmt_binding(bindings), name).into(),
                None => format!("{} [:{}]", doc, name).into(),
            },
            MappableCommand::Static { doc, name, .. } => match keymap.get(*name) {
                Some(bindings) => format!("{} ({}) [{}]", doc, fmt_binding(bindings), name).into(),
                None => format!("{} [{}]", doc, name).into(),
            },
        }
    }
}

pub fn command_palette(cx: &mut Context) {
    cx.callback = Some(Box::new(
        move |compositor: &mut Compositor, cx: &mut compositor::Context| {
            let keymap = compositor.find::<ui::EditorView>().unwrap().keymaps.map()
                [&cx.editor.mode]
                .reverse_map();

            let mut commands: Vec<MappableCommand> = MappableCommand::STATIC_COMMAND_LIST.into();
            commands.extend(typed::TYPABLE_COMMAND_LIST.iter().map(|cmd| {
                MappableCommand::Typable {
                    name: cmd.name.to_owned(),
                    doc: cmd.doc.to_owned(),
                    args: Vec::new(),
                }
            }));

            let picker = Picker::new(commands, keymap, move |cx, command, _action| {
                let mut ctx = Context {
                    register: None,
                    count: std::num::NonZeroUsize::new(1),
                    editor: cx.editor,
                    callback: None,
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
    cx.callback = Some(Box::new(|compositor, cx| {
        if let Some(picker) = compositor.last_picker.take() {
            compositor.push(picker);
        } else {
            cx.editor.set_error("no last picker")
        }
    }));
}

// I inserts at the first nonwhitespace character of each line with a selection
fn insert_at_line_start(cx: &mut Context) {
    goto_first_nonwhitespace(cx);
    enter_insert_mode(cx);
}

// A inserts at the end of each line with a selection
fn insert_at_line_end(cx: &mut Context) {
    enter_insert_mode(cx);
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let text = doc.text().slice(..);
        let line = range.cursor_line(text);
        let pos = line_end_char_index(&text, line);
        Range::new(pos, pos)
    });
    doc.set_selection(view.id, selection);
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

        if let Ok(format) = format {
            if doc.version() == doc_version {
                doc.apply(&format, view.id);
                doc.append_changes_to_history(view);
                doc.detect_indent_and_line_ending();
                view.ensure_cursor_in_view(doc, scrolloff);
            } else {
                log::info!("discarded formatting changes because the document changed");
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

fn open(cx: &mut Context, open: Open) {
    let count = cx.count();
    enter_insert_mode(cx);
    let (view, doc) = current!(cx.editor);

    let text = doc.text().slice(..);
    let contents = doc.text();
    let selection = doc.selection(view.id);

    let mut ranges = SmallVec::with_capacity(selection.len());
    let mut offs = 0;

    let mut transaction = Transaction::change_by_selection(contents, selection, |range| {
        let cursor_line = text.char_to_line(match open {
            Open::Below => graphemes::prev_grapheme_boundary(text, range.to()),
            Open::Above => range.from(),
        });
        let new_line = match open {
            // adjust position to the end of the line (next line - 1)
            Open::Below => cursor_line + 1,
            // adjust position to the end of the previous line (current line - 1)
            Open::Above => cursor_line,
        };

        // Index to insert newlines after, as well as the char width
        // to use to compensate for those inserted newlines.
        let (line_end_index, line_end_offset_width) = if new_line == 0 {
            (0, 0)
        } else {
            (
                line_end_char_index(&doc.text().slice(..), new_line.saturating_sub(1)),
                doc.line_ending.len_chars(),
            )
        };

        let indent = indent::indent_for_newline(
            doc.language_config(),
            doc.syntax(),
            &doc.indent_style,
            doc.tab_width(),
            text,
            new_line.saturating_sub(1),
            line_end_index,
            cursor_line,
        );
        let indent_len = indent.len();
        let mut text = String::with_capacity(1 + indent_len);
        text.push_str(doc.line_ending.as_str());
        text.push_str(&indent);
        let text = text.repeat(count);

        // calculate new selection ranges
        let pos = offs + line_end_index + line_end_offset_width;
        for i in 0..count {
            // pos                    -> beginning of reference line,
            // + (i * (1+indent_len)) -> beginning of i'th line from pos
            // + indent_len ->        -> indent for i'th line
            ranges.push(Range::point(pos + (i * (1 + indent_len)) + indent_len));
        }

        offs += text.chars().count();

        (line_end_index, line_end_index, Some(text.into()))
    });

    transaction = transaction.with_selection(Selection::new(ranges, selection.primary_index()));

    doc.apply(&transaction, view.id);
}

// o inserts a new line after each line with a selection
fn open_below(cx: &mut Context) {
    open(cx, Open::Below)
}

// O inserts a new line before each line with a selection
fn open_above(cx: &mut Context) {
    open(cx, Open::Above)
}

fn normal_mode(cx: &mut Context) {
    cx.editor.enter_normal_mode();
}

// Store a jump on the jumplist.
fn push_jump(view: &mut View, doc: &Document) {
    let jump = (doc.id(), doc.selection(view.id).clone());
    view.jumps.push(jump);
}

fn goto_line(cx: &mut Context) {
    if cx.count.is_some() {
        let (view, doc) = current!(cx.editor);
        push_jump(view, doc);

        goto_line_without_jumplist(cx.editor, cx.count);
    }
}

fn goto_line_without_jumplist(editor: &mut Editor, count: Option<NonZeroUsize>) {
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
            .transform(|range| range.put_cursor(text, pos, editor.mode == Mode::Select));

        doc.set_selection(view.id, selection);
    }
}

fn goto_last_line(cx: &mut Context) {
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
        .transform(|range| range.put_cursor(text, pos, cx.editor.mode == Mode::Select));

    push_jump(view, doc);
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
}

fn goto_last_diag(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let selection = match doc.diagnostics().last() {
        Some(diag) => Selection::single(diag.range.start, diag.range.end),
        None => return,
    };
    doc.set_selection(view.id, selection);
}

fn goto_next_diag(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let cursor_pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));

    let diag = doc
        .diagnostics()
        .iter()
        .find(|diag| diag.range.start > cursor_pos)
        .or_else(|| doc.diagnostics().first());

    let selection = match diag {
        Some(diag) => Selection::single(diag.range.start, diag.range.end),
        None => return,
    };
    doc.set_selection(view.id, selection);
}

fn goto_prev_diag(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    let cursor_pos = doc
        .selection(view.id)
        .primary()
        .cursor(doc.text().slice(..));

    let diag = doc
        .diagnostics()
        .iter()
        .rev()
        .find(|diag| diag.range.start < cursor_pos)
        .or_else(|| doc.diagnostics().last());

    let selection = match diag {
        // NOTE: the selection is reversed because we're jumping to the
        // previous diagnostic.
        Some(diag) => Selection::single(diag.range.end, diag.range.start),
        None => return,
    };
    doc.set_selection(view.id, selection);
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
    motion(cx.editor);
    cx.editor.last_motion = Some(Motion(Box::new(motion)));
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
    use super::*;
    pub type Hook = fn(&Rope, &Selection, char) -> Option<Transaction>;
    pub type PostHook = fn(&mut Context, char);

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

    // It trigger completion when idle timer reaches deadline
    // Only trigger completion if the word under cursor is longer than n characters
    pub fn idle_completion(cx: &mut Context) {
        let config = cx.editor.config();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let cursor = doc.selection(view.id).primary().cursor(text);

        use helix_core::chars::char_is_word;
        let mut iter = text.chars_at(cursor);
        iter.reverse();
        for _ in 0..config.completion_trigger_len {
            match iter.next() {
                Some(c) if char_is_word(c) => {}
                _ => return,
            }
        }
        super::completion(cx);
    }

    fn language_server_completion(cx: &mut Context, ch: char) {
        let config = cx.editor.config();
        if !config.auto_completion {
            return;
        }

        use helix_lsp::lsp;
        // if ch matches completion char, trigger completion
        let doc = doc_mut!(cx.editor);
        let language_server = match doc.language_server() {
            Some(language_server) => language_server,
            None => return,
        };

        let capabilities = language_server.capabilities();

        if let Some(lsp::CompletionOptions {
            trigger_characters: Some(triggers),
            ..
        }) = &capabilities.completion_provider
        {
            // TODO: what if trigger is multiple chars long
            if triggers.iter().any(|trigger| trigger.contains(ch)) {
                cx.editor.clear_idle_timer();
                super::completion(cx);
            }
        }
    }

    fn signature_help(cx: &mut Context, ch: char) {
        use helix_lsp::lsp;
        // if ch matches signature_help char, trigger
        let doc = doc_mut!(cx.editor);
        // The language_server!() macro is not used here since it will
        // print an "LSP not active for current buffer" message on
        // every keypress.
        let language_server = match doc.language_server() {
            Some(language_server) => language_server,
            None => return,
        };

        let capabilities = language_server.capabilities();

        if let lsp::ServerCapabilities {
            signature_help_provider:
                Some(lsp::SignatureHelpOptions {
                    trigger_characters: Some(triggers),
                    // TODO: retrigger_characters
                    ..
                }),
            ..
        } = capabilities
        {
            // TODO: what if trigger is multiple chars long
            let is_trigger = triggers.iter().any(|trigger| trigger.contains(ch));
            // lsp doesn't tell us when to close the signature help, so we request
            // the help information again after common close triggers which should
            // return None, which in turn closes the popup.
            let close_triggers = &[')', ';', '.'];

            if is_trigger || close_triggers.contains(&ch) {
                super::signature_help_impl(cx, SignatureHelpInvoked::Automatic);
            }
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

        // TODO: need a post insert hook too for certain triggers (autocomplete, signature help, etc)
        // this could also generically look at Transaction, but it's a bit annoying to look at
        // Operation instead of Change.
        for hook in &[language_server_completion, signature_help] {
            hook(cx, c);
        }
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
        let (view, doc) = current_ref!(cx.editor);
        let text = doc.text().slice(..);

        let contents = doc.text();
        let selection = doc.selection(view.id).clone();
        let mut ranges = SmallVec::with_capacity(selection.len());

        // TODO: this is annoying, but we need to do it to properly calculate pos after edits
        let mut global_offs = 0;

        let mut transaction = Transaction::change_by_selection(contents, &selection, |range| {
            let pos = range.cursor(text);

            let prev = if pos == 0 {
                ' '
            } else {
                contents.char(pos - 1)
            };
            let curr = contents.get_char(pos).unwrap_or(' ');

            let current_line = text.char_to_line(pos);
            let line_is_only_whitespace = text
                .line(current_line)
                .chars()
                .all(|char| char.is_ascii_whitespace());

            let mut new_text = String::new();

            // If the current line is all whitespace, insert a line ending at the beginning of
            // the current line. This makes the current line empty and the new line contain the
            // indentation of the old line.
            let (from, to, local_offs) = if line_is_only_whitespace {
                let line_start = text.line_to_char(current_line);
                new_text.push_str(doc.line_ending.as_str());

                (line_start, line_start, new_text.chars().count())
            } else {
                let indent = indent::indent_for_newline(
                    doc.language_config(),
                    doc.syntax(),
                    &doc.indent_style,
                    doc.tab_width(),
                    text,
                    current_line,
                    pos,
                    current_line,
                );

                // If we are between pairs (such as brackets), we want to
                // insert an additional line which is indented one level
                // more and place the cursor there
                let on_auto_pair = doc
                    .auto_pairs(cx.editor)
                    .and_then(|pairs| pairs.get(prev))
                    .map_or(false, |pair| pair.open == prev && pair.close == curr);

                let local_offs = if on_auto_pair {
                    let inner_indent = indent.clone() + doc.indent_style.as_str();
                    new_text.reserve_exact(2 + indent.len() + inner_indent.len());
                    new_text.push_str(doc.line_ending.as_str());
                    new_text.push_str(&inner_indent);
                    let local_offs = new_text.chars().count();
                    new_text.push_str(doc.line_ending.as_str());
                    new_text.push_str(&indent);
                    local_offs
                } else {
                    new_text.reserve_exact(1 + indent.len());
                    new_text.push_str(doc.line_ending.as_str());
                    new_text.push_str(&indent);
                    new_text.chars().count()
                };

                (pos, pos, local_offs)
            };

            let new_range = if doc.restore_cursor {
                // when appending, extend the range by local_offs
                Range::new(
                    range.anchor + global_offs,
                    range.head + local_offs + global_offs,
                )
            } else {
                // when inserting, slide the range by local_offs
                Range::new(
                    range.anchor + local_offs + global_offs,
                    range.head + local_offs + global_offs,
                )
            };

            // TODO: range replace or extend
            // range.replace(|range| range.is_empty(), head); -> fn extend if cond true, new head pos
            // can be used with cx.mode to do replace or extend on most changes
            ranges.push(new_range);
            global_offs += new_text.chars().count();

            (from, to, Some(new_text.into()))
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
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                let pos = range.cursor(text);
                if pos == 0 {
                    return (pos, pos, None);
                }
                let line_start_pos = text.line_to_char(range.cursor_line(text));
                // consider to delete by indent level if all characters before `pos` are indent units.
                let fragment = Cow::from(text.slice(line_start_pos..pos));
                if !fragment.is_empty() && fragment.chars().all(|ch| ch == ' ' || ch == '\t') {
                    if text.get_char(pos.saturating_sub(1)) == Some('\t') {
                        // fast path, delete one char
                        (
                            graphemes::nth_prev_grapheme_boundary(text, pos, 1),
                            pos,
                            None,
                        )
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
                        (start, pos, None) // delete!
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
                                None,
                            )
                        }
                        _ =>
                        // delete 1 char
                        {
                            (
                                graphemes::nth_prev_grapheme_boundary(text, pos, count),
                                pos,
                                None,
                            )
                        }
                    }
                }
            });
        let (view, doc) = current!(cx.editor);
        doc.apply(&transaction, view.id);

        lsp::signature_help_impl(cx, SignatureHelpInvoked::Automatic);
    }

    pub fn delete_char_forward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let transaction =
            Transaction::change_by_selection(doc.text(), doc.selection(view.id), |range| {
                let pos = range.cursor(text);
                (
                    pos,
                    graphemes::nth_next_grapheme_boundary(text, pos, count),
                    None,
                )
            });
        doc.apply(&transaction, view.id);

        lsp::signature_help_impl(cx, SignatureHelpInvoked::Automatic);
    }

    pub fn delete_word_backward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let selection = doc.selection(view.id).clone().transform(|range| {
            let anchor = movement::move_prev_word_start(text, range, count).from();
            let next = Range::new(anchor, range.cursor(text));
            exclude_cursor(text, next, range)
        });
        delete_selection_insert_mode(doc, view, &selection);

        lsp::signature_help_impl(cx, SignatureHelpInvoked::Automatic);
    }

    pub fn delete_word_forward(cx: &mut Context) {
        let count = cx.count();
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);

        let selection = doc.selection(view.id).clone().transform(|range| {
            let head = movement::move_next_word_end(text, range, count).to();
            Range::new(range.cursor(text), head)
        });

        delete_selection_insert_mode(doc, view, &selection);

        lsp::signature_help_impl(cx, SignatureHelpInvoked::Automatic);
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
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(text)
        .map(Cow::into_owned)
        .collect();

    let msg = format!(
        "yanked {} selection(s) to register {}",
        values.len(),
        cx.register.unwrap_or('"')
    );

    cx.editor
        .registers
        .write(cx.register.unwrap_or('"'), values);

    cx.editor.set_status(msg);
    exit_select_mode(cx);
}

fn yank_joined_to_clipboard_impl(
    editor: &mut Editor,
    separator: &str,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let values: Vec<String> = doc
        .selection(view.id)
        .fragments(text)
        .map(Cow::into_owned)
        .collect();

    let clipboard_text = match clipboard_type {
        ClipboardType::Clipboard => "system clipboard",
        ClipboardType::Selection => "primary clipboard",
    };

    let msg = format!(
        "joined and yanked {} selection(s) to {}",
        values.len(),
        clipboard_text,
    );

    let joined = values.join(separator);

    editor
        .clipboard_provider
        .set_contents(joined, clipboard_type)
        .context("Couldn't set system clipboard content")?;

    editor.set_status(msg);

    Ok(())
}

fn yank_joined_to_clipboard(cx: &mut Context) {
    let line_ending = doc!(cx.editor).line_ending;
    let _ =
        yank_joined_to_clipboard_impl(cx.editor, line_ending.as_str(), ClipboardType::Clipboard);
    exit_select_mode(cx);
}

fn yank_main_selection_to_clipboard_impl(
    editor: &mut Editor,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);
    let text = doc.text().slice(..);

    let message_text = match clipboard_type {
        ClipboardType::Clipboard => "yanked main selection to system clipboard",
        ClipboardType::Selection => "yanked main selection to primary clipboard",
    };

    let value = doc.selection(view.id).primary().fragment(text);

    if let Err(e) = editor
        .clipboard_provider
        .set_contents(value.into_owned(), clipboard_type)
    {
        bail!("Couldn't set system clipboard content: {}", e);
    }

    editor.set_status(message_text);
    Ok(())
}

fn yank_main_selection_to_clipboard(cx: &mut Context) {
    let _ = yank_main_selection_to_clipboard_impl(cx.editor, ClipboardType::Clipboard);
}

fn yank_joined_to_primary_clipboard(cx: &mut Context) {
    let line_ending = doc!(cx.editor).line_ending;
    let _ =
        yank_joined_to_clipboard_impl(cx.editor, line_ending.as_str(), ClipboardType::Selection);
}

fn yank_main_selection_to_primary_clipboard(cx: &mut Context) {
    let _ = yank_main_selection_to_clipboard_impl(cx.editor, ClipboardType::Selection);
    exit_select_mode(cx);
}

#[derive(Copy, Clone)]
enum Paste {
    Before,
    After,
    Cursor,
}

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

    let repeat = std::iter::repeat(
        // `values` is asserted to have at least one entry above.
        values
            .last()
            .map(|value| Tendril::from(value.repeat(count)))
            .unwrap(),
    );

    // if any of values ends with a line ending, it's linewise paste
    let linewise = values
        .iter()
        .any(|value| get_line_ending_of_str(value).is_some());

    // Only compiled once.
    static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\r\n|\r|\n").unwrap());
    let mut values = values
        .iter()
        .map(|value| REGEX.replace_all(value, doc.line_ending.as_str()))
        .map(|value| Tendril::from(value.as_ref().repeat(count)))
        .chain(repeat);

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
}

fn paste_clipboard_impl(
    editor: &mut Editor,
    action: Paste,
    clipboard_type: ClipboardType,
    count: usize,
) -> anyhow::Result<()> {
    let (view, doc) = current!(editor);
    match editor.clipboard_provider.get_contents(clipboard_type) {
        Ok(contents) => {
            paste_impl(&[contents], doc, view, action, count, editor.mode);
            Ok(())
        }
        Err(e) => Err(e.context("Couldn't get system clipboard contents")),
    }
}

fn paste_clipboard_after(cx: &mut Context) {
    let _ = paste_clipboard_impl(
        cx.editor,
        Paste::After,
        ClipboardType::Clipboard,
        cx.count(),
    );
}

fn paste_clipboard_before(cx: &mut Context) {
    let _ = paste_clipboard_impl(
        cx.editor,
        Paste::Before,
        ClipboardType::Clipboard,
        cx.count(),
    );
}

fn paste_primary_clipboard_after(cx: &mut Context) {
    let _ = paste_clipboard_impl(
        cx.editor,
        Paste::After,
        ClipboardType::Selection,
        cx.count(),
    );
}

fn paste_primary_clipboard_before(cx: &mut Context) {
    let _ = paste_clipboard_impl(
        cx.editor,
        Paste::Before,
        ClipboardType::Selection,
        cx.count(),
    );
}

fn replace_with_yanked(cx: &mut Context) {
    let count = cx.count();
    let reg_name = cx.register.unwrap_or('"');
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;

    if let Some(values) = registers.read(reg_name) {
        if !values.is_empty() {
            let repeat = std::iter::repeat(
                values
                    .last()
                    .map(|value| Tendril::from(&value.repeat(count)))
                    .unwrap(),
            );
            let mut values = values
                .iter()
                .map(|value| Tendril::from(&value.repeat(count)))
                .chain(repeat);
            let selection = doc.selection(view.id);
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                if !range.is_empty() {
                    (range.from(), range.to(), Some(values.next().unwrap()))
                } else {
                    (range.from(), range.to(), None)
                }
            });

            doc.apply(&transaction, view.id);
            exit_select_mode(cx);
        }
    }
}

fn replace_selections_with_clipboard_impl(
    cx: &mut Context,
    clipboard_type: ClipboardType,
) -> anyhow::Result<()> {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);

    match cx.editor.clipboard_provider.get_contents(clipboard_type) {
        Ok(contents) => {
            let selection = doc.selection(view.id);
            let transaction = Transaction::change_by_selection(doc.text(), selection, |range| {
                (
                    range.from(),
                    range.to(),
                    Some(contents.repeat(count).as_str().into()),
                )
            });

            doc.apply(&transaction, view.id);
            doc.append_changes_to_history(view);
        }
        Err(e) => return Err(e.context("Couldn't get system clipboard contents")),
    }

    exit_select_mode(cx);
    Ok(())
}

fn replace_selections_with_clipboard(cx: &mut Context) {
    let _ = replace_selections_with_clipboard_impl(cx, ClipboardType::Clipboard);
}

fn replace_selections_with_primary_clipboard(cx: &mut Context) {
    let _ = replace_selections_with_clipboard_impl(cx, ClipboardType::Selection);
}

fn paste(cx: &mut Context, pos: Paste) {
    let count = cx.count();
    let reg_name = cx.register.unwrap_or('"');
    let (view, doc) = current!(cx.editor);
    let registers = &mut cx.editor.registers;

    if let Some(values) = registers.read(reg_name) {
        paste_impl(values, doc, view, pos, count, cx.editor.mode);
    }
}

fn paste_after(cx: &mut Context) {
    paste(cx, Paste::After)
}

fn paste_before(cx: &mut Context) {
    paste(cx, Paste::Before)
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
}

fn format_selections(cx: &mut Context) {
    use helix_lsp::{lsp, util::range_to_lsp_range};

    let (view, doc) = current!(cx.editor);

    // via lsp if available
    // TODO: else via tree-sitter indentation calculations

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let ranges: Vec<lsp::Range> = doc
        .selection(view.id)
        .iter()
        .map(|range| range_to_lsp_range(doc.text(), *range, language_server.offset_encoding()))
        .collect();

    if ranges.len() != 1 {
        cx.editor
            .set_error("format_selections only supports a single selection for now");
        return;
    }

    // TODO: handle fails
    // TODO: concurrent map over all ranges

    let range = ranges[0];

    let request = match language_server.text_document_range_formatting(
        doc.identifier(),
        range,
        lsp::FormattingOptions::default(),
        None,
    ) {
        Some(future) => future,
        None => {
            cx.editor
                .set_error("Language server does not support range formatting");
            return;
        }
    };

    let edits = tokio::task::block_in_place(|| helix_lsp::block_on(request)).unwrap_or_default();

    let transaction = helix_lsp::util::generate_transaction_from_edits(
        doc.text(),
        edits,
        language_server.offset_encoding(),
    );

    doc.apply(&transaction, view.id);
}

fn join_selections_impl(cx: &mut Context, select_space: bool) {
    use movement::skip_while;
    let (view, doc) = current!(cx.editor);
    let text = doc.text();
    let slice = doc.text().slice(..);

    let mut changes = Vec::new();
    let fragment = Tendril::from(" ");

    for selection in doc.selection(view.id) {
        let (start, mut end) = selection.line_range(slice);
        if start == end {
            end = (end + 1).min(text.len_lines() - 1);
        }
        let lines = start..end;

        changes.reserve(lines.len());

        for line in lines {
            let start = line_end_char_index(&slice, line);
            let mut end = text.line_to_char(line + 1);
            end = skip_while(slice, end, |ch| matches!(ch, ' ' | '\t')).unwrap_or(end);

            // need to skip from start, not end
            let change = (start, end, Some(fragment.clone()));
            changes.push(change);
        }
    }

    // nothing to do, bail out early to avoid crashes later
    if changes.is_empty() {
        return;
    }

    changes.sort_unstable_by_key(|(from, _to, _text)| *from);
    changes.dedup();

    // TODO: joining multiple empty lines should be replaced by a single space.
    // need to merge change ranges that touch

    // select inserted spaces
    let transaction = if select_space {
        let ranges: SmallVec<_> = changes
            .iter()
            .scan(0, |offset, change| {
                let range = Range::point(change.0 - *offset);
                *offset += change.1 - change.0 - 1; // -1 because cursor is 0-sized
                Some(range)
            })
            .collect();
        let selection = Selection::new(ranges, 0);
        Transaction::change(doc.text(), changes.into_iter()).with_selection(selection)
    } else {
        Transaction::change(doc.text(), changes.into_iter())
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
        move |editor, regex, event| {
            let (view, doc) = current!(editor);
            if !matches!(event, PromptEvent::Update | PromptEvent::Validate) {
                return;
            }
            let text = doc.text().slice(..);

            if let Some(selection) =
                selection::keep_or_remove_matches(text, doc.selection(view.id), &regex, remove)
            {
                doc.set_selection(view.id, selection);
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
    use helix_lsp::{lsp, util::pos_to_lsp_pos};

    let (view, doc) = current!(cx.editor);

    let language_server = match doc.language_server() {
        Some(language_server) => language_server,
        None => return,
    };

    let offset_encoding = language_server.offset_encoding();
    let text = doc.text().slice(..);
    let cursor = doc.selection(view.id).primary().cursor(text);

    let pos = pos_to_lsp_pos(doc.text(), cursor, offset_encoding);

    let future = match language_server.completion(doc.identifier(), pos, None) {
        Some(future) => future,
        None => return,
    };

    // setup a channel that allows the request to be canceled
    let (tx, rx) = oneshot::channel();
    // set completion_request so that this request can be canceled
    // by setting completion_request, the old channel stored there is dropped
    // and the associated request is automatically dropped
    cx.editor.completion_request_handle = Some(tx);
    let future = async move {
        tokio::select! {
            biased;
            _ = rx => {
                Ok(serde_json::Value::Null)
            }
            res = future => {
                res
            }
        }
    };

    let trigger_offset = cursor;

    // TODO: trigger_offset should be the cursor offset but we also need a starting offset from where we want to apply
    // completion filtering. For example logger.te| should filter the initial suggestion list with "te".

    use helix_core::chars;
    let mut iter = text.chars_at(cursor);
    iter.reverse();
    let offset = iter.take_while(|ch| chars::char_is_word(*ch)).count();
    let start_offset = cursor.saturating_sub(offset);
    let savepoint = doc.savepoint(view);

    let trigger_doc = doc.id();
    let trigger_view = view.id;

    // FIXME: The commands Context can only have a single callback
    // which means it gets overwritten when executing keybindings
    // with multiple commands or macros. This would mean that completion
    // might be incorrectly applied when repeating the insertmode action
    //
    // TODO: to solve this either make cx.callback a Vec of callbacks or
    // alternatively move `last_insert` to `helix_view::Editor`
    cx.callback = Some(Box::new(
        move |compositor: &mut Compositor, _cx: &mut compositor::Context| {
            let ui = compositor.find::<ui::EditorView>().unwrap();
            ui.last_insert.1.push(InsertEvent::RequestCompletion);
        },
    ));

    cx.callback(
        future,
        move |editor, compositor, response: Option<lsp::CompletionResponse>| {
            let (view, doc) = current_ref!(editor);
            // check if the completion request is stale.
            //
            // Completions are completed asynchronously and therefore the user could
            //switch document/view or leave insert mode. In all of thoise cases the
            // completion should be discarded
            if editor.mode != Mode::Insert || view.id != trigger_view || doc.id() != trigger_doc {
                return;
            }

            let items = match response {
                Some(lsp::CompletionResponse::Array(items)) => items,
                // TODO: do something with is_incomplete
                Some(lsp::CompletionResponse::List(lsp::CompletionList {
                    is_incomplete: _is_incomplete,
                    items,
                })) => items,
                None => Vec::new(),
            };

            if items.is_empty() {
                // editor.set_error("No completion available");
                return;
            }
            let size = compositor.size();
            let ui = compositor.find::<ui::EditorView>().unwrap();
            let completion_area = ui.set_completion(
                editor,
                savepoint,
                items,
                offset_encoding,
                start_offset,
                trigger_offset,
                size,
            );
            let size = compositor.size();
            let signature_help_area = compositor
                .find_id::<Popup<SignatureHelp>>(SignatureHelp::ID)
                .map(|signature_help| signature_help.area(size, editor));
            // Delete the signature help popup if they intersect.
            if matches!((completion_area, signature_help_area),(Some(a), Some(b)) if a.intersects(b))
            {
                compositor.remove(SignatureHelp::ID);
            }
        },
    );
}

// comments
fn toggle_comments(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    let token = doc
        .language_config()
        .and_then(|lc| lc.comment_token.as_ref())
        .map(|tc| tc.as_ref());
    let transaction = comment::toggle_line_comments(doc.text(), doc.selection(view.id), token);

    doc.apply(&transaction, view.id);
    exit_select_mode(cx);
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

fn rotate_selection_contents(cx: &mut Context, direction: Direction) {
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
        match direction {
            Direction::Forward => chunk.rotate_right(1),
            Direction::Backward => chunk.rotate_left(1),
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
    rotate_selection_contents(cx, Direction::Forward)
}
fn rotate_selection_contents_backward(cx: &mut Context) {
    rotate_selection_contents(cx, Direction::Backward)
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
    motion(cx.editor);
    cx.editor.last_motion = Some(Motion(Box::new(motion)));
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
    motion(cx.editor);
    cx.editor.last_motion = Some(Motion(Box::new(motion)));
}

fn select_sibling_impl<F>(cx: &mut Context, sibling_fn: &'static F)
where
    F: Fn(Node) -> Option<Node>,
{
    let motion = |editor: &mut Editor| {
        let (view, doc) = current!(editor);

        if let Some(syntax) = doc.syntax() {
            let text = doc.text().slice(..);
            let current_selection = doc.selection(view.id);
            let selection =
                object::select_sibling(syntax, text, current_selection.clone(), sibling_fn);
            doc.set_selection(view.id, selection);
        }
    };
    motion(cx.editor);
    cx.editor.last_motion = Some(Motion(Box::new(motion)));
}

fn select_next_sibling(cx: &mut Context) {
    select_sibling_impl(cx, &|node| Node::next_sibling(&node))
}

fn select_prev_sibling(cx: &mut Context) {
    select_sibling_impl(cx, &|node| Node::prev_sibling(&node))
}

fn match_brackets(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);

    if let Some(syntax) = doc.syntax() {
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id).clone().transform(|range| {
            if let Some(pos) =
                match_brackets::find_matching_bracket_fuzzy(syntax, doc.text(), range.cursor(text))
            {
                range.put_cursor(text, pos, cx.editor.mode == Mode::Select)
            } else {
                range
            }
        });
        doc.set_selection(view.id, selection);
    }
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

// split helper, clear it later
fn split(cx: &mut Context, action: Action) {
    let (view, doc) = current!(cx.editor);
    let id = doc.id();
    let selection = doc.selection(view.id).clone();
    let offset = view.offset;

    cx.editor.switch(id, action);

    // match the selection in the previous view
    let (view, doc) = current!(cx.editor);
    doc.set_selection(view.id, selection);
    // match the view scroll offset (switch doesn't handle this fully
    // since the selection is only matched after the split)
    view.offset = offset;
}

fn hsplit(cx: &mut Context) {
    split(cx, Action::HorizontalSplit);
}

fn hsplit_new(cx: &mut Context) {
    cx.editor.new_file(Action::HorizontalSplit);
}

fn vsplit(cx: &mut Context) {
    split(cx, Action::VerticalSplit);
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
    cx.editor.autoinfo = Some(Info::from_registers(&cx.editor.registers));
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            cx.editor.autoinfo = None;
            cx.editor.selected_register = Some(ch);
        }
    })
}

fn insert_register(cx: &mut Context) {
    cx.editor.autoinfo = Some(Info::from_registers(&cx.editor.registers));
    cx.on_next_key(move |cx, event| {
        if let Some(ch) = event.char() {
            cx.editor.autoinfo = None;
            cx.register = Some(ch);
            paste(cx, Paste::Cursor);
        }
    })
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
    let annotations = view.text_annotations(doc, None);
    let pos = doc.selection(view.id).primary().cursor(doc_text);
    let pos =
        visual_offset_from_block(doc_text, view.offset.anchor, pos, &text_fmt, &annotations).0;

    view.offset.horizontal_offset = pos
        .col
        .saturating_sub((view.inner_area(doc).width as usize) / 2);
}

fn scroll_up(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Backward);
}

fn scroll_down(cx: &mut Context) {
    scroll(cx, cx.count(), Direction::Forward);
}

fn goto_ts_object_impl(cx: &mut Context, object: &'static str, direction: Direction) {
    let count = cx.count();
    let motion = move |editor: &mut Editor| {
        let (view, doc) = current!(editor);
        if let Some((lang_config, syntax)) = doc.language_config().zip(doc.syntax()) {
            let text = doc.text().slice(..);
            let root = syntax.tree().root_node();

            let selection = doc.selection(view.id).clone().transform(|range| {
                let new_range = movement::goto_treesitter_object(
                    text,
                    range,
                    object,
                    direction,
                    root,
                    lang_config,
                    count,
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
    motion(cx.editor);
    cx.editor.last_motion = Some(Motion(Box::new(motion)));
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
                let text = doc.text().slice(..);

                let textobject_treesitter = |obj_name: &str, range: Range| -> Range {
                    let (lang_config, syntax) = match doc.language_config().zip(doc.syntax()) {
                        Some(t) => t,
                        None => return range,
                    };
                    textobject::textobject_treesitter(
                        text,
                        range,
                        objtype,
                        obj_name,
                        syntax.tree().root_node(),
                        lang_config,
                        count,
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
                        'p' => textobject::textobject_paragraph(text, range, objtype, count),
                        'm' => textobject::textobject_pair_surround_closest(
                            text, range, objtype, count,
                        ),
                        'g' => textobject_change(range),
                        // TODO: cancel new ranges if inconsistent surround matches across lines
                        ch if !ch.is_ascii_alphanumeric() => {
                            textobject::textobject_pair_surround(text, range, objtype, ch, count)
                        }
                        _ => range,
                    }
                });
                doc.set_selection(view.id, selection);
            };
            textobject(cx.editor);
            cx.editor.last_motion = Some(Motion(Box::new(textobject)));
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
        ("m", "Closest surrounding pair"),
        (" ", "... or any character acting as a pair"),
    ];

    cx.editor.autoinfo = Some(Info::new(title, &help_text));
}

fn surround_add(cx: &mut Context) {
    cx.on_next_key(move |cx, event| {
        let (view, doc) = current!(cx.editor);
        // surround_len is the number of new characters being added.
        let (open, close, surround_len) = match event.char() {
            Some(ch) => {
                let (o, c) = surround::get_pair(ch);
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
    })
}

fn surround_replace(cx: &mut Context) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        let surround_ch = match event.char() {
            Some('m') => None, // m selects the closest surround pair
            Some(ch) => Some(ch),
            None => return,
        };
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id);

        let change_pos = match surround::get_surround_pos(text, selection, surround_ch, count) {
            Ok(c) => c,
            Err(err) => {
                cx.editor.set_error(err.to_string());
                return;
            }
        };

        cx.on_next_key(move |cx, event| {
            let (view, doc) = current!(cx.editor);
            let to = match event.char() {
                Some(to) => to,
                None => return,
            };
            let (open, close) = surround::get_pair(to);
            let transaction = Transaction::change(
                doc.text(),
                change_pos.iter().enumerate().map(|(i, &pos)| {
                    let mut t = Tendril::new();
                    t.push(if i % 2 == 0 { open } else { close });
                    (pos, pos + 1, Some(t))
                }),
            );
            doc.apply(&transaction, view.id);
            exit_select_mode(cx);
        });
    })
}

fn surround_delete(cx: &mut Context) {
    let count = cx.count();
    cx.on_next_key(move |cx, event| {
        let surround_ch = match event.char() {
            Some('m') => None, // m selects the closest surround pair
            Some(ch) => Some(ch),
            None => return,
        };
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        let selection = doc.selection(view.id);

        let change_pos = match surround::get_surround_pos(text, selection, surround_ch, count) {
            Ok(c) => c,
            Err(err) => {
                cx.editor.set_error(err.to_string());
                return;
            }
        };

        let transaction =
            Transaction::change(doc.text(), change_pos.into_iter().map(|p| (p, p + 1, None)));
        doc.apply(&transaction, view.id);
        exit_select_mode(cx);
    })
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
                let (_output, success) = match shell_impl(shell, input, Some(fragment.into())) {
                    Ok(result) => result,
                    Err(err) => {
                        cx.editor.set_error(err.to_string());
                        return;
                    }
                };

                // if the process exits successfully, keep the selection
                if success {
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

fn shell_impl(shell: &[String], cmd: &str, input: Option<Rope>) -> anyhow::Result<(Tendril, bool)> {
    tokio::task::block_in_place(|| helix_lsp::block_on(shell_impl_async(shell, cmd, input)))
}

async fn shell_impl_async(
    shell: &[String],
    cmd: &str,
    input: Option<Rope>,
) -> anyhow::Result<(Tendril, bool)> {
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
                helix_view::document::to_writer(&mut stdin, encoding::UTF_8, &input).await?;
            }
            Ok::<_, anyhow::Error>(())
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

    if !output.status.success() {
        if !output.stderr.is_empty() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            log::error!("Shell error: {}", err);
            bail!("Shell error: {}", err);
        }
        match output.status.code() {
            Some(exit_code) => bail!("Shell command failed: status {}", exit_code),
            None => bail!("Shell command failed"),
        }
    } else if !output.stderr.is_empty() {
        log::debug!(
            "Command printed to stderr: {}",
            String::from_utf8_lossy(&output.stderr).to_string()
        );
    }

    let str = std::str::from_utf8(&output.stdout)
        .map_err(|_| anyhow!("Process did not output valid UTF-8"))?;
    let tendril = Tendril::from(str);
    Ok((tendril, output.status.success()))
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
        let (output, success) = if let Some(output) = shell_output.as_ref() {
            (output.clone(), true)
        } else {
            let fragment = range.slice(text);
            match shell_impl(shell, cmd, pipe.then(|| fragment.into())) {
                Ok(result) => {
                    if !pipe {
                        shell_output = Some(result.0.clone());
                    }
                    result
                }
                Err(err) => {
                    cx.editor.set_error(err.to_string());
                    return;
                }
            }
        };

        if !success {
            cx.editor.set_error("Command failed");
            return;
        }

        let output_len = output.chars().count();

        let (from, to, deleted_len) = match behavior {
            ShellBehavior::Replace => (range.from(), range.to(), range.len()),
            ShellBehavior::Insert => (range.from(), range.from(), 0),
            ShellBehavior::Append => (range.to(), range.to(), 0),
            _ => (range.from(), range.from(), 0),
        };

        // These `usize`s cannot underflow because selection ranges cannot overlap.
        // Once the MSRV is 1.66.0 (mixed_integer_ops is stabilized), we can use checked
        // arithmetic to assert this.
        let anchor = (to as isize + offset - deleted_len as isize) as usize;
        let new_range = Range::new(anchor, anchor + output_len).with_direction(range.direction());
        ranges.push(new_range);
        offset = offset + output_len as isize - deleted_len as isize;

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
        ui::completers::none,
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
    signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
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
        cx.editor.registers.write(reg, vec![s]);
        cx.editor
            .set_status(format!("Recorded to register [{}]", reg));
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

    let keys: Vec<KeyEvent> = if let Some([keys_str]) = cx.editor.registers.read(reg) {
        match helix_view::input::parse_macro(keys_str) {
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
    cx.callback = Some(Box::new(move |compositor, cx| {
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

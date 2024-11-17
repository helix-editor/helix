use std::num::NonZeroUsize;

use helix_core::match_brackets::find_matching_bracket;
use helix_core::movement::{self, Movement};
use helix_view::document::Mode;

use crate::commands::{
    change_selection, delete_selection, extend_to_line_bounds,
    extend_word_impl, goto_line_end_impl, select_line_below, select_mode, yank, Context,
};

pub(crate) fn change_to_end_of_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_word_end);
    change_selection(cx);
}

pub(crate) fn change_to_end_of_long_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_end);
    change_selection(cx);
}

pub(crate) fn change_to_beginning_of_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_word_start);
    change_selection(cx);
}

pub(crate) fn change_to_beginning_of_long_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_long_word_start);
    change_selection(cx);
}

pub(crate) fn delete_to_end_of_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_word_end);
    delete_selection(cx);
}

pub(crate) fn delete_to_end_of_long_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_end);
    delete_selection(cx);
}

pub(crate) fn delete_to_beginning_of_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_word_start);
    delete_selection(cx);
}

pub(crate) fn delete_to_beginning_of_long_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_long_word_start);
    delete_selection(cx);
}

pub(crate) fn select_to_start_of_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_word_start);
    select_mode(cx);
}

pub(crate) fn select_to_start_of_long_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_start);
    select_mode(cx);
}

pub(crate) fn select_to_end_of_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_word_end);
    select_mode(cx);
}

pub(crate) fn select_to_end_of_long_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_end);
    select_mode(cx);
}

pub(crate) fn select_to_beginning_of_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_word_start);
    select_mode(cx);
}

pub(crate) fn select_to_beginning_of_long_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_long_word_start);
    select_mode(cx);
}

pub(crate) fn yank_to_end_of_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_word_end);
    yank(cx);
}

pub(crate) fn yank_to_end_of_long_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_next_long_word_end);
    yank(cx);
}

pub(crate) fn yank_to_beginning_of_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_word_start);
    yank(cx);
}

pub(crate) fn yank_to_beginning_of_long_word(cx: &mut Context) {
    extend_word_impl(cx, movement::move_prev_long_word_start);
    yank(cx);
}

pub(crate) fn change_line(cx: &mut Context) {
    let count = cx.count();
    extend_to_line_bounds(cx);
    if cx.count() > 1 {
        cx.count = NonZeroUsize::new(1);
        for _ in 0..count - 1 {
            select_line_below(cx);
            extend_to_line_bounds(cx);
        }
    }
    change_selection(cx);
}

pub(crate) fn delete_line(cx: &mut Context) {
    let count = cx.count();
    for _ in 0..count {
        extend_to_line_bounds(cx);
        delete_selection(cx);
    }
}

pub(crate) fn yank_line(cx: &mut Context) {
    let count = cx.count();
    extend_to_line_bounds(cx);
    if cx.count() > 1 {
        cx.count = NonZeroUsize::new(1);
        for _ in 0..count - 1 {
            select_line_below(cx);
            extend_to_line_bounds(cx);
        }
    }
    yank(cx);
}

pub(crate) fn change_to_end_of_line(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_impl(view, doc, Movement::Extend);
    change_selection(cx);
}

pub(crate) fn delete_to_end_of_line(cx: &mut Context) {
    let (view, doc) = current!(cx.editor);
    goto_line_end_impl(view, doc, Movement::Extend);
    delete_selection(cx);
}

pub(crate) fn goto_matching_pair(cx: &mut Context) {
    let mode = cx.editor.mode();
    let (view, doc) = current!(cx.editor);
    if let Some(syntax) = doc.syntax() {
        let text = doc.text().slice(..);
        let original_pos = doc.selection(view.id).primary().cursor(text);
        if let Some(pos) = find_matching_bracket(syntax, text, original_pos) {
            let selection = doc.selection(view.id).clone().transform(|mut range| {
                if mode == Mode::Select {
                    if pos > original_pos {
                        range.anchor = original_pos;
                        range.head = pos + 1;
                    } else {
                        range.anchor = original_pos + 1;
                        range.head = pos;
                    }
                } else {
                    range.anchor = pos;
                    range.head = pos + 1;
                }
                range
            });
            doc.set_selection(view.id, selection);
        }
    }
}

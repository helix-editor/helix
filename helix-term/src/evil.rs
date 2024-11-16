use helix_core::movement::{self};

use crate::commands::{Context, extend_word_impl, change_selection, select_mode, delete_selection, yank};

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
// pub(crate) fn change_inner_textobject(cx: &mut Context) {
//     change_textobject_inner(cx);
//     // change_selection(cx);
// }

// pub(crate) fn c_motion(cx: &mut Context) {
//     // TODO: count is reset to 1 before next key so we move it into the closure here.
//     // Would be nice to carry over.
//     // let count = cx.count();
//
//     // need to wait for next key
//     // TODO: should this be done by grapheme rather than char?  For example,
//     // we can't properly handle the line-ending CRLF case here in terms of char.
//     cx.on_next_key(move |cx, event| {
//         let ch = match event {
//             KeyEvent {
//                 code: KeyCode::Tab, ..
//             } => '\t',
//
//             KeyEvent {
//                 code: KeyCode::Char(ch),
//                 ..
//             } => ch,
//             _ => {
//                 return
//             },
//         };
//         // let motion = move |editor: &mut Editor| {
//             match ch {
//                 'w' => {
//                     extend_word_impl(cx, movement::move_next_word_end);
//                     change_selection(cx);
//                 },
//                 'W' => {
//                     extend_word_impl(cx, movement::move_next_long_word_end);
//                     change_selection(cx);
//                 },
//                 'i' => {
//                     change_textobject_inner(cx);
//                     change_selection(cx);
//                 },
//                 _ => return,
//             }
//         // };
//
//         // cx.editor.apply_motion(motion);
//     })
// }


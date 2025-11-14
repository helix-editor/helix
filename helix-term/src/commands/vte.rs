use super::Context;

pub fn toggle_terminal(cx: &mut Context) {
    cx.editor.terminals.toggle_terminal();
}

pub fn close_terminal(cx: &mut Context) {
    cx.editor.terminals.toggle_terminal();
}

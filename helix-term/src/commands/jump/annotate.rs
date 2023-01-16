use super::JumpAnnotation;
use crate::commands::Context;
use helix_core::chars::char_is_line_ending;
use helix_core::text_annotations::Overlay;
use helix_view::{input::KeyEvent, View};
use std::rc::Rc;

pub const JUMP_KEYS: &[u8] = b"etovxqpdygfblzhckisuran";

#[inline]
pub fn setup(ctx: &mut Context) {
    let (view, doc) = current!(ctx.editor);
    if doc.config.load().jump_mode.dim_during_jump {
        view.dimmed = true;
    }
    view.in_visual_jump_mode = true;
}

#[inline]
fn clear_dimming(view: &mut View) {
    view.dimmed = false;
    view.in_visual_jump_mode = false;
}

#[inline]
pub fn cleanup(ctx: &mut Context) {
    let mut view = view_mut!(ctx.editor);
    clear_dimming(view);
    view.visual_jump_labels[0] = Rc::new([]);
    view.visual_jump_labels[1] = Rc::new([]);
    view.visual_jump_labels[2] = Rc::new([]);
}

/// `annotations` should already be sorted by the `loc` attribute (= char_idx)
pub fn show_key_annotations_with_callback<F>(
    ctx: &mut Context,
    annotations: Vec<JumpAnnotation>,
    on_key_press_callback: F,
) where
    F: FnOnce(&mut Context, KeyEvent) + 'static,
{
    setup(ctx);
    let (view, doc) = current!(ctx.editor);
    let text = doc.text().slice(..);
    let mut overlays_single: Vec<Overlay> = Vec::new();
    let mut overlays_multi_first: Vec<Overlay> = Vec::new();
    let mut overlays_multi_rest: Vec<Overlay> = Vec::new();
    for jump in annotations.into_iter() {
        if jump.keys.len() == 1 {
            overlays_single.push(Overlay {
                char_idx: jump.loc,
                grapheme: jump.keys.into(),
            });
            continue;
        }
        overlays_multi_first.push(Overlay {
            char_idx: jump.loc,
            grapheme: jump.keys.chars().next().unwrap().to_string().into(),
        });
        for (i, c) in (1..jump.keys.len()).zip(jump.keys.chars().skip(1)) {
            let char_idx = jump.loc + i;
            let char = text.chars_at(char_idx).next().unwrap();
            // We shouldn't overlay anything on top of a line break. If we do, the next line will
            // crawl up and concatenate with the current line.
            if char_is_line_ending(char) {
                break;
            }
            overlays_multi_rest.push(Overlay {
                char_idx,
                grapheme: c.to_string().into(),
            });
        }
    }
    view.visual_jump_labels[0] = overlays_single.into();
    view.visual_jump_labels[1] = overlays_multi_first.into();
    view.visual_jump_labels[2] = overlays_multi_rest.into();
    ctx.on_next_key(on_key_press_callback);
}

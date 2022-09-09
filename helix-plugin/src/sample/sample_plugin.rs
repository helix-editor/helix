/*

This is just a silly plugin
It will render a hard coded watermark/ruler at 20 cols from the left

This is more a demo of one way to do "rust plugins"

*/
use helix_view::{
    graphics::Rect,
    theme::{Color, Style},
    Document, Editor, Theme, View,
};
use tui::buffer::Buffer as Surface;

use crate::plugin::BasePlugin;

pub struct SamplePlugin;

impl BasePlugin for SamplePlugin {
    fn name(&self) -> &'static str {
        "sample"
    }
    fn do_render_extras(
        &self,
        _: &Editor,
        _: &Document,
        _: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
    ) {
        let ruler_theme = theme
            .try_get("ui.virtual.ruler")
            .unwrap_or_else(|| Style::default().bg(Color::Red));

        //HACK: super stupid hard coded ruler at 20
        vec![20u16]
            .iter()
            .map(|ruler| viewport.clip_left(*ruler).with_width(1))
            .for_each(|area| surface.set_style(area, ruler_theme))
    }
}

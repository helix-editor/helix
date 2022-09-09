use helix_view::{graphics::Rect, Document, Editor, Theme, View};
use tui::buffer::Buffer as Surface;

/// Somewhere, all plugins needs to be added
/// Probably in helix-loader ?
/// This means that a new plugin PR;
///   will at least touch one additional file outside its folder
use crate::sample::sample_plugin::SamplePlugin;

pub struct Plugins {
    plugins: Vec<Box<dyn BasePlugin>>,
}

/// This is the interface to all the loaded plugins
/// For the demo, it simply creates a list every time it's needed
/// Should be cached/memoized after loading from config
/// The plan would be to have some sort of plugins = ["my", "list", "of", "plugins"]
/// Which would instantiate and report in to the common list of plugins
impl Plugins {
    pub fn new() -> Plugins {
        //TODO: load from config and cache in context?
        Plugins {
            plugins: vec![Box::new(SamplePlugin)],
        }
    }

    // the first example plugin feature, render some extras to the surface
    pub fn render_extras(
        editor: &Editor,
        doc: &Document,
        view: &View,
        viewport: Rect,
        surface: &mut Surface,
        theme: &Theme,
    ) {
        Plugins::new()
            .plugins
            .iter()
            .for_each(|p| p.do_render_extras(editor, doc, view, viewport, surface, theme));
    }
}

impl Default for Plugins {
    fn default() -> Self {
        Self::new()
    }
}

/// The base trait for plugins
/// Every "hook" needs to be added here
pub trait BasePlugin {
    fn name(&self) -> &'static str;
    /// Hook in to rendering, ability to render extras
    /// API not at all "designed", simply
    fn do_render_extras(
        &self,
        _: &Editor,
        _: &Document,
        _: &View,
        _: Rect,
        _: &mut Surface,
        _: &Theme,
    ) {
        // Default to doing nothing
    }
}

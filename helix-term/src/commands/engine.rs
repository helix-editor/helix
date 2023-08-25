use helix_view::{document::Mode, input::KeyEvent};

use std::borrow::Cow;

use crate::{
    compositor,
    keymap::KeymapResult,
    ui::{self, PromptEvent},
};

use super::{indent, shell_impl, Context, MappableCommand, TYPABLE_COMMAND_LIST};

#[cfg(feature = "steel")]
mod components;

// TODO: Change this visibility to pub(crate) probably, and adjust the status line message to not refer to the one
// in the scheme module directly. Probably create some kind of object here to refer to instead

#[cfg(feature = "steel")]
pub mod scheme;

// For now, we will allow _one_ embedded scripting engine to live inside of helix.
// In theory, we could allow multiple, with some kind of hierarchy where we check
// each one, with some kind of precedence.
struct PluginEngine<T: PluginSystem>(T);

// This is where we can configure our system to use the correct one
#[cfg(feature = "steel")]
static PLUGIN_SYSTEM: PluginEngine<scheme::SteelScriptingEngine> =
    PluginEngine(scheme::SteelScriptingEngine);

#[cfg(not(feature = "steel"))]
static PLUGIN_SYSTEM: PluginEngine<NoEngine> = PluginEngine(NoEngine);

enum PluginSystemTypes {
    None,
    Steel,
}

// The order in which the plugins will be evaluated against - if we wanted to include, lets say `rhai`,
// we would have to
static PLUGIN_PRECEDENCE: &[PluginSystemTypes] = &[PluginSystemTypes::Steel];

pub struct NoEngine;

// This will be the boundary layer between the editor and the engine.
pub struct ScriptingEngine;

impl ScriptingEngine {
    pub fn initialize() {
        PLUGIN_SYSTEM.0.initialize();
    }

    pub fn run_initialization_script(cx: &mut Context) {
        PLUGIN_SYSTEM.0.run_initialization_script(cx);
    }

    pub fn handle_keymap_event(
        editor: &mut ui::EditorView,
        mode: Mode,
        cxt: &mut Context,
        event: KeyEvent,
    ) -> KeymapResult {
        PLUGIN_SYSTEM
            .0
            .handle_keymap_event(editor, mode, cxt, event)
    }

    pub fn call_function_if_global_exists(
        cx: &mut Context,
        name: &str,
        args: Vec<Cow<str>>,
    ) -> bool {
        PLUGIN_SYSTEM
            .0
            .call_function_if_global_exists(cx, name, args)
    }

    pub fn call_typed_command_if_global_exists<'a>(
        cx: &mut compositor::Context,
        input: &'a str,
        parts: &'a [&'a str],
        event: PromptEvent,
    ) -> bool {
        PLUGIN_SYSTEM
            .0
            .call_typed_command_if_global_exists(cx, input, parts, event)
    }

    pub fn get_doc_for_identifier(ident: &str) -> Option<String> {
        PLUGIN_SYSTEM.0.get_doc_for_identifier(ident)
    }

    pub fn fuzzy_match<'a>(
        fuzzy_matcher: &'a fuzzy_matcher::skim::SkimMatcherV2,
        input: &'a str,
    ) -> Vec<(String, i64)> {
        PLUGIN_SYSTEM.0.fuzzy_match(fuzzy_matcher, input)
    }

    pub fn is_exported(ident: &str) -> bool {
        PLUGIN_SYSTEM.0.is_exported(ident)
    }

    pub fn engine_get_doc(ident: &str) -> Option<String> {
        PLUGIN_SYSTEM.0.engine_get_doc(ident)
    }
}

impl PluginSystem for NoEngine {}

/// These methods are the main entry point for interaction with the rest of
/// the editor system.
pub trait PluginSystem {
    fn initialize(&self) {}

    fn run_initialization_script(&self, cx: &mut Context) {}

    fn handle_keymap_event(
        &self,
        editor: &mut ui::EditorView,
        mode: Mode,
        cxt: &mut Context,
        event: KeyEvent,
    ) -> KeymapResult {
        editor.keymaps.get(mode, event)
    }

    fn call_function_if_global_exists(
        &self,
        cx: &mut Context,
        name: &str,
        args: Vec<Cow<str>>,
    ) -> bool {
        false
    }

    fn call_typed_command_if_global_exists<'a>(
        &self,
        cx: &mut compositor::Context,
        input: &'a str,
        parts: &'a [&'a str],
        event: PromptEvent,
    ) -> bool {
        false
    }

    fn get_doc_for_identifier(&self, ident: &str) -> Option<String> {
        None
    }

    fn fuzzy_match<'a>(
        &self,
        fuzzy_matcher: &'a fuzzy_matcher::skim::SkimMatcherV2,
        input: &'a str,
    ) -> Vec<(String, i64)> {
        Vec::new()
    }

    fn is_exported(&self, ident: &str) -> bool {
        false
    }

    fn engine_get_doc(&self, ident: &str) -> Option<String> {
        None
    }
}

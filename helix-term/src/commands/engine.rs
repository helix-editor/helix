use helix_view::{document::Mode, input::KeyEvent};

use std::{borrow::Cow, collections::HashMap};

use crate::{
    compositor,
    keymap::{KeyTrie, KeymapResult},
    ui::{self, PromptEvent},
};

use super::{shell_impl, Context, MappableCommand, TYPABLE_COMMAND_LIST};

#[cfg(feature = "steel")]
mod components;

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
/// The default plugin system used ends up with no ops for all of the behavior.
static PLUGIN_SYSTEM: PluginEngine<NoEngine> = PluginEngine(NoEngine);

// enum PluginSystemTypes {
//     None,
//     Steel,
// }

// The order in which the plugins will be evaluated against - if we wanted to include, lets say `rhai`,
// we would have to order the precedence for searching for exported commands, or somehow merge them?
// static PLUGIN_PRECEDENCE: &[PluginSystemTypes] = &[PluginSystemTypes::Steel];

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

    pub fn get_keybindings() -> Option<HashMap<Mode, KeyTrie>> {
        PLUGIN_SYSTEM.0.get_keybindings()
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
}

impl PluginSystem for NoEngine {}

/// These methods are the main entry point for interaction with the rest of
/// the editor system.
pub trait PluginSystem {
    /// If any initialization needs to happen prior to the initialization script being run,
    /// this is done here. This is run before the context is available.
    fn initialize(&self) {}

    /// Post initialization, once the context is available. This means you should be able to
    /// run anything here that could modify the context before the main editor is available.
    fn run_initialization_script(&self, _cx: &mut Context) {}

    /// Fetch the keybindings so that these can be loaded in to the keybinding map. These are
    /// keybindings that overwrite the default ones.
    fn get_keybindings(&self) -> Option<HashMap<Mode, KeyTrie>> {
        None
    }

    /// Allow the engine to directly handle a keymap event. This is some of the tightest integration
    /// with the engine, directly intercepting any keymap events. By default, this just delegates to the
    /// editors default keybindings.
    fn handle_keymap_event(
        &self,
        editor: &mut ui::EditorView,
        mode: Mode,
        _cxt: &mut Context,
        event: KeyEvent,
    ) -> KeymapResult {
        editor.keymaps.get(mode, event)
    }

    /// This attempts to call a function in the engine with the name `name` using the args `args`. The context
    /// is available here. Returns a bool indicating whether the function exists or not.
    fn call_function_if_global_exists(
        &self,
        _cx: &mut Context,
        _name: &str,
        _args: Vec<Cow<str>>,
    ) -> bool {
        false
    }

    /// This is explicitly for calling a function via the typed command interface, e.g. `:vsplit`. The context here
    /// that is available is more limited than the context available in `call_function_if_global_exists`. This also
    /// gives the ability to handle in progress commands with `PromptEvent`.
    fn call_typed_command_if_global_exists<'a>(
        &self,
        _cx: &mut compositor::Context,
        _input: &'a str,
        _parts: &'a [&'a str],
        _event: PromptEvent,
    ) -> bool {
        false
    }

    /// Given an identifier, extract the documentation from the engine.
    fn get_doc_for_identifier(&self, _ident: &str) -> Option<String> {
        None
    }

    /// Fuzzy match the input against the fuzzy matcher, used for handling completions on typed commands
    fn fuzzy_match<'a>(
        &self,
        _fuzzy_matcher: &'a fuzzy_matcher::skim::SkimMatcherV2,
        _input: &'a str,
    ) -> Vec<(String, i64)> {
        Vec::new()
    }
}

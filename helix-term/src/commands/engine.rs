use arc_swap::{ArcSwap, ArcSwapAny};
use helix_core::syntax;
use helix_lsp::{jsonrpc, LanguageServerId};
use helix_view::{document::Mode, input::KeyEvent};

use std::{borrow::Cow, sync::Arc};

use crate::{
    compositor,
    config::Config,
    keymap::KeymapResult,
    ui::{self, PromptEvent},
};

use super::{Context, MappableCommand, TYPABLE_COMMAND_LIST};

#[cfg(feature = "steel")]
mod components;

#[cfg(feature = "steel")]
pub mod steel;

pub enum PluginSystemKind {
    None,
    #[cfg(feature = "steel")]
    Steel,
}

pub enum PluginSystemTypes {
    None(NoEngine),
    #[cfg(feature = "steel")]
    Steel(steel::SteelScriptingEngine),
}

// The order in which the plugins will be evaluated against - if we wanted to include, lets say `rhai`,
// we would have to order the precedence for searching for exported commands, or somehow merge them?
const PLUGIN_PRECEDENCE: &[PluginSystemTypes] = &[
    #[cfg(feature = "steel")]
    PluginSystemTypes::Steel(steel::SteelScriptingEngine),
    PluginSystemTypes::None(NoEngine),
];

pub struct NoEngine;

// This will be the boundary layer between the editor and the engine.
pub struct ScriptingEngine;

// Macro to automatically dispatch to hopefully get some inlining
macro_rules! manual_dispatch {
    ($kind:expr, $raw:tt ($($args:expr),* $(,)?) ) => {
        match $kind {
            PluginSystemTypes::None(n) => n.$raw($($args),*),
            #[cfg(feature = "steel")]
            PluginSystemTypes::Steel(s) => s.$raw($($args),*),
        }
    };
}

impl ScriptingEngine {
    pub fn initialize() {
        for kind in PLUGIN_PRECEDENCE {
            manual_dispatch!(kind, initialize())
        }
    }

    pub fn run_initialization_script(
        cx: &mut Context,
        configuration: Arc<ArcSwapAny<Arc<Config>>>,
        language_configuration: Arc<ArcSwap<syntax::Loader>>,
    ) {
        for kind in PLUGIN_PRECEDENCE {
            manual_dispatch!(
                kind,
                run_initialization_script(
                    cx,
                    configuration.clone(),
                    language_configuration.clone()
                )
            )
        }
    }

    pub fn handle_keymap_event(
        editor: &mut ui::EditorView,
        mode: Mode,
        cxt: &mut Context,
        event: KeyEvent,
    ) -> Option<KeymapResult> {
        for kind in PLUGIN_PRECEDENCE {
            let res = manual_dispatch!(kind, handle_keymap_event(editor, mode, cxt, event));

            if res.is_some() {
                return res;
            }
        }

        None
    }

    pub fn call_function_by_name(cx: &mut Context, name: &str, args: Vec<Cow<str>>) -> bool {
        for kind in PLUGIN_PRECEDENCE {
            if manual_dispatch!(kind, call_function_by_name(cx, name, &args)) {
                return true;
            }
        }

        false
    }

    pub fn call_typed_command<'a>(
        cx: &mut compositor::Context,
        command: &'a str,
        parts: &'a [&'a str],
        event: PromptEvent,
    ) -> bool {
        for kind in PLUGIN_PRECEDENCE {
            if manual_dispatch!(kind, call_typed_command(cx, command, parts, event)) {
                return true;
            }
        }

        false
    }

    pub fn get_doc_for_identifier(ident: &str) -> Option<String> {
        for kind in PLUGIN_PRECEDENCE {
            let doc = manual_dispatch!(kind, get_doc_for_identifier(ident));

            if doc.is_some() {
                return doc;
            }
        }

        None
    }

    pub fn available_commands<'a>() -> Vec<Cow<'a, str>> {
        PLUGIN_PRECEDENCE
            .iter()
            .flat_map(|kind| manual_dispatch!(kind, available_commands()))
            .collect()
    }

    pub fn handle_lsp_notification(
        cx: &mut compositor::Context,
        server_id: LanguageServerId,
        event_name: String,
        params: jsonrpc::Params,
    ) {
        for kind in PLUGIN_PRECEDENCE {
            if manual_dispatch!(
                kind,
                // TODO: Get rid of these clones!
                handle_lsp_notification(cx, server_id, event_name.clone(), params.clone())
            ) {
                return;
            }
        }
    }

    pub fn generate_sources() {
        for kind in PLUGIN_PRECEDENCE {
            manual_dispatch!(kind, generate_sources())
        }
    }
}

impl PluginSystem for NoEngine {
    fn engine_name(&self) -> PluginSystemKind {
        PluginSystemKind::None
    }
}

/// These methods are the main entry point for interaction with the rest of
/// the editor system.
pub trait PluginSystem {
    /// If any initialization needs to happen prior to the initialization script being run,
    /// this is done here. This is run before the context is available.
    fn initialize(&self) {}

    #[allow(unused)]
    fn engine_name(&self) -> PluginSystemKind;

    /// Post initialization, once the context is available. This means you should be able to
    /// run anything here that could modify the context before the main editor is available.
    fn run_initialization_script(
        &self,
        _cx: &mut Context,
        _configuration: Arc<ArcSwapAny<Arc<Config>>>,
        _language_configuration: Arc<ArcSwap<syntax::Loader>>,
    ) {
    }

    /// Allow the engine to directly handle a keymap event. This is some of the tightest integration
    /// with the engine, directly intercepting any keymap events. By default, this just delegates to the
    /// editors default keybindings.
    #[inline(always)]
    fn handle_keymap_event(
        &self,
        _editor: &mut ui::EditorView,
        _mode: Mode,
        _cxt: &mut Context,
        _event: KeyEvent,
    ) -> Option<KeymapResult> {
        None
    }

    /// This attempts to call a function in the engine with the name `name` using the args `args`. The context
    /// is available here. Returns a bool indicating whether the function exists or not.
    #[inline(always)]
    fn call_function_by_name(&self, _cx: &mut Context, _name: &str, _args: &[Cow<str>]) -> bool {
        false
    }

    /// This is explicitly for calling a function via the typed command interface, e.g. `:vsplit`. The context here
    /// that is available is more limited than the context available in `call_function_if_global_exists`. This also
    /// gives the ability to handle in progress commands with `PromptEvent`.
    #[inline(always)]
    fn call_typed_command<'a>(
        &self,
        _cx: &mut compositor::Context,
        _input: &'a str,
        _parts: &'a [&'a str],
        _event: PromptEvent,
    ) -> bool {
        false
    }

    /// Call into the scripting engine to handle an unhandled LSP notification, sent from the server
    /// to the client.
    #[inline(always)]
    fn handle_lsp_notification(
        &self,
        _cx: &mut compositor::Context,
        _server_id: LanguageServerId,
        _event_name: String,
        _params: jsonrpc::Params,
    ) -> bool {
        false
    }

    /// Given an identifier, extract the documentation from the engine.
    #[inline(always)]
    fn get_doc_for_identifier(&self, _ident: &str) -> Option<String> {
        None
    }

    /// Fuzzy match the input against the fuzzy matcher, used for handling completions on typed commands
    #[inline(always)]
    fn available_commands<'a>(&self) -> Vec<Cow<'a, str>> {
        Vec::new()
    }

    fn generate_sources(&self) {}
}

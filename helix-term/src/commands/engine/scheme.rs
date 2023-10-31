use helix_core::{
    extensions::steel_implementations::{rope_module, SteelRopeSlice},
    graphemes,
    path::expand_tilde,
    regex::Regex,
    shellwords::Shellwords,
    syntax::Configuration,
    Range, Selection, Tendril,
};
use helix_event::register_hook;
use helix_loader::merge_toml_values;
use helix_lsp::lsp::CompletionItem;
use helix_view::{
    document::Mode,
    editor::{Action, ConfigEvent},
    extension::document_id_to_usize,
    input::KeyEvent,
    Document, DocumentId, Editor, Theme,
};
use once_cell::sync::Lazy;
use serde_json::Value;
use steel::{
    gc::unsafe_erased_pointers::CustomReference,
    rerrs::ErrorKind,
    rvals::{as_underlying_type, AsRefMutSteelValFromRef, FromSteelVal, IntoSteelVal},
    steel_vm::{engine::Engine, register_fn::RegisterFn},
    steelerr, SteelErr, SteelVal,
};

use std::{borrow::Cow, cell::RefCell, collections::HashMap, ops::Deref, path::PathBuf, rc::Rc};
use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use steel::{rvals::Custom, steel_vm::builtin::BuiltInModule};

use crate::{
    commands::insert,
    compositor::{self, Component, Compositor},
    config::Config,
    events::{OnModeSwitch, PostCommand, PostInsertChar},
    job::{self, Callback},
    keymap::{self, merge_keys, KeyTrie, KeymapResult},
    ui::{self, Popup, Prompt, PromptEvent},
};

use components::SteelDynamicComponent;

use super::{components, shell_impl, Context, MappableCommand, TYPABLE_COMMAND_LIST};
use insert::{insert_char, insert_string};

thread_local! {
    pub static ENGINE: std::rc::Rc<std::cell::RefCell<steel::steel_vm::engine::Engine>> = configure_engine();
}

enum CoreModules {
    Document(DocumentApi),
    Editor(EditorApi),
    Component(ComponentApi),
    TypedCommands(TypedCommandsApi),
    StaticCommands(StaticCommandsApi),
}

// APIs / Modules that need to be accepted by the plugin system
// Without these, the core functionality cannot operate
pub struct DocumentApi;
pub struct EditorApi;
pub struct ComponentApi;
pub struct TypedCommandsApi;
pub struct StaticCommandsApi;

pub struct KeyMapApi {
    get_keymap: fn() -> EmbeddedKeyMap,
    default_keymap: fn() -> EmbeddedKeyMap,
    empty_keymap: fn() -> EmbeddedKeyMap,
    string_to_embedded_keymap: fn(String) -> EmbeddedKeyMap,
    merge_keybindings: fn(&mut EmbeddedKeyMap, EmbeddedKeyMap),
    is_keymap: fn(SteelVal) -> bool,
    deep_copy_keymap: fn(EmbeddedKeyMap) -> EmbeddedKeyMap,
}

impl KeyMapApi {
    fn new() -> Self {
        KeyMapApi {
            get_keymap,
            default_keymap,
            empty_keymap,
            string_to_embedded_keymap,
            merge_keybindings,
            is_keymap,
            deep_copy_keymap,
        }
    }
}

thread_local! {
    pub static BUFFER_OR_EXTENSION_KEYBINDING_MAP: SteelVal =
        SteelVal::boxed(SteelVal::empty_hashmap());

    pub static REVERSE_BUFFER_MAP: SteelVal =
        SteelVal::boxed(SteelVal::empty_hashmap());

    pub static GLOBAL_KEYBINDING_MAP: SteelVal = get_keymap().into_steelval().unwrap();

    static THEME_MAP: ThemeContainer = ThemeContainer::new();

    static LANGUAGE_CONFIGURATIONS: LanguageConfigurationContainer = LanguageConfigurationContainer::new();
}

// Any configurations that we'd like to overlay from the toml
struct LanguageConfigurationContainer {
    configuration: Rc<RefCell<Option<toml::Value>>>,
}

impl LanguageConfigurationContainer {
    fn new() -> Self {
        Self {
            configuration: Rc::new(RefCell::new(None)),
        }
    }
}

impl Custom for LanguageConfigurationContainer {}

impl LanguageConfigurationContainer {
    fn add_configuration(&self, config_as_string: String) -> Result<(), String> {
        let left = self
            .configuration
            .replace(Some(toml::Value::Boolean(false)));

        if let Some(left) = left {
            let right = serde_json::from_str(&config_as_string).map_err(|err| err.to_string());

            // panic!("{:#?}", right);

            match right {
                Ok(right) => {
                    self.configuration
                        .replace(Some(merge_toml_values(left, right, 3)));

                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            let right = serde_json::from_str(&config_as_string).map_err(|err| err.to_string())?;

            self.configuration.replace(Some(right));

            Ok(())
        }
    }

    fn as_language_configuration(&self) -> Option<Result<Configuration, toml::de::Error>> {
        if let Some(right) = self.configuration.borrow().clone() {
            let config = helix_loader::config::user_lang_config();

            let res = config
                .map(|left| merge_toml_values(left, right, 3))
                .and_then(|x| x.try_into());

            Some(res)
            // )
        } else {
            None
        }
    }

    fn get_language_configuration() -> Option<Result<Configuration, toml::de::Error>> {
        LANGUAGE_CONFIGURATIONS.with(|x| x.as_language_configuration())
    }
}

struct ThemeContainer {
    themes: Rc<RefCell<HashMap<String, Theme>>>,
}

impl Custom for ThemeContainer {}

impl ThemeContainer {
    fn new() -> Self {
        Self {
            themes: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn get(name: &str) -> Option<Theme> {
        THEME_MAP.with(|x| x.themes.borrow().get(name).cloned())
    }

    fn names() -> Vec<String> {
        THEME_MAP.with(|x| x.themes.borrow().keys().cloned().collect())
    }

    fn register(name: String, theme: Theme) {
        THEME_MAP.with(|x| x.themes.borrow_mut().insert(name, theme));
    }
}

fn load_language_configuration_api(engine: &mut Engine) {
    let mut module = BuiltInModule::new("helix/core/languages".to_string());

    module.register_fn(
        "register-language-configuration!",
        |language_configuration: String| -> Result<(), String> {
            LANGUAGE_CONFIGURATIONS.with(|x| x.add_configuration(language_configuration))
        },
    );

    module.register_fn("flush-configuration", refresh_language_configuration);

    engine.register_module(module);
}

fn load_theme_api(engine: &mut Engine) {
    let mut module = BuiltInModule::new("helix/core/themes");

    module.register_fn(
        "register-theme!",
        |name: String, theme_as_json_string: String| -> Result<(), String> {
            Ok(ThemeContainer::register(
                name,
                serde_json::from_str(&theme_as_json_string).map_err(|err| err.to_string())?,
            ))
        },
    );

    engine.register_module(module);
}

fn load_keymap_api(engine: &mut Engine, api: KeyMapApi) {
    let mut module = BuiltInModule::new("helix/core/keymaps");

    module.register_fn("helix-current-keymap", api.get_keymap);
    module.register_fn("helix-empty-keymap", api.empty_keymap);
    module.register_fn("helix-default-keymap", api.default_keymap);
    module.register_fn("helix-merge-keybindings", api.merge_keybindings);
    module.register_fn("helix-string->keymap", api.string_to_embedded_keymap);
    module.register_fn("keymap?", api.is_keymap);

    module.register_fn("helix-deep-copy-keymap", api.deep_copy_keymap);

    // Alternatively, could store these values in a steel module, like so:
    // let keymap_core_map = helix_loader::runtime_file(&PathBuf::from("steel").join("keymap.scm"));
    // let require_module = format!("(require {})", keymap_core_map.to_str().unwrap());
    // engine.run(&require_module).unwrap();

    // This should be associated with a corresponding scheme module to wrap this up
    module.register_value(
        "*buffer-or-extension-keybindings*",
        BUFFER_OR_EXTENSION_KEYBINDING_MAP.with(|x| x.clone()),
    );
    module.register_value(
        "*reverse-buffer-map*",
        REVERSE_BUFFER_MAP.with(|x| x.clone()),
    );

    module.register_value(
        "*global-keybinding-map*",
        GLOBAL_KEYBINDING_MAP.with(|x| x.clone()),
    );

    engine.register_module(module);
}

fn load_static_commands(engine: &mut Engine) {
    let mut module = BuiltInModule::new("helix/core/static");

    for command in TYPABLE_COMMAND_LIST {
        let func = |cx: &mut Context| {
            let mut cx = compositor::Context {
                editor: cx.editor,
                scroll: None,
                jobs: cx.jobs,
            };

            (command.fun)(&mut cx, &[], PromptEvent::Validate)
        };

        module.register_fn(command.name, func);
    }

    // Register everything in the static command list as well
    // These just accept the context, no arguments
    for command in MappableCommand::STATIC_COMMAND_LIST {
        if let MappableCommand::Static { name, fun, .. } = command {
            module.register_fn(name, fun);
        }
    }

    // Adhoc static commands that probably needs evaluating
    module.register_fn("insert_char", insert_char);
    module.register_fn("insert_string", insert_string);
    module.register_fn("current_selection", get_selection);
    module.register_fn("current-highlighted-text!", get_highlighted_text);
    module.register_fn("get-current-line-number", current_line_number);
    module.register_fn("current-selection-object", current_selection);
    module.register_fn("set-current-selection-object!", set_selection);
    module.register_fn("run-in-engine!", run_in_engine);
    module.register_fn("get-helix-scm-path", get_helix_scm_path);
    module.register_fn("get-init-scm-path", get_init_scm_path);
    module.register_fn("get-helix-cwd", get_helix_cwd);
    module.register_fn("search-in-directory", search_in_directory);
    module.register_fn("regex-selection", regex_selection);
    module.register_fn("replace-selection-with", replace_selection);
    module.register_fn("show-completion-prompt-with", show_completion_prompt);
    module.register_fn("move-window-far-left", move_window_to_the_left);
    module.register_fn("move-window-far-right", move_window_to_the_right);

    module.register_fn("block-on-shell-command", run_shell_command_text);

    module.register_fn("cx->current-file", current_path);

    engine.register_module(module);
}

fn load_typed_commands(engine: &mut Engine) {
    let mut module = BuiltInModule::new("helix/core/typable".to_string());

    {
        let func = |cx: &mut Context, args: &[Cow<str>], event: PromptEvent| {
            let mut cx = compositor::Context {
                editor: cx.editor,
                scroll: None,
                jobs: cx.jobs,
            };

            set_options(&mut cx, args, event)
        };

        module.register_fn("set-options", func);
    }

    module.register_value(
        "PromptEvent::Validate",
        PromptEvent::Validate.into_steelval().unwrap(),
    );
    module.register_value(
        "PromptEvent::Update",
        PromptEvent::Update.into_steelval().unwrap(),
    );

    // Register everything in the typable command list. Now these are all available
    for command in TYPABLE_COMMAND_LIST {
        let func = |cx: &mut Context, args: &[Cow<str>], event: PromptEvent| {
            let mut cx = compositor::Context {
                editor: cx.editor,
                scroll: None,
                jobs: cx.jobs,
            };

            (command.fun)(&mut cx, args, event)
        };

        module.register_fn(command.name, func);
    }

    engine.register_module(module);
}

fn load_editor_api(engine: &mut Engine, _api: EditorApi) {
    let mut module = BuiltInModule::new("helix/core/editor");

    RegisterFn::<
        _,
        steel::steel_vm::register_fn::MarkerWrapper7<(
            Context<'_>,
            helix_view::Editor,
            helix_view::Editor,
            Context<'static>,
        )>,
        helix_view::Editor,
    >::register_fn(&mut module, "cx-editor!", get_editor);

    module.register_fn("set-scratch-buffer-name!", set_scratch_buffer_name);

    module.register_fn("editor-focus", current_focus);
    module.register_fn("editor->doc-id", get_document_id);
    module.register_fn("doc-id->usize", document_id_to_usize);
    module.register_fn("editor-switch!", switch);
    module.register_fn("editor-set-focus!", Editor::focus);
    module.register_fn("editor-mode", editor_get_mode);
    module.register_fn("editor-set-mode!", editor_set_mode);
    module.register_fn("editor-doc-in-view?", is_document_in_view);

    // TODO: These are some horrendous type annotations, however... they do work.
    // If the type annotations are a bit more ergonomic, we might be able to get away with this
    // (i.e. if they're sensible enough)
    RegisterFn::<
        _,
        steel::steel_vm::register_fn::MarkerWrapper8<(
            helix_view::Editor,
            DocumentId,
            Document,
            Document,
            helix_view::Editor,
        )>,
        Document,
    >::register_fn(&mut module, "editor->get-document", get_document);

    // Check if the doc exists first
    module.register_fn("editor-doc-exists?", document_exists);
    module.register_fn("Document-path", document_path);
    module.register_fn("Document-focused-at", document_focused_at);
    module.register_fn("editor-all-documents", editor_all_documents);

    module.register_fn("helix.context?", is_context);
    module.register_type::<DocumentId>("DocumentId?");

    module.register_fn("editor-cursor", Editor::cursor);

    module.register_fn("cx->cursor", |cx: &mut Context| cx.editor.cursor());

    // TODO:
    // Position related functions. These probably should be defined alongside the actual impl for Custom in the core crate
    module.register_fn("Position::new", helix_core::Position::new);
    module.register_fn("Position::default", helix_core::Position::default);
    module.register_fn("Position-row", |position: helix_core::Position| {
        position.row
    });

    module.register_fn("cx->themes", get_themes);

    engine.register_module(module);
}

fn load_document_api(engine: &mut Engine, api: DocumentApi) {
    todo!("Decide what should go in the document API!")
}

fn load_component_api(engine: &mut Engine, api: ComponentApi) {
    todo!("Decide what should go in the component API")
}

pub struct SteelScriptingEngine;

impl super::PluginSystem for SteelScriptingEngine {
    fn initialize(&self) {
        initialize_engine();
    }

    fn engine_name(&self) -> super::PluginSystemKind {
        super::PluginSystemKind::Steel
    }

    fn run_initialization_script(&self, cx: &mut Context) {
        run_initialization_script(cx);
    }

    fn handle_keymap_event(
        &self,
        editor: &mut ui::EditorView,
        mode: Mode,
        cxt: &mut Context,
        event: KeyEvent,
    ) -> Option<KeymapResult> {
        SteelScriptingEngine::get_keymap_for_extension(cxt).and_then(|map| {
            if let steel::SteelVal::Custom(inner) = map {
                if let Some(underlying) =
                    steel::rvals::as_underlying_type::<EmbeddedKeyMap>(inner.borrow().as_ref())
                {
                    return Some(editor.keymaps.get_with_map(&underlying.0, mode, event));
                }
            }

            None
        })
    }

    fn call_function_if_global_exists(
        &self,
        cx: &mut Context,
        name: &str,
        args: &[Cow<str>],
    ) -> bool {
        if ENGINE.with(|x| x.borrow().global_exists(name)) {
            let mut args = args
                .iter()
                .map(|x| x.clone().into_steelval().unwrap())
                .collect::<Vec<_>>();

            if let Err(e) = ENGINE.with(|x| {
                let mut guard = x.borrow_mut();

                {
                    guard.with_mut_reference::<Context, Context>(cx).consume(
                        move |engine, mut arguments| {
                            arguments.append(&mut args);

                            engine.call_function_by_name_with_args(name, arguments)
                        },
                    )
                }
            }) {
                cx.editor.set_error(format!("{}", e));
            }
            true
        } else {
            false
        }
    }

    fn call_typed_command_if_global_exists<'a>(
        &self,
        cx: &mut compositor::Context,
        input: &'a str,
        parts: &'a [&'a str],
        event: PromptEvent,
    ) -> bool {
        if ENGINE.with(|x| x.borrow().global_exists(parts[0])) {
            let shellwords = Shellwords::from(input);
            let args = shellwords.words();

            // We're finalizing the event - we actually want to call the function
            if event == PromptEvent::Validate {
                if let Err(e) = ENGINE.with(|x| {
                    let mut args = args[1..]
                        .iter()
                        .map(|x| x.clone().into_steelval().unwrap())
                        .collect::<Vec<_>>();

                    let mut guard = x.borrow_mut();

                    let res = {
                        let mut cx = Context {
                            register: None,
                            count: std::num::NonZeroUsize::new(1),
                            editor: cx.editor,
                            callback: Vec::new(),
                            on_next_key_callback: None,
                            jobs: cx.jobs,
                        };

                        guard
                            .with_mut_reference(&mut cx)
                            .consume(|engine, mut arguments| {
                                arguments.append(&mut args);

                                engine.call_function_by_name_with_args(&parts[0], arguments)
                            })
                    };

                    res
                }) {
                    compositor_present_error(cx, e);
                };
            }

            // Global exists
            true
        } else {
            // Global does not exist
            false
        }
    }

    fn get_doc_for_identifier(&self, ident: &str) -> Option<String> {
        ExportedIdentifiers::engine_get_doc(ident)
            .map(|v| v.into())
            .or_else(|| {
                if Self::is_exported(self, ident) {
                    Some("Run this plugin command!".into())
                } else {
                    None
                }
            })
    }

    fn available_commands<'a>(&self) -> Vec<Cow<'a, str>> {
        EXPORTED_IDENTIFIERS
            .identifiers
            .read()
            .unwrap()
            .iter()
            .map(|x| x.clone().into())
            .collect::<Vec<_>>()
    }

    fn load_theme(&self, name: &str) -> Option<helix_view::Theme> {
        ThemeContainer::get(name)
    }

    fn themes(&self) -> Option<Vec<String>> {
        let names = ThemeContainer::names();

        if !names.is_empty() {
            Some(names)
        } else {
            None
        }
    }

    fn load_language_configuration(&self) -> Option<Result<Configuration, toml::de::Error>> {
        LanguageConfigurationContainer::get_language_configuration()
    }
}

impl SteelScriptingEngine {
    fn is_exported(&self, ident: &str) -> bool {
        EXPORTED_IDENTIFIERS
            .identifiers
            .read()
            .unwrap()
            .contains(ident)
    }

    // Attempt to fetch the keymap for the extension
    fn get_keymap_for_extension<'a>(cx: &'a mut Context) -> Option<SteelVal> {
        // Get the currently activated extension, also need to check the
        // buffer type.
        let extension = {
            let current_focus = cx.editor.tree.focus;
            let view = cx.editor.tree.get(current_focus);
            let doc = &view.doc;
            let current_doc = cx.editor.documents.get(doc);

            current_doc
                .and_then(|x| x.path())
                .and_then(|x| x.extension())
                .and_then(|x| x.to_str())
        };

        let doc_id = {
            let current_focus = cx.editor.tree.focus;
            let view = cx.editor.tree.get(current_focus);
            let doc = &view.doc;

            doc
        };

        if let Some(extension) = extension {
            if let SteelVal::Boxed(boxed_map) =
                BUFFER_OR_EXTENSION_KEYBINDING_MAP.with(|x| x.clone())
            {
                if let SteelVal::HashMapV(map) = boxed_map.borrow().clone() {
                    if let Some(value) = map.get(&SteelVal::StringV(extension.into())) {
                        if let SteelVal::Custom(inner) = value {
                            if let Some(_) = steel::rvals::as_underlying_type::<EmbeddedKeyMap>(
                                inner.borrow().as_ref(),
                            ) {
                                return Some(value.clone());
                            }
                        }
                    }
                }
            }
        }

        if let SteelVal::Boxed(boxed_map) = REVERSE_BUFFER_MAP.with(|x| x.clone()) {
            if let SteelVal::HashMapV(map) = boxed_map.borrow().clone() {
                if let Some(label) = map.get(&SteelVal::IntV(document_id_to_usize(doc_id) as isize))
                {
                    if let SteelVal::Boxed(boxed_map) =
                        BUFFER_OR_EXTENSION_KEYBINDING_MAP.with(|x| x.clone())
                    {
                        if let SteelVal::HashMapV(map) = boxed_map.borrow().clone() {
                            if let Some(value) = map.get(label) {
                                if let SteelVal::Custom(inner) = value {
                                    if let Some(_) =
                                        steel::rvals::as_underlying_type::<EmbeddedKeyMap>(
                                            inner.borrow().as_ref(),
                                        )
                                    {
                                        return Some(value.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Refer to the global keybinding map for the rest
        Some(GLOBAL_KEYBINDING_MAP.with(|x| x.clone()))
    }
}

pub fn initialize_engine() {
    ENGINE.with(|x| x.borrow().globals().first().copied());
}

pub fn compositor_present_error(cx: &mut compositor::Context, e: SteelErr) {
    cx.editor.set_error(format!("{}", e));

    let backtrace = ENGINE.with(|x| x.borrow_mut().raise_error_to_string(e));

    let callback = async move {
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                if let Some(backtrace) = backtrace {
                    let contents = ui::Markdown::new(
                        format!("```\n{}\n```", backtrace),
                        editor.syn_loader.clone(),
                    );
                    ui::Text::new(format!("```\n{}\n```", backtrace));
                    let popup = Popup::new("engine", contents).position(Some(
                        helix_core::Position::new(editor.cursor().0.unwrap_or_default().row, 2),
                    ));
                    compositor.replace_or_push("engine", popup);
                }
            },
        ));
        Ok(call)
    };
    cx.jobs.callback(callback);
}

pub fn present_error_inside_engine_context(cx: &mut Context, engine: &mut Engine, e: SteelErr) {
    cx.editor.set_error(format!("{}", e));

    let backtrace = engine.raise_error_to_string(e);

    let callback = async move {
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                if let Some(backtrace) = backtrace {
                    let contents = ui::Markdown::new(
                        format!("```\n{}\n```", backtrace),
                        editor.syn_loader.clone(),
                    );
                    ui::Text::new(format!("```\n{}\n```", backtrace));
                    let popup = Popup::new("engine", contents).position(Some(
                        helix_core::Position::new(editor.cursor().0.unwrap_or_default().row, 2),
                    ));
                    compositor.replace_or_push("engine", popup);
                }
            },
        ));
        Ok(call)
    };
    cx.jobs.callback(callback);
}

pub fn present_error(cx: &mut Context, e: SteelErr) {
    cx.editor.set_error(format!("{}", e));

    let backtrace = ENGINE.with(|x| x.borrow_mut().raise_error_to_string(e));

    let callback = async move {
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                if let Some(backtrace) = backtrace {
                    let contents = ui::Markdown::new(
                        format!("```\n{}\n```", backtrace),
                        editor.syn_loader.clone(),
                    );
                    ui::Text::new(format!("```\n{}\n```", backtrace));
                    let popup = Popup::new("engine", contents).position(Some(
                        helix_core::Position::new(editor.cursor().0.unwrap_or_default().row, 2),
                    ));
                    compositor.replace_or_push("engine", popup);
                }
            },
        ));
        Ok(call)
    };
    cx.jobs.callback(callback);
}

// Key maps
#[derive(Clone, Debug)]
pub struct EmbeddedKeyMap(pub HashMap<Mode, KeyTrie>);
impl Custom for EmbeddedKeyMap {}

pub fn get_keymap() -> EmbeddedKeyMap {
    // Snapsnot current configuration for use in forking the keymap
    let keymap = Config::load_default().unwrap();

    // These are the actual mappings that we want
    let map = keymap.keys;

    EmbeddedKeyMap(map)
}

// Will deep copy a value by default when using a value type
pub fn deep_copy_keymap(copied: EmbeddedKeyMap) -> EmbeddedKeyMap {
    copied
}

// Base level - no configuration
pub fn default_keymap() -> EmbeddedKeyMap {
    EmbeddedKeyMap(keymap::default())
}

// Completely empty, allow for overriding
pub fn empty_keymap() -> EmbeddedKeyMap {
    EmbeddedKeyMap(HashMap::default())
}

pub fn string_to_embedded_keymap(value: String) -> EmbeddedKeyMap {
    EmbeddedKeyMap(serde_json::from_str(&value).unwrap())
}

pub fn merge_keybindings(left: &mut EmbeddedKeyMap, right: EmbeddedKeyMap) {
    merge_keys(&mut left.0, right.0)
}

pub fn is_keymap(keymap: SteelVal) -> bool {
    if let SteelVal::Custom(underlying) = keymap {
        as_underlying_type::<EmbeddedKeyMap>(underlying.borrow().as_ref()).is_some()
    } else {
        false
    }
}

pub fn helix_module_file() -> PathBuf {
    helix_loader::config_dir().join("helix.scm")
}

pub fn steel_init_file() -> PathBuf {
    helix_loader::config_dir().join("init.scm")
}

/// Run the initialization script located at `$helix_config/init.scm`
/// This runs the script in the global environment, and does _not_ load it as a module directly
fn run_initialization_script(cx: &mut Context) {
    log::info!("Loading init.scm...");

    let helix_module_path = helix_module_file();

    // TODO: Report the error from requiring the file!
    ENGINE.with(|engine| {
        let mut guard = engine.borrow_mut();

        let res = guard.run(&format!(
            r#"(require "{}")"#,
            helix_module_path.to_str().unwrap()
        ));

        // Present the error in the helix.scm loading
        if let Err(e) = res {
            present_error_inside_engine_context(cx, &mut guard, e);
            return;
        }

        if let Ok(module) = guard.get_module(helix_module_path) {
            if let steel::rvals::SteelVal::HashMapV(m) = module {
                let exported = m
                    .iter()
                    .filter(|(_, v)| v.is_function())
                    .map(|(k, _)| {
                        if let steel::rvals::SteelVal::SymbolV(s) = k {
                            s.to_string()
                        } else {
                            panic!("Found a non symbol!")
                        }
                    })
                    .collect::<HashSet<_>>();

                let docs = exported
                    .iter()
                    .filter_map(|x| {
                        if let Ok(value) = guard.run(&format!(
                            "(#%function-ptr-table-get #%function-ptr-table {})",
                            x
                        )) {
                            if let Some(SteelVal::StringV(doc)) = value.first() {
                                Some((x.to_string(), doc.to_string()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<HashMap<_, _>>();

                *EXPORTED_IDENTIFIERS.identifiers.write().unwrap() = exported;
                *EXPORTED_IDENTIFIERS.docs.write().unwrap() = docs;
            } else {
                present_error_inside_engine_context(
                    cx,
                    &mut guard,
                    SteelErr::new(
                        ErrorKind::Generic,
                        "Unable to parse exported identifiers from helix module!".to_string(),
                    ),
                );

                return;
            }
        }

        let helix_module_path = steel_init_file();

        // These contents need to be registered with the path?
        if let Ok(contents) = std::fs::read_to_string(&helix_module_path) {
            let res = guard.run_with_reference_from_path::<Context, Context>(
                cx,
                "*helix.cx*",
                &contents,
                helix_module_path,
            );

            match res {
                Ok(_) => {}
                Err(e) => present_error_inside_engine_context(cx, &mut guard, e),
            }

            log::info!("Finished loading init.scm!")
        } else {
            log::info!("No init.scm found, skipping loading.")
        }
    });
}

// pub static KEYBINDING_QUEUE: Lazy<SharedKeyBindingsEventQueue> =
//     Lazy::new(|| SharedKeyBindingsEventQueue::new());

pub static EXPORTED_IDENTIFIERS: Lazy<ExportedIdentifiers> =
    Lazy::new(|| ExportedIdentifiers::default());

impl Custom for PromptEvent {}

impl<'a> CustomReference for Context<'a> {}

steel::custom_reference!(Context<'a>);

fn get_editor<'a>(cx: &'a mut Context<'a>) -> &'a mut Editor {
    cx.editor
}

fn get_themes(cx: &mut Context) -> Vec<String> {
    ui::completers::theme(cx.editor, "")
        .into_iter()
        .map(|x| x.1.to_string())
        .collect()
}

/// A dynamic component, used for rendering thing

impl Custom for compositor::EventResult {}
impl FromSteelVal for compositor::EventResult {
    fn from_steelval(val: &SteelVal) -> steel::rvals::Result<Self> {
        match val {
            SteelVal::SymbolV(v) if v.as_str() == "EventResult::Ignored" => {
                Ok(compositor::EventResult::Ignored(None))
            }
            SteelVal::SymbolV(v) if v.as_str() == "EventResult::Consumed" => {
                Ok(compositor::EventResult::Consumed(None))
            }
            _ => Err(steel::SteelErr::new(
                steel::rerrs::ErrorKind::TypeMismatch,
                "Unable to convert value to event result".to_string(),
            )),
        }
    }
}

struct WrappedDynComponent {
    inner: Option<Box<dyn Component>>,
}

impl Custom for WrappedDynComponent {}

struct BoxDynComponent {
    inner: Box<dyn Component>,
}

impl BoxDynComponent {
    pub fn new(inner: Box<dyn Component>) -> Self {
        Self { inner }
    }
}

impl Component for BoxDynComponent {
    fn handle_event(
        &mut self,
        _event: &helix_view::input::Event,
        _ctx: &mut compositor::Context,
    ) -> compositor::EventResult {
        self.inner.handle_event(_event, _ctx)
    }

    fn should_update(&self) -> bool {
        self.inner.should_update()
    }

    fn cursor(
        &self,
        _area: helix_view::graphics::Rect,
        _ctx: &Editor,
    ) -> (
        Option<helix_core::Position>,
        helix_view::graphics::CursorKind,
    ) {
        self.inner.cursor(_area, _ctx)
    }

    fn required_size(&mut self, _viewport: (u16, u16)) -> Option<(u16, u16)> {
        self.inner.required_size(_viewport)
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn id(&self) -> Option<&'static str> {
        None
    }

    fn render(
        &mut self,
        area: helix_view::graphics::Rect,
        frame: &mut tui::buffer::Buffer,
        ctx: &mut compositor::Context,
    ) {
        self.inner.render(area, frame, ctx)
    }
}

// OnModeSwitch<'a, 'cx> { old_mode: Mode, new_mode: Mode, cx: &'a mut commands::Context<'cx> }
// PostInsertChar<'a, 'cx> { c: char, cx: &'a mut commands::Context<'cx> }
// PostCommand<'a, 'cx> { command: & 'a MappableCommand, cx: &'a mut commands::Context<'cx> }

#[derive(Debug, Clone, Copy)]
struct OnModeSwitchEvent {
    old_mode: Mode,
    new_mode: Mode,
}

impl Custom for OnModeSwitchEvent {}

// MappableCommands can be values too!
impl Custom for MappableCommand {}

fn register_hook(event_kind: String, function_name: String) -> steel::UnRecoverableResult {
    match event_kind.as_str() {
        "on-mode-switch" => {
            register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
                if ENGINE.with(|x| x.borrow().global_exists(&function_name)) {
                    if let Err(e) = ENGINE.with(|x| {
                        let mut guard = x.borrow_mut();

                        let minimized_event = OnModeSwitchEvent {
                            old_mode: event.old_mode,
                            new_mode: event.new_mode,
                        };

                        guard
                            .with_mut_reference(event.cx)
                            .consume(|engine, mut args| {
                                args.push(minimized_event.into_steelval().unwrap());
                                engine.call_function_by_name_with_args(&function_name, args)
                            })
                    }) {
                        event.cx.editor.set_error(format!("{}", e));
                    }
                }

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }
        "post-insert-char" => {
            register_hook!(move |event: &mut PostInsertChar<'_, '_>| {
                if ENGINE.with(|x| x.borrow().global_exists(&function_name)) {
                    if let Err(e) = ENGINE.with(|x| {
                        let mut guard = x.borrow_mut();

                        guard
                            .with_mut_reference(event.cx)
                            .consume(|engine, mut args| {
                                args.push(event.c.into());
                                engine.call_function_by_name_with_args(&function_name, args)
                            })
                    }) {
                        event.cx.editor.set_error(format!("{}", e));
                    }
                }

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }
        "post-command" => {
            register_hook!(move |event: &mut PostCommand<'_, '_>| {
                if ENGINE.with(|x| x.borrow().global_exists(&function_name)) {
                    if let Err(e) = ENGINE.with(|x| {
                        let mut guard = x.borrow_mut();

                        guard
                            .with_mut_reference(event.cx)
                            .consume(|engine, mut args| {
                                args.push(event.command.clone().into_steelval().unwrap());
                                engine.call_function_by_name_with_args(&function_name, args)
                            })
                    }) {
                        event.cx.editor.set_error(format!("{}", e));
                    }
                }

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }
        // Unimplemented!
        // "document-did-change" => {
        //     todo!()
        // }
        // "selection-did-change" => {
        //     todo!()
        // }
        _ => steelerr!(Generic => "Unable to register hook: Unknown event type: {}", event_kind)
            .into(),
    }
}

fn load_rope_api(engine: &mut Engine) {
    let mut rope_slice_module = rope_module();

    rope_slice_module.register_fn("document->slice", document_to_text);

    engine.register_module(rope_slice_module);
}

fn configure_engine() -> std::rc::Rc<std::cell::RefCell<steel::steel_vm::engine::Engine>> {
    let mut engine = steel::steel_vm::engine::Engine::new();

    log::info!("Loading engine!");

    // TODO: Load (require-builtin helix/core/editor) in more or less every file that needs it
    load_editor_api(&mut engine, EditorApi);
    load_typed_commands(&mut engine);
    load_static_commands(&mut engine);
    load_keymap_api(&mut engine, KeyMapApi::new());
    load_theme_api(&mut engine);
    load_rope_api(&mut engine);
    load_language_configuration_api(&mut engine);

    // Async context used for referencing the context?
    engine.register_value("*helix-async-context*", SteelVal::Void);

    // Include this?
    // engine.register_fn("call-function-in-context", call_function_in_external_engine);

    engine.register_fn("register-hook!", register_hook);

    engine.register_value("*context*", SteelVal::Void);

    engine.register_fn("hx.context?", |_: &mut Context| true);

    engine.register_fn("log::info!", |message: String| log::info!("{}", message));

    engine.register_fn("hx.custom-insert-newline", custom_insert_newline);
    engine.register_fn("hx.cx->pos", cx_pos_within_text);

    // Find the workspace
    engine.register_fn("helix-find-workspace", || {
        helix_core::find_workspace().0.to_str().unwrap().to_string()
    });

    // Get the current OS
    engine.register_fn("current-os!", || std::env::consts::OS);
    engine.register_fn("new-component!", SteelDynamicComponent::new_dyn);

    engine.register_fn("SteelDynamicComponent?", |object: SteelVal| {
        if let SteelVal::Custom(v) = object {
            if let Some(wrapped) = v.borrow().as_any_ref().downcast_ref::<BoxDynComponent>() {
                return wrapped.inner.as_any().is::<SteelDynamicComponent>();
            } else {
                false
            }
        } else {
            false
        }
    });

    engine.register_fn(
        "SteelDynamicComponent-state",
        SteelDynamicComponent::get_state,
    );
    engine.register_fn(
        "SteelDynamicComponent-render",
        SteelDynamicComponent::get_render,
    );
    engine.register_fn(
        "SteelDynamicComponent-handle-event",
        SteelDynamicComponent::get_handle_event,
    );
    engine.register_fn(
        "SteelDynamicComponent-should-update",
        SteelDynamicComponent::should_update,
    );
    engine.register_fn(
        "SteelDynamicComponent-cursor",
        SteelDynamicComponent::cursor,
    );
    engine.register_fn(
        "SteelDynamicComponent-required-size",
        SteelDynamicComponent::get_required_size,
    );

    // engine.register_fn("WrappedComponent", WrappedDynComponent::new)

    // engine.register_fn(
    //     "Popup::new",
    //     |contents: &mut WrappedDynComponent,
    //      position: helix_core::Position|
    //      -> WrappedDynComponent {
    //         let inner = contents.inner.take().unwrap(); // Panic, for now

    //         WrappedDynComponent {
    //             inner: Some(Box::new(
    //                 Popup::<BoxDynComponent>::new("popup", BoxDynComponent::new(inner))
    //                     .position(Some(position)),
    //             )),
    //         }
    //     },
    // );

    engine.register_fn(
        "Prompt::new",
        |prompt: String, callback_fn: SteelVal| -> WrappedDynComponent {
            let callback_fn_guard = callback_fn.as_rooted();

            let prompt = Prompt::new(
                prompt.into(),
                None,
                |_, _| Vec::new(),
                move |cx, input, prompt_event| {
                    log::info!("Calling dynamic prompt callback");

                    if prompt_event != PromptEvent::Validate {
                        return;
                    }

                    let mut ctx = Context {
                        register: None,
                        count: None,
                        editor: cx.editor,
                        callback: Vec::new(),
                        on_next_key_callback: None,
                        jobs: cx.jobs,
                    };

                    let cloned_func = callback_fn_guard.value();

                    if let Err(e) = ENGINE.with(|x| {
                        x.borrow_mut()
                            .with_mut_reference::<Context, Context>(&mut ctx)
                            .consume(move |engine, mut args| {
                                // Add the string as an argument to the callback
                                args.push(input.into_steelval().unwrap());

                                engine.call_function_with_args(cloned_func.clone(), args)
                            })
                    }) {
                        present_error(&mut ctx, e);
                    }
                },
            );

            WrappedDynComponent {
                inner: Some(Box::new(prompt)),
            }
        },
    );

    engine.register_fn(
        "Picker::new",
        |values: steel::List<String>| -> WrappedDynComponent {
            let picker = ui::Picker::new(
                Vec::new(),
                PathBuf::from(""),
                move |cx, path: &PathBuf, action| {
                    if let Err(e) = cx.editor.open(path, action) {
                        let err = if let Some(err) = e.source() {
                            format!("{}", err)
                        } else {
                            format!("unable to open \"{}\"", path.display())
                        };
                        cx.editor.set_error(err);
                    }
                },
            )
            .with_preview(|_editor, path| Some((path.clone().into(), None)));

            let injector = picker.injector();

            for file in values {
                if injector.push(PathBuf::from(file)).is_err() {
                    break;
                }
            }

            WrappedDynComponent {
                inner: Some(Box::new(ui::overlay::overlaid(picker))),
            }
        },
    );

    engine.register_fn("Component::Text", |contents: String| WrappedDynComponent {
        inner: Some(Box::new(crate::ui::Text::new(contents))),
    });

    // Separate this out into its own component module - This just lets us call the underlying
    // component, not sure if we can go from trait object -> trait object easily but we'll see!
    engine.register_fn(
        "Component::render",
        |t: &mut WrappedDynComponent,
         area: helix_view::graphics::Rect,
         frame: &mut tui::buffer::Buffer,
         ctx: &mut Context| {
            t.inner.as_mut().unwrap().render(
                area,
                frame,
                &mut compositor::Context {
                    jobs: ctx.jobs,
                    editor: ctx.editor,
                    scroll: None,
                },
            )
        },
    );

    engine.register_fn(
        "Component::handle-event",
        |s: &mut WrappedDynComponent, event: &helix_view::input::Event, ctx: &mut Context| {
            s.inner.as_mut().unwrap().handle_event(
                event,
                &mut compositor::Context {
                    jobs: ctx.jobs,
                    editor: ctx.editor,
                    scroll: None,
                },
            )
        },
    );

    engine.register_fn("Component::should-update", |s: &mut WrappedDynComponent| {
        s.inner.as_mut().unwrap().should_update()
    });

    engine.register_fn(
        "Component::cursor",
        |s: &WrappedDynComponent, area: helix_view::graphics::Rect, ctx: &Editor| {
            s.inner.as_ref().unwrap().cursor(area, ctx)
        },
    );

    engine.register_fn(
        "Component::required-size",
        |s: &mut WrappedDynComponent, viewport: (u16, u16)| {
            s.inner.as_mut().unwrap().required_size(viewport)
        },
    );

    // TODO: Load (require-builtin helix/core/editor) in more or less every file that needs it

    engine.register_fn("push-component!", push_component);
    engine.register_fn("enqueue-thread-local-callback", enqueue_command);
    engine.register_fn(
        "enqueue-thread-local-callback-with-delay",
        enqueue_command_with_delay,
    );

    engine.register_fn("helix-await-callback", await_value);

    // Create directory since we can't do that in the current state
    engine.register_fn("hx.create-directory", create_directory);

    std::rc::Rc::new(std::cell::RefCell::new(engine))
}

#[derive(Default, Debug)]
pub struct ExportedIdentifiers {
    identifiers: Arc<RwLock<HashSet<String>>>,
    docs: Arc<RwLock<HashMap<String, String>>>,
}

impl ExportedIdentifiers {
    pub(crate) fn engine_get_doc(ident: &str) -> Option<String> {
        EXPORTED_IDENTIFIERS
            .docs
            .read()
            .unwrap()
            .get(ident)
            .cloned()
    }
}

fn get_highlighted_text(cx: &mut Context) -> String {
    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text().slice(..);
    doc.selection(view.id).primary().slice(text).to_string()
}

fn current_selection(cx: &mut Context) -> Selection {
    let (view, doc) = current_ref!(cx.editor);
    doc.selection(view.id).clone()
}

fn set_selection(cx: &mut Context, selection: Selection) {
    let (view, doc) = current!(cx.editor);
    doc.set_selection(view.id, selection)
}

fn current_line_number(cx: &mut Context) -> usize {
    let (view, doc) = current_ref!(cx.editor);
    helix_core::coords_at_pos(
        doc.text().slice(..),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
    )
    .row
}

fn get_selection(cx: &mut Context) -> String {
    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text().slice(..);

    let grapheme_start = doc.selection(view.id).primary().cursor(text);
    let grapheme_end = graphemes::next_grapheme_boundary(text, grapheme_start);

    if grapheme_start == grapheme_end {
        return "".into();
    }

    let grapheme = text.slice(grapheme_start..grapheme_end).to_string();

    let printable = grapheme.chars().fold(String::new(), |mut s, c| {
        match c {
            '\0' => s.push_str("\\0"),
            '\t' => s.push_str("\\t"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            _ => s.push(c),
        }

        s
    });

    printable
}

fn run_in_engine(cx: &mut Context, arg: String) -> anyhow::Result<()> {
    let callback = async move {
        let output = ENGINE
            .with(|x| x.borrow_mut().run(&arg))
            .map(|x| format!("{:?}", x));

        let (output, success) = match output {
            Ok(v) => (Tendril::from(v), true),
            Err(e) => (Tendril::from(e.to_string()), false),
        };

        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                if !output.is_empty() {
                    let contents = ui::Markdown::new(
                        format!("```\n{}\n```", output),
                        editor.syn_loader.clone(),
                    );
                    let popup = Popup::new("engine", contents).position(Some(
                        helix_core::Position::new(editor.cursor().0.unwrap_or_default().row, 2),
                    ));
                    compositor.replace_or_push("engine", popup);
                }
                if success {
                    editor.set_status("Command succeeded");
                } else {
                    editor.set_error("Command failed");
                }
            },
        ));
        Ok(call)
    };
    cx.jobs.callback(callback);

    Ok(())
}

fn get_helix_scm_path() -> String {
    helix_module_file().to_str().unwrap().to_string()
}

fn get_init_scm_path() -> String {
    steel_init_file().to_str().unwrap().to_string()
}

/// Get the current path! See if this can be done _without_ this function?
// TODO:
fn current_path(cx: &mut Context) -> Option<String> {
    let current_focus = cx.editor.tree.focus;
    let view = cx.editor.tree.get(current_focus);
    let doc = &view.doc;
    // Lifetime of this needs to be tied to the existing document
    let current_doc = cx.editor.documents.get(doc);
    current_doc.and_then(|x| x.path().and_then(|x| x.to_str().map(|x| x.to_string())))
}

fn set_scratch_buffer_name(cx: &mut Context, name: String) {
    let current_focus = cx.editor.tree.focus;
    let view = cx.editor.tree.get(current_focus);
    let doc = &view.doc;
    // Lifetime of this needs to be tied to the existing document
    let current_doc = cx.editor.documents.get_mut(doc);

    if let Some(current_doc) = current_doc {
        current_doc.name = Some(name);
    }
}

// TODO: Use this over handing around the editor reference, probably
// fn cx_current_focus(cx: &mut Context) -> helix_view::ViewId {
//     cx.editor.tree.focus
// }

// TODO: Expose the below in a separate module, make things a bit more clear!
fn current_focus(editor: &mut Editor) -> helix_view::ViewId {
    editor.tree.focus
}

// Get the document id
fn get_document_id(editor: &mut Editor, view_id: helix_view::ViewId) -> DocumentId {
    editor.tree.get(view_id).doc
}

// Get the document from the document id - TODO: Add result type here
fn get_document(editor: &mut Editor, doc_id: DocumentId) -> &Document {
    editor.documents.get(&doc_id).unwrap()
}

fn document_to_text(doc: &Document) -> SteelRopeSlice {
    SteelRopeSlice::new(doc.text().clone())
}

fn is_document_in_view(editor: &mut Editor, doc_id: DocumentId) -> Option<helix_view::ViewId> {
    editor
        .tree
        .traverse()
        .find(|(_, v)| v.doc == doc_id)
        .map(|(id, _)| id)
}

fn document_exists(editor: &mut Editor, doc_id: DocumentId) -> bool {
    editor.documents.get(&doc_id).is_some()
}

fn document_path(doc: &Document) -> Option<String> {
    doc.path().and_then(|x| x.to_str()).map(|x| x.to_string())
}

// Get the time the document was focused
fn document_focused_at(doc: &Document) -> std::time::Instant {
    doc.focused_at
}

// Get all the editor documents
fn editor_all_documents(editor: &mut Editor) -> Vec<DocumentId> {
    editor.documents.keys().copied().collect()
}

fn switch(editor: &mut Editor, doc_id: DocumentId) {
    editor.switch(doc_id, Action::VerticalSplit)
}

fn editor_get_mode(editor: &mut Editor) -> Mode {
    editor.mode
}

fn editor_set_mode(editor: &mut Editor, mode: Mode) {
    editor.mode = mode
}

fn run_shell_command_text(
    cx: &mut Context,
    args: &[Cow<str>],
    _event: PromptEvent,
) -> anyhow::Result<String> {
    let shell = cx.editor.config().shell.clone();
    let args = args.join(" ");

    let (output, success) = shell_impl(&shell, &args, None)?;

    if success {
        Ok(output.to_string())
    } else {
        anyhow::bail!("Command failed!: {}", output.to_string())
    }
}

fn is_context(value: SteelVal) -> bool {
    Context::as_mut_ref_from_ref(&value).is_ok()
}

// Overlay the dynamic component, see what happens?
// Probably need to pin the values to this thread - wrap it in a shim which pins the value
// to this thread? - call methods on the thread local value?
fn push_component(cx: &mut Context, component: &mut WrappedDynComponent) {
    log::info!("Pushing dynamic component!");

    let inner = component.inner.take().unwrap();

    let callback = async move {
        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |_editor: &mut Editor, compositor: &mut Compositor, _| compositor.push(inner),
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);
}

fn enqueue_command(cx: &mut Context, callback_fn: SteelVal) {
    let rooted = callback_fn.as_rooted();

    let callback = async move {
        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut ctx = Context {
                    register: None,
                    count: None,
                    editor,
                    callback: Vec::new(),
                    on_next_key_callback: None,
                    jobs,
                };

                let cloned_func = rooted.value();

                if let Err(e) = ENGINE.with(|x| {
                    x.borrow_mut()
                        .with_mut_reference::<Context, Context>(&mut ctx)
                        .consume(move |engine, args| {
                            engine.call_function_with_args(cloned_func.clone(), args)
                        })
                }) {
                    present_error(&mut ctx, e);
                }
            },
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);
}

// Apply arbitrary delay for update rate...
fn enqueue_command_with_delay(cx: &mut Context, delay: SteelVal, callback_fn: SteelVal) {
    let rooted = callback_fn.as_rooted();

    let callback = async move {
        let delay = delay.int_or_else(|| panic!("FIX ME")).unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(delay as u64)).await;

        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut ctx = Context {
                    register: None,
                    count: None,
                    editor,
                    callback: Vec::new(),
                    on_next_key_callback: None,
                    jobs,
                };

                let cloned_func = rooted.value();

                if let Err(e) = ENGINE.with(|x| {
                    x.borrow_mut()
                        .with_mut_reference::<Context, Context>(&mut ctx)
                        .consume(move |engine, args| {
                            engine.call_function_with_args(cloned_func.clone(), args)
                        })
                }) {
                    present_error(&mut ctx, e);
                }
            },
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);
}

// value _must_ be a future here. Otherwise awaiting will cause problems!
fn await_value(cx: &mut Context, value: SteelVal, callback_fn: SteelVal) {
    if !value.is_future() {
        return;
    }

    let rooted = callback_fn.as_rooted();

    let callback = async move {
        let future_value = value.as_future().unwrap().await;

        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut ctx = Context {
                    register: None,
                    count: None,
                    editor,
                    callback: Vec::new(),
                    on_next_key_callback: None,
                    jobs,
                };

                let cloned_func = rooted.value();

                match future_value {
                    Ok(inner) => {
                        let callback = move |engine: &mut Engine, mut args: Vec<SteelVal>| {
                            args.push(inner);
                            engine.call_function_with_args(cloned_func.clone(), args)
                        };
                        if let Err(e) = ENGINE.with(|x| {
                            x.borrow_mut()
                                .with_mut_reference::<Context, Context>(&mut ctx)
                                .consume_once(callback)
                        }) {
                            present_error(&mut ctx, e);
                        }
                    }
                    Err(e) => {
                        present_error(&mut ctx, e);
                    }
                }
            },
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);
}
// Check that we successfully created a directory?
fn create_directory(path: String) {
    let path = helix_core::path::get_canonicalized_path(&PathBuf::from(path));

    if path.exists() {
        return;
    } else {
        std::fs::create_dir(path).unwrap();
    }
}

/// Change config at runtime. Access nested values by dot syntax, for
/// example to disable smart case search, use `:set search.smart-case false`.
fn set_options(
    cx: &mut compositor::Context,
    args: &[Cow<str>],
    event: PromptEvent,
) -> anyhow::Result<()> {
    if event != PromptEvent::Validate {
        return Ok(());
    }

    if args.len() % 2 != 0 {
        anyhow::bail!("Bad arguments. Usage: `:set key field`");
    }

    let mut config = serde_json::json!(&cx.editor.config().deref());
    // let key_error = || anyhow::anyhow!("Unknown key `{}`", key);
    // let field_error = |_| anyhow::anyhow!("Could not parse field `{}`", arg);

    for args in args.chunks_exact(2) {
        let (key, arg) = (&args[0].to_lowercase(), &args[1]);

        let key_error = || anyhow::anyhow!("Unknown key `{}`", key);
        let field_error = |_| anyhow::anyhow!("Could not parse field `{}`", arg);

        // let mut config = serde_json::json!(&cx.editor.config().deref());
        let pointer = format!("/{}", key.replace('.', "/"));
        let value = config.pointer_mut(&pointer).ok_or_else(key_error)?;

        *value = if value.is_string() {
            // JSON strings require quotes, so we can't .parse() directly
            Value::String(arg.to_string())
        } else {
            arg.parse().map_err(field_error)?
        };
    }

    let config =
        serde_json::from_value(config).map_err(|_| anyhow::anyhow!("Could not parse config"))?;

    cx.editor
        .config_events
        .0
        .send(ConfigEvent::Update(config))?;
    Ok(())
}

pub fn refresh_language_configuration(cx: &mut Context) -> anyhow::Result<()> {
    cx.editor
        .config_events
        .0
        .send(ConfigEvent::UpdateLanguageConfiguration)?;

    Ok(())
}

pub fn cx_pos_within_text(cx: &mut Context) -> usize {
    let (view, doc) = current_ref!(cx.editor);

    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone();

    let pos = selection.primary().cursor(text);

    pos
}

pub fn get_helix_cwd(_cx: &mut Context) -> Option<String> {
    helix_loader::current_working_dir()
        .as_os_str()
        .to_str()
        .map(|x| x.into())
}

// Special newline...
pub fn custom_insert_newline(cx: &mut Context, indent: String) {
    let (view, doc) = current_ref!(cx.editor);

    // let rope = doc.text().clone();

    let text = doc.text().slice(..);

    let contents = doc.text();
    let selection = doc.selection(view.id).clone();
    let mut ranges = helix_core::SmallVec::with_capacity(selection.len());

    // TODO: this is annoying, but we need to do it to properly calculate pos after edits
    let mut global_offs = 0;

    let mut transaction =
        helix_core::Transaction::change_by_selection(contents, &selection, |range| {
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

fn search_in_directory(cx: &mut Context, directory: String) {
    let search_path = expand_tilde(&PathBuf::from(directory));
    crate::commands::search_in_directory(cx, search_path);
}

// TODO: Result should create unrecoverable result, and should have a special
// recoverable result - that way we can handle both, not one in particular
fn regex_selection(cx: &mut Context, regex: String) {
    if let Ok(regex) = Regex::new(&regex) {
        let (view, doc) = current!(cx.editor);
        let text = doc.text().slice(..);
        if let Some(selection) =
            helix_core::selection::select_on_matches(text, doc.selection(view.id), &regex)
        {
            doc.set_selection(view.id, selection);
        }
    }
}

fn replace_selection(cx: &mut Context, value: String) {
    let (view, doc) = current!(cx.editor);

    let selection = doc.selection(view.id);
    let transaction =
        helix_core::Transaction::change_by_selection(doc.text(), selection, |range| {
            if !range.is_empty() {
                (range.from(), range.to(), Some(value.to_owned().into()))
            } else {
                (range.from(), range.to(), None)
            }
        });

    doc.apply(&transaction, view.id);
}

fn show_completion_prompt(cx: &mut Context, items: Vec<String>) {
    let (view, doc) = current!(cx.editor);

    let items = items
        .into_iter()
        .map(|x| crate::ui::CompletionItem {
            item: CompletionItem::new_simple(x, "".to_string()),
            language_server_id: usize::MAX,
            resolved: true,
        })
        .collect();

    let text = doc.text();
    let cursor = doc.selection(view.id).primary().cursor(text.slice(..));

    let trigger = crate::handlers::completion::Trigger::new(
        cursor,
        view.id,
        doc.id(),
        crate::handlers::completion::TriggerKind::Manual,
    );

    let savepoint = doc.savepoint(view);

    let callback = async move {
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                crate::handlers::completion::show_completion(
                    editor, compositor, items, trigger, savepoint,
                );
            },
        ));
        Ok(call)
    };
    cx.jobs.callback(callback);
}

fn move_window_to_the_left(cx: &mut Context) {
    while cx
        .editor
        .tree
        .swap_split_in_direction(helix_view::tree::Direction::Left)
        .is_some()
    {}
}

fn move_window_to_the_right(cx: &mut Context) {
    while cx
        .editor
        .tree
        .swap_split_in_direction(helix_view::tree::Direction::Right)
        .is_some()
    {}
}

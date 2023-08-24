use fuzzy_matcher::FuzzyMatcher;
use helix_core::{
    extensions::{rope_module, rope_slice_module, SRopeSlice, SteelRopeSlice},
    graphemes,
    shellwords::Shellwords,
    Range, Selection, Tendril,
};
use helix_view::{
    document::Mode,
    editor::{Action, ConfigEvent},
    extension::document_id_to_usize,
    input::KeyEvent,
    Document, DocumentId, Editor,
};
use once_cell::sync::Lazy;
use serde_json::Value;
use steel::{
    gc::unsafe_erased_pointers::CustomReference,
    rerrs::ErrorKind,
    rvals::{
        as_underlying_type, AsRefMutSteelValFromRef, AsRefSteelVal, FromSteelVal, IntoSteelVal,
    },
    steel_vm::{
        engine::Engine,
        register_fn::{RegisterFn, RegisterFnBorrowed},
    },
    SteelErr, SteelVal,
};

use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    marker::PhantomData,
    ops::Deref,
    path::PathBuf,
    sync::Mutex,
};
use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use steel::{rvals::Custom, steel_vm::builtin::BuiltInModule};

use crate::{
    compositor::{self, Component, Compositor},
    config::Config,
    job::{self, Callback},
    keymap::{self, merge_keys, KeyTrie, KeymapResult, Keymaps},
    ui::{self, menu::Item, overlay::overlaid, Popup, Prompt, PromptEvent},
};

use self::components::SteelDynamicComponent;

use super::{
    indent,
    insert::{insert_char, insert_string},
    plugin::{DylibContainers, ExternalModule},
    shell_impl, Context, MappableCommand, TYPABLE_COMMAND_LIST,
};

thread_local! {
    pub static ENGINE: std::rc::Rc<std::cell::RefCell<steel::steel_vm::engine::Engine>> = configure_engine();
}

pub struct ExternalContainersAndModules {
    containers: DylibContainers,
    modules: Vec<ExternalModule>,
}

mod components;

// pub struct PluginEngine<T: PluginSystem>(PhantomData<T>);

pub struct ScriptingEngine;

pub trait PluginSystem {
    fn initialize();
    fn run_initialization_script(cx: &mut Context);

    fn call_function_if_global_exists(cx: &mut Context, name: &str, args: Vec<Cow<str>>);
    fn call_typed_command_if_global_exists<'a>(
        cx: &mut compositor::Context,
        input: &'a str,
        parts: &'a [&'a str],
        event: PromptEvent,
    ) -> bool;

    fn get_doc_for_identifier(ident: &str) -> Option<String>;
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
        }
    }
}

thread_local! {
    pub static BUFFER_OR_EXTENSION_KEYBINDING_MAP: SteelVal =
        SteelVal::boxed(SteelVal::empty_hashmap());

    pub static REVERSE_BUFFER_MAP: SteelVal =
        SteelVal::boxed(SteelVal::empty_hashmap());
}

fn load_keymap_api(engine: &mut Engine, api: KeyMapApi) {
    let mut module = BuiltInModule::new("helix/core/keymaps");

    module.register_fn("helix-current-keymap", api.get_keymap);
    module.register_fn("helix-empty-keymap", api.empty_keymap);
    module.register_fn("helix-default-keymap", api.default_keymap);
    module.register_fn("helix-merge-keybindings", api.merge_keybindings);
    module.register_fn("helix-string->keymap", api.string_to_embedded_keymap);
    module.register_fn("keymap?", api.is_keymap);

    // This should be associated with a corresponding scheme module to wrap this up
    module.register_value(
        "*buffer-or-extension-keybindings*",
        BUFFER_OR_EXTENSION_KEYBINDING_MAP.with(|x| x.clone()),
    );
    module.register_value(
        "*reverse-buffer-map*",
        REVERSE_BUFFER_MAP.with(|x| x.clone()),
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

    engine.register_module(module);
}

fn load_typed_commands(engine: &mut Engine) {
    let mut module = BuiltInModule::new("helix/core/typable".to_string());

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

fn load_editor_api(engine: &mut Engine, api: EditorApi) {
    todo!()
}

fn load_document_api(engine: &mut Engine, api: DocumentApi) {
    todo!()
}

fn load_component_api(engine: &mut Engine, api: ComponentApi) {
    todo!()
}

impl ScriptingEngine {
    pub fn initialize() {
        initialize_engine();
    }

    pub fn run_initialization_script(cx: &mut Context) {
        run_initialization_script(cx)
    }

    // Attempt to fetch the keymap for the extension
    pub fn get_keymap_for_extension<'a>(cx: &'a mut Context) -> Option<SteelVal> {
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

        None
    }

    pub fn call_function_if_global_exists(
        cx: &mut Context,
        name: &str,
        args: Vec<Cow<str>>,
    ) -> bool {
        if ENGINE.with(|x| x.borrow().global_exists(name)) {
            let args = steel::List::from(
                args.iter()
                    .map(|x| x.clone().into_steelval().unwrap())
                    .collect::<Vec<_>>(),
            );

            if let Err(e) = ENGINE.with(|x| {
                let mut guard = x.borrow_mut();

                {
                    guard.register_value("_helix_args", steel::rvals::SteelVal::ListV(args));

                    let res = guard.run_with_reference::<Context, Context>(
                        cx,
                        "*context*",
                        &format!("(apply {} (cons *context* _helix_args))", name),
                    );

                    guard.register_value("_helix_args", steel::rvals::SteelVal::Void);

                    res
                }
            }) {
                cx.editor.set_error(format!("{}", e));
            }
            true
        } else {
            false
        }
    }

    pub fn call_typed_command_if_global_exists<'a>(
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
                // TODO: @Matt - extract this whole API call here to just be inside the engine module
                // For what its worth, also explore a more elegant API for calling apply with some arguments,
                // this does work, but its a little opaque.
                if let Err(e) = ENGINE.with(|x| {
                    let args = steel::List::from(
                        args[1..]
                            .iter()
                            .map(|x| x.clone().into_steelval().unwrap())
                            .collect::<Vec<_>>(),
                    );

                    let mut guard = x.borrow_mut();
                    // let mut maybe_callback = None;

                    let res = {
                        let mut cx = Context {
                            register: None,
                            count: std::num::NonZeroUsize::new(1),
                            editor: cx.editor,
                            callback: None,
                            on_next_key_callback: None,
                            jobs: cx.jobs,
                        };

                        guard.register_value("_helix_args", steel::rvals::SteelVal::ListV(args));

                        let res = guard.run_with_reference::<Context, Context>(
                            &mut cx,
                            "*context*",
                            &format!("(apply {} (cons *context* _helix_args))", parts[0]),
                        );

                        guard.register_value("_helix_args", steel::rvals::SteelVal::Void);

                        res
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

    pub fn get_doc_for_identifier(ident: &str) -> Option<String> {
        if ENGINE.with(|x| x.borrow().global_exists(ident)) {
            if let Some(v) = super::engine::ExportedIdentifiers::engine_get_doc(ident) {
                return Some(v.into());
            }

            return Some("Run this plugin command!".into());
        }

        None
    }

    pub(crate) fn fuzzy_match<'a>(
        fuzzy_matcher: &'a fuzzy_matcher::skim::SkimMatcherV2,
        input: &'a str,
    ) -> Vec<(String, i64)> {
        EXPORTED_IDENTIFIERS
            .identifiers
            .read()
            .unwrap()
            .iter()
            .filter_map(|name| {
                fuzzy_matcher
                    .fuzzy_match(name, input)
                    .map(|score| (name, score))
            })
            .map(|x| (x.0.to_string(), x.1))
            .collect::<Vec<_>>()
    }

    pub(crate) fn is_exported(ident: &str) -> bool {
        EXPORTED_IDENTIFIERS
            .identifiers
            .read()
            .unwrap()
            .contains(ident)
    }

    pub(crate) fn engine_get_doc(ident: &str) -> Option<String> {
        EXPORTED_IDENTIFIERS
            .docs
            .read()
            .unwrap()
            .get(ident)
            .cloned()
    }

    // fn get_doc(&self, ident: &str) -> Option<String> {
    //     self.docs.read().unwrap().get(ident).cloned()
    // }
}

// External modules that can load via rust dylib. These can then be consumed from
// steel as needed, via the standard FFI for plugin functions.
// pub(crate) static EXTERNAL_DYLIBS: Lazy<Arc<RwLock<ExternalContainersAndModules>>> =
//     Lazy::new(|| {
//         let mut containers = DylibContainers::new();

//         // Load the plugins with respect to the extensions directory.
//         // containers.load_modules_from_directory(Some(
//         //     helix_loader::config_dir()
//         //         .join("extensions")
//         //         .to_str()
//         //         .unwrap()
//         //         .to_string(),
//         // ));

//         println!("Found dylibs: {}", containers.containers.len());

//         let modules = containers.create_commands();

//         println!("Modules length: {}", modules.len());

//         Arc::new(RwLock::new(ExternalContainersAndModules {
//             containers,
//             modules,
//         }))

//         // Arc::new(RwLock::new(containers))
//     });

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

/// Run the initialization script located at `$helix_config/init.scm`
/// This runs the script in the global environment, and does _not_ load it as a module directly
fn run_initialization_script(cx: &mut Context) {
    log::info!("Loading init.scm...");

    let helix_module_path = helix_loader::helix_module_file();

    // TODO: Report the error from requiring the file!
    ENGINE.with(|engine| {

        let mut guard = engine.borrow_mut();

        let res = guard.run(&format!(
            r#"(require "{}")"#,
            helix_module_path.to_str().unwrap()
        ));

        // Present the error in the helix.scm loading
        if let Err(e) = res {
            present_error(cx, e);
            return;
        }

        let helix_path =
            "__module-mangler".to_string() + helix_module_path.as_os_str().to_str().unwrap();

        if let Ok(module) = guard.extract_value(&helix_path) {
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
                present_error(
                    cx,
                    SteelErr::new(
                        ErrorKind::Generic,
                        "Unable to parse exported identifiers from helix module!".to_string(),
                    ),
                );

                return;
            }
        }

        let helix_module_path = helix_loader::steel_init_file();

        // These contents need to be registered with the path?
        if let Ok(contents) = std::fs::read_to_string(&helix_module_path) {
            let res = guard
                .run_with_reference_from_path::<Context, Context>(
                    cx,
                    "*helix.cx*",
                    &contents,
                    helix_module_path,
                );

            match res {
                Ok(_) => {}
                Err(e) => present_error(cx, e),
            }

            log::info!("Finished loading init.scm!")
        } else {
            log::info!("No init.scm found, skipping loading.")
        }
    });
}

pub static KEYBINDING_QUEUE: Lazy<SharedKeyBindingsEventQueue> =
    Lazy::new(|| SharedKeyBindingsEventQueue::new());

pub static CALLBACK_QUEUE: Lazy<CallbackQueue> = Lazy::new(|| CallbackQueue::new());

pub static EXPORTED_IDENTIFIERS: Lazy<ExportedIdentifiers> =
    Lazy::new(|| ExportedIdentifiers::default());

pub static STATUS_LINE_MESSAGE: Lazy<StatusLineMessage> = Lazy::new(|| StatusLineMessage::new());

pub struct StatusLineMessage {
    message: Arc<RwLock<Option<String>>>,
}

impl StatusLineMessage {
    pub fn new() -> Self {
        Self {
            message: std::sync::Arc::new(std::sync::RwLock::new(None)),
        }
    }

    pub fn set(message: String) {
        *STATUS_LINE_MESSAGE.message.write().unwrap() = Some(message);
    }

    pub fn get() -> Option<String> {
        STATUS_LINE_MESSAGE.message.read().unwrap().clone()
    }
}

impl Item for SteelVal {
    type Data = ();

    // TODO: This shouldn't copy the data every time
    fn format(&self, _data: &Self::Data) -> tui::widgets::Row {
        let formatted = self.to_string();

        formatted
            .strip_prefix("\"")
            .unwrap_or(&formatted)
            .strip_suffix("\"")
            .unwrap_or(&formatted)
            .to_owned()
            .into()
    }
}

pub struct CallbackQueue {
    queue: Arc<Mutex<VecDeque<String>>>,
}

impl CallbackQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn enqueue(value: String) {
        CALLBACK_QUEUE.queue.lock().unwrap().push_back(value);
    }

    // Dequeue should probably be a R/W lock?
    pub fn dequeue() -> Option<String> {
        CALLBACK_QUEUE.queue.lock().unwrap().pop_front()
    }
}

/// In order to send events from the engine back to the configuration, we can created a shared
/// queue that the engine and the config push and pull from. Alternatively, we could use a channel
/// directly, however this was easy enough to set up.
pub struct SharedKeyBindingsEventQueue {
    raw_bindings: Arc<Mutex<Vec<String>>>,
}

impl SharedKeyBindingsEventQueue {
    pub fn new() -> Self {
        Self {
            raw_bindings: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn merge(other_as_json: String) {
        KEYBINDING_QUEUE
            .raw_bindings
            .lock()
            .unwrap()
            .push(other_as_json);
    }

    pub fn get() -> Option<HashMap<Mode, KeyTrie>> {
        let guard = KEYBINDING_QUEUE.raw_bindings.lock().unwrap();

        if let Some(first) = guard.get(0).clone() {
            let mut initial = serde_json::from_str(first).unwrap();

            // while let Some(remaining_event) = guard.pop_front() {
            for remaining_event in guard.iter() {
                let bindings = serde_json::from_str(remaining_event).unwrap();

                merge_keys(&mut initial, bindings);
            }

            return Some(initial);
        }

        None
    }
}

impl Custom for PromptEvent {}

impl<'a> CustomReference for Context<'a> {}

steel::custom_reference!(Context<'a>);

fn get_editor<'a>(cx: &'a mut Context<'a>) -> &'a mut Editor {
    cx.editor
}

fn get_ro_editor<'a>(cx: &'a mut Context<'a>) -> &'a Editor {
    &cx.editor
}

fn get_themes(cx: &mut Context) -> Vec<String> {
    ui::completers::theme(cx.editor, "")
        .into_iter()
        .map(|x| x.1.to_string())
        .collect()
}

// TODO: This is not necessary anymore. We can get away with native threads in steel, and otherwise run background tasks
// that may or may not live the duration of the editor time in there.
// fn configure_background_thread() {
//     std::thread::spawn(move || {
//         let mut engine = steel::steel_vm::engine::Engine::new();

//         engine.register_fn("set-status-line!", StatusLineMessage::set);

//         let helix_module_path = helix_loader::config_dir().join("background.scm");

//         if let Ok(contents) = std::fs::read_to_string(&helix_module_path) {
//             engine.run(&contents).ok();
//         }
//     });
// }

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

// impl CustomReference for tui::buffer::Buffer {}

// TODO: Call the function inside the component, using the global engine. Consider running in its own engine
// but leaving it all in the same one is kinda nice

// Does this work?

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

fn configure_engine() -> std::rc::Rc<std::cell::RefCell<steel::steel_vm::engine::Engine>> {
    let mut engine = steel::steel_vm::engine::Engine::new();

    log::info!("Loading engine!");

    engine.register_fn("hx.context?", |_: &mut Context| true);

    engine.register_fn("log::info!", |message: String| log::info!("{}", message));

    engine.register_fn("hx.custom-insert-newline", custom_insert_newline);
    engine.register_fn("hx.cx->pos", cx_pos_within_text);

    // Load native modules from the directory. Another idea - have a separate dlopen loading system
    // in place that does not use the type id, and instead we generate the module after the dylib
    // is added. That way functions _must_ have a specific signature, and then we add the integration
    // later.
    // engine.load_modules_from_directory(
    //     helix_loader::config_dir()
    //         .join("extensions")
    //         .to_str()
    //         .unwrap()
    //         .to_string(),
    // );

    engine.register_fn("enqueue-callback!", CallbackQueue::enqueue);

    load_keymap_api(&mut engine, KeyMapApi::new());

    let mut rope_slice_module = rope_module();

    rope_slice_module.register_fn("document->slice", document_to_text);

    // Load the ropes + slice module
    // engine.register_module(rope_slice_module());
    // rope_slice_module.register_fn("document->slice", document_to_text);

    // RegisterFnBorrowed::<
    //     _,
    //     steel::steel_vm::register_fn::MarkerWrapper9<(
    //         Document,
    //         Document,
    //         SRopeSlice<'_>,
    //         SRopeSlice<'static>,
    //     )>,
    //     SRopeSlice,
    // >::register_fn_borrowed(&mut rope_slice_module, "document->slice", document_to_text);

    engine.register_module(rope_slice_module);

    // engine.register_fn("helix-current-keymap", get_keymap);
    // engine.register_fn("helix-empty-keymap", empty_keymap);
    // engine.register_fn("helix-default-keymap", default_keymap);
    // engine.register_fn("helix-merge-keybindings", merge_keybindings);
    // engine.register_fn("helix-string->keymap", string_to_embedded_keymap);

    // Use this to get at buffer specific keybindings
    // engine.register_value(
    // "*buffer-or-extension-keybindings*",
    // SteelVal::empty_hashmap(),
    // );
    // engine.register_value("*reverse-buffer-map*", SteelVal::empty_hashmap());

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
                        callback: None,
                        on_next_key_callback: None,
                        jobs: cx.jobs,
                    };

                    let cloned_func = callback_fn.clone();

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

    engine.register_fn("Picker::new", |values: Vec<String>| todo!());

    // engine.register_fn(
    //     "Picker::new",
    //     |contents: &mut Wrapped
    // )

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

    let mut module = BuiltInModule::new("helix/core/keybindings".to_string());
    module.register_fn("set-keybindings!", SharedKeyBindingsEventQueue::merge);

    RegisterFn::<
        _,
        steel::steel_vm::register_fn::MarkerWrapper7<(
            Context<'_>,
            helix_view::Editor,
            helix_view::Editor,
            Context<'static>,
        )>,
        helix_view::Editor,
    >::register_fn(&mut engine, "cx-editor!", get_editor);

    engine.register_fn("set-scratch-buffer-name!", set_scratch_buffer_name);

    engine.register_fn("editor-focus", current_focus);
    engine.register_fn("editor->doc-id", get_document_id);
    engine.register_fn("doc-id->usize", document_id_to_usize);
    engine.register_fn("editor-switch!", switch);
    engine.register_fn("editor-set-focus!", Editor::focus);
    engine.register_fn("editor-mode", editor_get_mode);
    engine.register_fn("editor-set-mode!", editor_set_mode);
    engine.register_fn("editor-doc-in-view?", is_document_in_view);

    // engine.register_fn("editor->get-document", get_document);

    // TODO: These are some horrendous type annotations, however... they do work?
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
    >::register_fn(&mut engine, "editor->get-document", get_document);

    // Check if the doc exists first
    engine.register_fn("editor-doc-exists?", document_exists);
    engine.register_fn("Document-path", document_path);
    engine.register_fn("helix.context?", is_context);
    engine.register_type::<DocumentId>("DocumentId?");

    // RegisterFn::<
    //     _,
    //     steel::steel_vm::register_fn::MarkerWrapper7<(
    //         Context<'_>,
    //         helix_view::Editor,
    //         helix_view::Editor,
    //         Context<'static>,
    //     )>,
    //     helix_view::Editor,
    // >::register_fn(&mut engine, "cx-editor-ro!", get_ro_editor);

    engine.register_fn("editor-cursor", Editor::cursor);

    engine.register_fn("cx->cursor", |cx: &mut Context| cx.editor.cursor());

    // TODO:
    // Position related functions. These probably should be defined alongside the actual impl for Custom in the core crate
    engine.register_fn("Position::new", helix_core::Position::new);
    engine.register_fn("Position::default", helix_core::Position::default);
    engine.register_fn("Position-row", |position: helix_core::Position| {
        position.row
    });

    engine.register_fn("cx->themes", get_themes);
    engine.register_fn("set-status-line!", StatusLineMessage::set);

    engine.register_module(module);

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

    let mut module = BuiltInModule::new("helix/core/static".to_string());

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
    module.register_fn("block-on-shell-command", run_shell_command_text);

    module.register_fn("cx->current-file", current_path);

    engine.register_module(module);

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
    pub(crate) fn fuzzy_match<'a>(
        fuzzy_matcher: &'a fuzzy_matcher::skim::SkimMatcherV2,
        input: &'a str,
    ) -> Vec<(String, i64)> {
        EXPORTED_IDENTIFIERS
            .identifiers
            .read()
            .unwrap()
            .iter()
            .filter_map(|name| {
                fuzzy_matcher
                    .fuzzy_match(name, input)
                    .map(|score| (name, score))
            })
            .map(|x| (x.0.to_string(), x.1))
            .collect::<Vec<_>>()
    }

    pub(crate) fn is_exported(ident: &str) -> bool {
        EXPORTED_IDENTIFIERS
            .identifiers
            .read()
            .unwrap()
            .contains(ident)
    }

    pub(crate) fn engine_get_doc(ident: &str) -> Option<String> {
        EXPORTED_IDENTIFIERS
            .docs
            .read()
            .unwrap()
            .get(ident)
            .cloned()
    }

    // fn get_doc(&self, ident: &str) -> Option<String> {
    //     self.docs.read().unwrap().get(ident).cloned()
    // }
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
    helix_loader::helix_module_file()
        .to_str()
        .unwrap()
        .to_string()
}

fn get_init_scm_path() -> String {
    helix_loader::steel_init_file()
        .to_str()
        .unwrap()
        .to_string()
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

fn cx_current_focus(cx: &mut Context) -> helix_view::ViewId {
    cx.editor.tree.focus
}

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

fn switch(editor: &mut Editor, doc_id: DocumentId) {
    editor.switch(doc_id, Action::VerticalSplit)
}

// fn editor_set_focus(editor: &mut Editor, view_id: helix_view::ViewId) {
//     editor.tree.focus = view_id
// }

fn editor_get_mode(editor: &mut Editor) -> Mode {
    editor.mode
}

fn editor_set_mode(editor: &mut Editor, mode: Mode) {
    editor.mode = mode
}

// fn insert_text(cx: &mut Context, text: String) {
//     let count = cx.count();
//     let reg_name = cx.register.unwrap_or('"');
//     let (view, doc) = current!(cx.editor);
//     let registers = &mut cx.editor.registers;

//     if let Some(values) = registers.read(reg_name) {
//         paste_impl(values, doc, view, pos, count, cx.editor.mode);
//     }
// }
// cx->editor
//

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
    let callback = async move {
        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut ctx = Context {
                    register: None,
                    count: None,
                    editor,
                    callback: None,
                    on_next_key_callback: None,
                    jobs,
                };

                let cloned_func = callback_fn.clone();

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
    let callback = async move {
        let delay = delay.int_or_else(|| panic!("FIX ME")).unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(delay as u64)).await;

        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut ctx = Context {
                    register: None,
                    count: None,
                    editor,
                    callback: None,
                    on_next_key_callback: None,
                    jobs,
                };

                let cloned_func = callback_fn.clone();

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
    let callback = async move {
        let future_value = value.as_future().unwrap().await;

        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut ctx = Context {
                    register: None,
                    count: None,
                    editor,
                    callback: None,
                    on_next_key_callback: None,
                    jobs,
                };

                let cloned_func = callback_fn.clone();

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

pub fn cx_pos_within_text(cx: &mut Context) -> usize {
    let (view, doc) = current_ref!(cx.editor);

    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone();

    let pos = selection.primary().cursor(text);

    pos
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
                // let indent = indent::indent_for_newline(
                //     doc.language_config(),
                //     doc.syntax(),
                //     &doc.indent_style,
                //     doc.tab_width(),
                //     text,
                //     current_line,
                //     pos,
                //     current_line,
                // );

                // let cloned_func = thunk.clone();
                // let steel_rope = SteelRopeSlice::new(rope.clone()).into_steelval().unwrap();

                // let indent = if let Ok(result) = ENGINE.with(|x| {
                //     x.borrow_mut().call_function_with_args(
                //         cloned_func,
                //         vec![
                //             steel_rope,
                //             current_line.into_steelval().unwrap(),
                //             pos.into_steelval().unwrap(),
                //         ],
                //     )
                // }) {
                //     result.as_string().unwrap().to_string()
                // } else {
                //     "".to_string()
                // };

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

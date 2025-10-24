mod components;

use arc_swap::{ArcSwap, ArcSwapAny};
use helix_core::{
    command_line::Args,
    diagnostic::Severity,
    extensions::steel_implementations::{rope_module, SteelRopeSlice},
    find_workspace, graphemes,
    syntax::{
        self,
        config::{
            default_timeout, AutoPairConfig, LanguageConfiguration, LanguageServerConfiguration,
            SoftWrap,
        },
    },
    text_annotations::InlineAnnotation,
    Range, Selection, Tendril, Transaction,
};
use helix_event::register_hook;
use helix_lsp::jsonrpc;
use helix_view::{
    annotations::diagnostics::DiagnosticFilter,
    document::{DocumentInlayHints, DocumentInlayHintsId, Mode},
    editor::{
        Action, AutoSave, BufferLine, ClippingConfiguration, ConfigEvent, CursorShapeConfig,
        FilePickerConfig, GutterConfig, IndentGuidesConfig, LineEndingConfig, LineNumber,
        LspConfig, SearchConfig, SmartTabConfig, StatusLineConfig, TerminalConfig,
        WhitespaceConfig, WhitespaceRender, WhitespaceRenderValue,
    },
    events::{DocumentDidOpen, DocumentFocusLost, DocumentSaved, SelectionDidChange},
    extension::document_id_to_usize,
    graphics::CursorKind,
    input::KeyEvent,
    theme::Color,
    DocumentId, Editor, Theme, ViewId,
};
use once_cell::sync::{Lazy, OnceCell};
use serde_json::Value;
use steel::{
    compiler::modules::steel_home,
    gc::{unsafe_erased_pointers::CustomReference, ShareableMut},
    rerrs::ErrorKind,
    rvals::{as_underlying_type, AsRefMutSteelVal, FromSteelVal, IntoSteelVal, SteelString},
    steel_vm::{
        engine::Engine, mutex_lock, mutex_unlock, register_fn::RegisterFn, ThreadStateController,
    },
    steelerr, RootedSteelVal, SteelErr, SteelVal,
};
use termina::EventReader;

use std::{
    borrow::Cow,
    collections::HashMap,
    error::Error,
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex, MutexGuard, RwLock, RwLockReadGuard, Weak,
    },
    time::{Duration, SystemTime},
};
use std::{str::FromStr as _, sync::Arc};

use steel::{rvals::Custom, steel_vm::builtin::BuiltInModule};

use crate::{
    commands::{insert, TYPABLE_COMMAND_LIST},
    compositor::{self, Component, Compositor},
    config::Config,
    events::{OnModeSwitch, PostCommand, PostInsertChar},
    job::{self, Callback},
    keymap::{self, merge_keys, KeyTrie, KeymapResult, MappableCommand},
    ui::{self, picker::PathOrId, PickerColumn, Popup, Prompt, PromptEvent},
};

use components::SteelDynamicComponent;

use components::helix_component_module;

use super::{Context, TerminalEventReaderHandle};
use insert::insert_char;

static INTERRUPT_HANDLER: Lazy<Mutex<Option<Arc<InterruptHandler>>>> =
    Lazy::new(|| Mutex::new(None));
static SAFEPOINT_HANDLER: Lazy<Mutex<Option<Arc<SafepointHandler>>>> =
    Lazy::new(|| Mutex::new(None));

static GLOBAL_OFFSET: AtomicUsize = AtomicUsize::new(0);

static EVENT_READER: OnceCell<EventReader> = OnceCell::new();

fn install_event_reader(event_reader: TerminalEventReaderHandle) {
    #[cfg(feature = "integration")]
    {}

    #[cfg(all(not(windows), not(feature = "integration")))]
    {
        EVENT_READER.set(event_reader.reader).ok();
    }
}

fn reload_engine() {
    enter_engine(|engine| {
        // Install a new generation. Anything using the old engine at this point
        // should (hopefully) gracefully go out of scope.
        increment_generation();

        reset_buffer_extension_keymap();
        reset_lsp_call_registry();

        *engine = setup();
    })
}

fn setup() -> Engine {
    let engine = steel::steel_vm::engine::Engine::new();

    let controller = engine.get_thread_state_controller();
    let running = Arc::new(AtomicBool::new(false));

    let current_generation = load_generation();

    fn is_event_available() -> std::io::Result<bool> {
        #[cfg(windows)]
        {
            crossterm::event::poll(Duration::from_millis(10))
        }

        #[cfg(unix)]
        {
            EVENT_READER
                .get()
                .unwrap()
                .poll(Some(Duration::from_millis(10)), |_| true)
        }
    }

    let controller_clone = controller.clone();
    let running_clone = running.clone();

    // TODO: Only allow interrupt after a certain amount of time...
    // perhaps something like, 500 ms? That way interleaving calls to
    // steel functions don't accidentally cause an interrupt.
    let thread_handle = std::thread::spawn(move || {
        let controller = controller_clone;
        let running = running_clone;

        while is_current_generation(current_generation) {
            std::thread::park();

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                #[cfg(unix)]
                if is_event_available().unwrap_or(false) {
                    let event = EVENT_READER.get().unwrap().read(|_| true);

                    if let Ok(termina::Event::Key(termina::event::KeyEvent {
                        code: termina::event::KeyCode::Char('c'),
                        modifiers: termina::event::Modifiers::CONTROL,
                        ..
                    })) = event
                    {
                        controller.interrupt();
                        break;
                    }
                }

                #[cfg(windows)]
                if is_event_available().unwrap_or(false) {
                    use crossterm::event::{Event, KeyCode, KeyModifiers};
                    let event = crossterm::event::read();

                    if let Ok(Event::Key(crossterm::event::KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    })) = event
                    {
                        controller.interrupt();
                        break;
                    }
                }
            }
        }
    });

    let running_command = Arc::new(AtomicBool::new(true));
    let running_command_clone = running_command.clone();

    // Put the engine in a place where we can make substantive progress.
    let safepoint_wakeup = std::thread::spawn(move || {
        let running_command = running_command_clone;

        while is_current_generation(current_generation) {
            // If this is running, don't acquire the lock. Park until we're back.
            if running_command.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::park();
                continue;
            }

            GLOBAL_ENGINE.lock().unwrap().enter_safepoint(|| {
                // Set the thread to running, and then park it.
                // Eventually it will be awoken once the engine
                // exits the engine context
                while !running_command.load(std::sync::atomic::Ordering::Relaxed) {
                    std::thread::park();
                }
            });
        }
    });

    *SAFEPOINT_HANDLER.lock().unwrap() = Some(Arc::new(SafepointHandler {
        running_command,
        handle: safepoint_wakeup,
    }));

    *INTERRUPT_HANDLER.lock().unwrap() = Some(Arc::new(InterruptHandler {
        controller: controller.clone(),
        running: running.clone(),
        handle: thread_handle,
    }));

    configure_engine_impl(engine)
}

// The Steel scripting engine instance. This is what drives the whole integration.
pub static GLOBAL_ENGINE: Lazy<Mutex<steel::steel_vm::engine::Engine>> =
    Lazy::new(|| Mutex::new(setup()));

static GENERATION: AtomicUsize = AtomicUsize::new(0);

fn increment_generation() {
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

fn is_current_generation(gen: usize) -> bool {
    GENERATION.load(std::sync::atomic::Ordering::SeqCst) == gen
}

fn load_generation() -> usize {
    GENERATION.load(std::sync::atomic::Ordering::SeqCst)
}

fn acquire_engine_lock() -> MutexGuard<'static, Engine> {
    GLOBAL_ENGINE.lock().unwrap()
}

/// Run a function with exclusive access to the engine. This only
/// locks the engine that is running on the main thread.
pub fn enter_engine<F, R>(f: F) -> R
where
    F: FnOnce(&mut Engine) -> R,
{
    // Unpark the other thread, get it ready
    let handler = SAFEPOINT_HANDLER.lock().unwrap().clone();
    if let Some(x) = &handler {
        x.running_command
            .store(true, std::sync::atomic::Ordering::Relaxed);
        x.handle.thread().unpark();
    };

    let res = f(&mut acquire_engine_lock());

    if let Some(x) = handler {
        x.running_command
            .store(false, std::sync::atomic::Ordering::Relaxed);
        x.handle.thread().unpark();
    };

    res
}

pub fn try_enter_engine<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut Engine) -> R,
{
    let handler = SAFEPOINT_HANDLER.lock().unwrap().clone().unwrap();

    // If we're currently running a command, we need to try lock against
    // the lock since we don't want to lock up the engine explicitly.
    if handler
        .running_command
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        let res = match GLOBAL_ENGINE.try_lock() {
            Ok(mut v) => Some((f)(&mut v)),
            Err(_) => None,
        };

        res
    } else {
        handler
            .running_command
            .store(true, std::sync::atomic::Ordering::Relaxed);
        handler.handle.thread().unpark();

        let res = match GLOBAL_ENGINE.lock() {
            Ok(mut v) => Some((f)(&mut v)),
            Err(_) => None,
        };

        handler
            .running_command
            .store(false, std::sync::atomic::Ordering::Relaxed);
        handler.handle.thread().unpark();

        res
    }
}

pub struct SafepointHandler {
    running_command: Arc<AtomicBool>,
    handle: std::thread::JoinHandle<()>,
}

pub struct InterruptHandler {
    controller: ThreadStateController,
    running: Arc<AtomicBool>,
    handle: std::thread::JoinHandle<()>,
}

pub fn with_interrupt_handler<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let handler = INTERRUPT_HANDLER.lock().unwrap().clone().unwrap();
    handler
        .running
        .store(true, std::sync::atomic::Ordering::Relaxed);

    handler.handle.thread().unpark();

    let res = (f)();

    handler.controller.resume();
    handler
        .running
        .store(false, std::sync::atomic::Ordering::Relaxed);

    res
}

static BUFFER_EXTENSION_KEYMAP: Lazy<RwLock<BufferExtensionKeyMap>> = Lazy::new(|| {
    RwLock::new(BufferExtensionKeyMap {
        map: HashMap::new(),
        reverse: HashMap::new(),
    })
});

fn reset_buffer_extension_keymap() {
    let mut guard = BUFFER_EXTENSION_KEYMAP.write().unwrap();
    guard.map.clear();
    guard.reverse.clear();
}

enum LspKind {
    Call(RootedSteelVal),
    Notification(RootedSteelVal),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
struct LspCallRegistryId {
    lsp_name: String,
    event_name: String,
    generation: usize,
}

struct LspCallRegistry {
    map: HashMap<LspCallRegistryId, LspKind>,
}

static LSP_CALL_REGISTRY: Lazy<RwLock<LspCallRegistry>> = Lazy::new(|| {
    RwLock::new(LspCallRegistry {
        map: HashMap::new(),
    })
});

fn reset_lsp_call_registry() {
    LSP_CALL_REGISTRY.write().unwrap().map.clear();
}

fn register_lsp_call_callback(lsp: String, kind: String, function: SteelVal) {
    let rooted = function.as_rooted();

    let id = LspCallRegistryId {
        lsp_name: lsp,
        event_name: kind,
        generation: load_generation(),
    };

    LSP_CALL_REGISTRY
        .write()
        .unwrap()
        .map
        .insert(id, LspKind::Call(rooted));
}

fn register_lsp_notification_callback(lsp: String, kind: String, function: SteelVal) {
    let rooted = function.as_rooted();

    let id = LspCallRegistryId {
        lsp_name: lsp,
        event_name: kind,
        generation: load_generation(),
    };

    LSP_CALL_REGISTRY
        .write()
        .unwrap()
        .map
        .insert(id, LspKind::Notification(rooted));
}

fn send_arbitrary_lsp_notification(
    cx: &mut Context,
    name: SteelString,
    method: SteelString,
    params: Option<SteelVal>,
) -> anyhow::Result<()> {
    let argument = params.map(|x| serde_json::Value::try_from(x).unwrap());

    let (_view, doc) = current!(cx.editor);

    let language_server_id = anyhow::Context::context(
        doc.language_servers().find(|x| x.name() == name.as_str()),
        "Unable to find the language server specified!",
    )?
    .id();

    let language_server = cx
        .editor
        .language_server_by_id(language_server_id)
        .ok_or(anyhow::anyhow!("Failed to find a language server by id"))?;

    // Send the notification using the custom method and arguments
    language_server.send_custom_notification(method.to_string(), argument)?;

    Ok(())
}

pub struct BufferExtensionKeyMap {
    map: HashMap<String, EmbeddedKeyMap>,
    reverse: HashMap<usize, String>,
}

impl BufferExtensionKeyMap {
    fn get_extension(&self, extension: &str) -> Option<&EmbeddedKeyMap> {
        self.map.get(extension)
    }

    fn get_doc_id(&self, id: usize) -> Option<&EmbeddedKeyMap> {
        self.reverse.get(&id).and_then(|x| self.map.get(x))
    }
}

pub fn get_extension_keymap() -> RwLockReadGuard<'static, BufferExtensionKeyMap> {
    BUFFER_EXTENSION_KEYMAP.read().unwrap()
}

fn add_extension_or_labeled_keymap(label: String, keymap: EmbeddedKeyMap) {
    BUFFER_EXTENSION_KEYMAP
        .write()
        .unwrap()
        .map
        .insert(label, keymap);
}

fn add_reverse_mapping(key: usize, label: String) {
    BUFFER_EXTENSION_KEYMAP
        .write()
        .unwrap()
        .reverse
        .insert(key, label);
}

fn load_component_api(engine: &mut Engine, generate_sources: bool) {
    let module = helix_component_module(generate_sources);

    if generate_sources {
        configure_lsp_builtins("component", &module);
    }

    engine.register_module(module);
}

fn load_keymap_api(engine: &mut Engine, generate_sources: bool) {
    let mut module = BuiltInModule::new("helix/core/keymaps");

    module.register_fn("helix-empty-keymap", empty_keymap);
    module.register_fn("helix-default-keymap", default_keymap);
    module.register_fn("helix-merge-keybindings", merge_keybindings);
    module.register_fn("helix-string->keymap", string_to_embedded_keymap);
    module.register_fn("keymap?", is_keymap);
    module.register_fn("helix-deep-copy-keymap", deep_copy_keymap);

    module.register_fn(
        "#%add-extension-or-labeled-keymap",
        add_extension_or_labeled_keymap,
    );

    module.register_fn("#%add-reverse-mapping", add_reverse_mapping);

    // This should be associated with a corresponding scheme module to wrap this up
    module.register_fn("keymap-update-documentation!", update_documentation);

    if generate_sources {
        configure_lsp_builtins("keymap", &module)
    }

    engine.register_module(module);
}

pub fn format_docstring(doc: &str) -> String {
    let mut docstring = doc
        .lines()
        .map(|x| {
            let mut line = ";;".to_string();
            line.push_str(x);
            line.push('\n');
            line
        })
        .collect::<String>();

    docstring.pop();

    docstring
}

fn load_static_commands(engine: &mut Engine, generate_sources: bool) {
    let mut module = BuiltInModule::new("helix/core/static");

    let mut builtin_static_command_module = if generate_sources {
        "(require-builtin helix/core/static as helix.static.)".to_string()
    } else {
        "".to_string()
    };

    for command in TYPABLE_COMMAND_LIST {
        let func = |cx: &mut Context| {
            let mut cx = compositor::Context {
                editor: cx.editor,
                scroll: None,
                jobs: cx.jobs,
            };

            (command.fun)(&mut cx, Args::default(), PromptEvent::Validate)
        };

        module.register_fn(command.name, func);
    }

    // Register everything in the static command list as well
    // These just accept the context, no arguments
    for command in MappableCommand::STATIC_COMMAND_LIST {
        if let MappableCommand::Static { name, fun, doc } = command {
            module.register_fn(name, fun);

            if generate_sources {
                let docstring = format_docstring(doc);

                builtin_static_command_module.push_str(&format!(
                    r#"
(provide {})
;;@doc
{}
(define ({})
    (helix.static.{} *helix.cx*))
"#,
                    name, docstring, name, name
                ));
            }
        }
    }

    let mut template_function_arity_1 = |name: &str, doc: &str| {
        if generate_sources {
            let docstring = format_docstring(doc);

            builtin_static_command_module.push_str(&format!(
                r#"
(provide {})
;;@doc
{}
(define ({} arg)
    (helix.static.{} *helix.cx* arg))
"#,
                name, docstring, name, name
            ));
        }
    };

    macro_rules! function1 {
        ($name:expr, $function:expr, $doc:expr) => {{
            module.register_fn($name, $function);
            template_function_arity_1($name, $doc);
        }};
    }

    // Adhoc static commands that probably needs evaluating
    // Arity 1
    function1!(
        "insert_char",
        insert_char,
        "Insert a given character at the cursor cursor position"
    );
    function1!(
        "insert_string",
        insert_string,
        "Insert a given string at the current cursor position"
    );

    function1!(
        "set-current-selection-object!",
        set_selection,
        "Update the selection object to the current selection within the editor"
    );
    function1!(
        "push-range-to-selection!",
        push_range_to_selection,
        "Push a new range to a selection. The new selection will be the primary one"
    );
    function1!(
        "set-current-selection-primary-index!",
        set_selection_primary_index,
        "Set the primary index of the current selection"
    );
    function1!(
        "remove-current-selection-range!",
        remove_selection_range,
        "Remove a range from the current selection"
    );

    function1!(
        "regex-selection",
        regex_selection,
        "Run the given regex within the existing buffer"
    );

    function1!(
        "replace-selection-with",
        replace_selection,
        "Replace the existing selection with the given string"
    );

    function1!(
        "enqueue-expression-in-engine",
        run_expression_in_engine,
        "Enqueue an expression to run at the top level context, 
        after the existing function context has exited."
    );

    function1!(
        "get-current-line-character",
        current_line_character,
        "Returns the current column number with the given position encoding"
    );

    let mut template_function_arity_0 = |name: &str, doc: &str| {
        if generate_sources {
            let docstring = format_docstring(doc);

            builtin_static_command_module.push_str(&format!(
                r#"
(provide {})
;;@doc
{}
(define ({})
    (helix.static.{} *helix.cx*))
"#,
                name, docstring, name, name
            ));
        }
    };

    macro_rules! function0 {
        ($name:expr, $function:expr, $doc:expr) => {{
            module.register_fn($name, $function);
            template_function_arity_0($name, $doc);
        }};
    }

    function0!(
        "cx->current-file",
        current_path,
        "Get the currently focused file path"
    );

    function0!(
        "current_selection",
        get_selection,
        "Returns the current selection as a string"
    );
    function0!(
        "current-selection->string",
        get_selection,
        "Returns the current selection as a string"
    );
    function0!("load-buffer!", load_buffer, "Evaluates the current buffer");
    function0!(
        "current-highlighted-text!",
        get_highlighted_text,
        "Returns the currently highlighted text as a string"
    );
    function0!(
        "get-current-line-number",
        current_line_number,
        "Returns the current line number"
    );
    function0!(
        "get-current-column-number",
        current_column_number,
        "Returns the visual current column number of unicode graphemes"
    );
    function0!(
        "current-selection-object",
        current_selection,
        "Returns the current selection object"
    );
    function0!(
        "get-helix-cwd",
        get_helix_cwd,
        "Returns the current working directly that helix is using"
    );
    function0!(
        "move-window-far-left",
        move_window_to_the_left,
        "Moves the current window to the far left"
    );
    function0!(
        "move-window-far-right",
        move_window_to_the_right,
        "Moves the current window to the far right"
    );

    let mut template_function_no_context = |name: &str, doc: &str| {
        if generate_sources {
            let docstring = format_docstring(doc);

            builtin_static_command_module.push_str(&format!(
                r#"
(provide {})
;;@doc
{}
(define {} helix.static.{})                
            "#,
                name, docstring, name, name
            ))
        }
    };

    macro_rules! no_context {
        ($name:expr, $function:expr, $doc:expr) => {{
            module.register_fn($name, $function);
            template_function_no_context($name, $doc);
        }};
    }

    no_context!(
        "selection->primary-index",
        |sel: Selection| sel.primary_index(),
        "Returns index of the primary selection"
    );
    no_context!(
        "selection->primary-range",
        |sel: Selection| sel.primary(),
        "Returns the range for primary selection"
    );
    no_context!(
        "selection->ranges",
        |sel: Selection| sel.ranges().to_vec(),
        "Returns all ranges of the selection"
    );
    no_context!(
        "range-anchor",
        |range: Range| range.anchor,
        "Get the anchor of the range: the side that doesn't move when extending."
    );
    no_context!(
        "range->from",
        |range: Range| range.from(),
        "Get the start of the range"
    );
    no_context!(
        "range-head",
        |range: Range| range.head,
        "Get the head of the range, moved when extending."
    );
    no_context!(
        "range->to",
        |range: Range| range.to(),
        "Get the end of the range"
    );
    no_context!(
        "range->span",
        |range: Range| (range.from(), range.to()),
        "Get the span of the range (from, to)"
    );

    no_context!(
        "range",
        Range::new,
        r#"Construct a new range object

```scheme
(range anchor head) -> Range?
```
        "#
    );
    no_context!(
        "range->selection",
        |range: Range| Selection::from(range),
        "Convert a range into a selection"
    );

    module.register_fn("get-helix-scm-path", get_helix_scm_path);
    module.register_fn("get-init-scm-path", get_init_scm_path);

    template_function_no_context(
        "get-helix-scm-path",
        "Returns the path to the helix.scm file as a string",
    );
    template_function_no_context(
        "get-init-scm-path",
        "Returns the path to the init.scm file as a string",
    );

    if generate_sources {
        if let Some(mut target_directory) = alternative_runtime_search_path() {
            if !target_directory.exists() {
                std::fs::create_dir_all(&target_directory).unwrap();
            }

            target_directory.push("static.scm");

            std::fs::write(target_directory, &builtin_static_command_module).unwrap();
        }

        engine.register_steel_module(
            "helix/static.scm".to_string(),
            builtin_static_command_module,
        );
    }

    if generate_sources {
        configure_lsp_builtins("static", &module);
    }

    engine.register_module(module);
}

fn load_typed_commands(engine: &mut Engine, generate_sources: bool) {
    let mut module = BuiltInModule::new("helix/core/typable".to_string());

    let mut builtin_typable_command_module = if generate_sources {
        "(require-builtin helix/core/typable as helix.)".to_string()
    } else {
        "".to_string()
    };

    // Register everything in the typable command list. Now these are all available
    for command in TYPABLE_COMMAND_LIST {
        let func = move |cx: &mut Context, args: Vec<Cow<'static, str>>| {
            let mut cx = compositor::Context {
                editor: cx.editor,
                scroll: None,
                jobs: cx.jobs,
            };

            let mut verified_args = Args::new(command.signature, true);
            for arg in args {
                verified_args.push(arg)?;
            }

            verified_args
                .finish()
                .map_err(|e| anyhow::Error::msg(e.to_string()))?;

            (command.fun)(&mut cx, verified_args, PromptEvent::Validate)
        };

        module.register_fn(command.name, func);

        if generate_sources {
            // Create an ephemeral builtin module to reference until I figure out how
            // to wrap the functions with a reference to the engine context better.
            builtin_typable_command_module.push_str(&format!(
                r#"
(provide {})

;;@doc
{}
(define ({} . args)
    (helix.{} *helix.cx* args))
"#,
                command.name,
                format_docstring(command.doc),
                command.name,
                command.name
            ));
        }
    }

    if generate_sources {
        if let Some(mut target_directory) = alternative_runtime_search_path() {
            if !target_directory.exists() {
                std::fs::create_dir_all(&target_directory).unwrap();
            }

            target_directory.push("commands.scm");

            std::fs::write(target_directory, &builtin_typable_command_module).unwrap();
        }

        engine.register_steel_module(
            "helix/commands.scm".to_string(),
            builtin_typable_command_module,
        );
    }

    if generate_sources {
        configure_lsp_builtins("typed", &module);
    }

    engine.register_module(module);
}

fn get_option_value(cx: &mut Context, option: String) -> anyhow::Result<SteelVal> {
    let key_error = || anyhow::anyhow!("Unknown key `{}`", option);

    let config = serde_json::json!(std::ops::Deref::deref(&cx.editor.config()));
    let pointer = format!("/{}", option.replace('.', "/"));
    let value = config.pointer(&pointer).ok_or_else(key_error)?;
    Ok(value.to_owned().into_steelval().unwrap())
}

// Indent guides configurations
fn ig_render(config: &mut IndentGuidesConfig, option: bool) {
    config.render = option;
}

fn ig_character(config: &mut IndentGuidesConfig, option: char) {
    config.character = option;
}

fn ig_skip_levels(config: &mut IndentGuidesConfig, option: u8) {
    config.skip_levels = option;
}

// Whitespace configurations
fn ws_visible(config: &mut WhitespaceConfig, option: bool) {
    let value = if option {
        WhitespaceRenderValue::All
    } else {
        WhitespaceRenderValue::None
    };
    config.render = WhitespaceRender::Basic(value);
}

fn ws_chars(config: &mut WhitespaceConfig, option: HashMap<SteelVal, char>) -> anyhow::Result<()> {
    for (k, v) in option {
        match k {
            SteelVal::StringV(s) | SteelVal::SymbolV(s) => match s.as_str() {
                "space" => config.characters.space = v,
                "tab" => config.characters.tab = v,
                "nbsp" => config.characters.nbsp = v,
                "nnbsp" => config.characters.nnbsp = v,
                "newline" => config.characters.newline = v,
                "tabpad" => config.characters.tabpad = v,
                unknown => anyhow::bail!("Unrecognized key: {}", unknown),
            },
            other => anyhow::bail!("Unrecognized key option: {}", other),
        }
    }
    Ok(())
}

fn ws_render(config: &mut WhitespaceConfig, option: HashMap<SteelVal, bool>) -> anyhow::Result<()> {
    #[derive(Default)]
    struct RenderFlags {
        space: Option<WhitespaceRenderValue>,
        tab: Option<WhitespaceRenderValue>,
        nbsp: Option<WhitespaceRenderValue>,
        nnbsp: Option<WhitespaceRenderValue>,
        newline: Option<WhitespaceRenderValue>,
        default: Option<WhitespaceRenderValue>,
    }

    let mut base = match config.render {
        WhitespaceRender::Basic(v) => RenderFlags {
            default: Some(v.clone()),
            space: Some(v.clone()),
            nbsp: Some(v.clone()),
            nnbsp: Some(v.clone()),
            tab: Some(v.clone()),
            newline: Some(v.clone()),
        },
        WhitespaceRender::Specific { .. } => RenderFlags::default(),
    };

    for (k, v) in option {
        let value = if v {
            WhitespaceRenderValue::All
        } else {
            WhitespaceRenderValue::None
        };
        match k {
            SteelVal::StringV(s) | SteelVal::SymbolV(s) => match s.as_str() {
                "space" => base.space = Some(value),
                "tab" => base.tab = Some(value),
                "nbsp" => base.nbsp = Some(value),
                "nnbsp" => base.nnbsp = Some(value),
                "newline" => base.newline = Some(value),
                "default" => base.default = Some(value),
                unknown => anyhow::bail!("Unrecognized key: {}", unknown),
            },
            unknown => anyhow::bail!("Unrecognized key: {}", unknown),
        }
    }

    config.render = WhitespaceRender::Specific {
        default: base.default,
        space: base.space,
        nbsp: base.nbsp,
        nnbsp: base.nnbsp,
        tab: base.tab,
        newline: base.newline,
    };

    Ok(())
}

// File picker configurations
fn fp_hidden(config: &mut FilePickerConfig, option: bool) {
    config.hidden = option;
}

fn fp_follow_symlinks(config: &mut FilePickerConfig, option: bool) {
    config.follow_symlinks = option;
}

fn fp_deduplicate_links(config: &mut FilePickerConfig, option: bool) {
    config.deduplicate_links = option;
}

fn fp_parents(config: &mut FilePickerConfig, option: bool) {
    config.parents = option;
}

fn fp_ignore(config: &mut FilePickerConfig, option: bool) {
    config.ignore = option;
}

fn fp_git_ignore(config: &mut FilePickerConfig, option: bool) {
    config.git_ignore = option;
}

fn fp_git_global(config: &mut FilePickerConfig, option: bool) {
    config.git_global = option;
}

fn fp_git_exclude(config: &mut FilePickerConfig, option: bool) {
    config.git_exclude = option;
}

fn fp_max_depth(config: &mut FilePickerConfig, option: Option<usize>) {
    config.max_depth = option;
}

// Soft wrap configurations
fn sw_enable(config: &mut SoftWrap, option: Option<bool>) {
    config.enable = option;
}

fn sw_max_wrap(config: &mut SoftWrap, option: Option<u16>) {
    config.max_wrap = option;
}

fn sw_max_indent_retain(config: &mut SoftWrap, option: Option<u16>) {
    config.max_indent_retain = option;
}

fn sw_wrap_indicator(config: &mut SoftWrap, option: Option<String>) {
    config.wrap_indicator = option;
}

fn wrap_at_text_width(config: &mut SoftWrap, option: Option<bool>) {
    config.wrap_at_text_width = option;
}

// Attempt to fuss with the configuration?
fn dynamic_set_option(
    configuration: &HelixConfiguration,
    key: String,
    value: SteelVal,
) -> anyhow::Result<()> {
    let key = key.to_lowercase();

    let key_error = || anyhow::anyhow!("Unknown key `{}`", key);

    let mut config = serde_json::json!(configuration.load_config().editor);
    let pointer = format!("/{}", key.replace('.', "/"));
    let jvalue = config.pointer_mut(&pointer).ok_or_else(key_error)?;

    let cloned = value.clone();
    let field_error = move |_| anyhow::anyhow!("Could not parse field `{}`", cloned);
    *jvalue = serde_json::Value::try_from(value)?;

    let config = serde_json::from_value(config).map_err(field_error)?;

    let mut new_config = configuration.load_config();
    new_config.editor = config;

    configuration.store_config(new_config);

    Ok(())
}

fn load_configuration_api(engine: &mut Engine, generate_sources: bool) {
    let mut module = BuiltInModule::new("helix/core/configuration");

    module.register_fn(
        "register-lsp-notification-handler",
        register_lsp_notification_callback,
    );

    module.register_fn("register-lsp-call-handler", register_lsp_call_callback);

    module.register_fn("update-configuration!", |ctx: &mut Context| {
        ctx.editor
            .config_events
            .0
            .send(ConfigEvent::Change)
            .unwrap();
    });

    module.register_fn("get-config-option-value", get_option_value);

    module.register_fn("set-configuration-for-file!", set_configuration_for_file);

    module
        .register_fn(
            "get-language-config",
            HelixConfiguration::get_language_config,
        )
        // .register_fn(
        //     "get-language-config-by-filename",
        //     HelixConfiguration::get_individual_language_config_for_filename,
        // )
        .register_fn(
            "set-language-config!",
            HelixConfiguration::update_individual_language_config,
        );

    module.register_fn(
        "set-lsp-config!",
        HelixConfiguration::update_language_server_config,
    );

    module.register_fn(
        "update-language-config!",
        HelixConfiguration::update_language_config,
    );

    module.register_fn(
        "refresh-all-language-configs!",
        update_configuration_for_all_open_documents,
    );

    module
        .register_fn("raw-cursor-shape", CursorShapeConfig::default)
        .register_fn(
            "raw-cursor-shape-set!",
            |value: SteelVal, mode: String, shape: String| -> anyhow::Result<SteelVal> {
                let mut config = CursorShapeConfig::as_mut_ref(&value)?;

                let mode = match mode.as_str() {
                    "normal" => Mode::Normal,
                    "select" => Mode::Select,
                    "insert" => Mode::Insert,
                    _ => anyhow::bail!("Unable to match mode from string: {}", mode),
                };

                let kind = match shape.as_str() {
                    "block" => CursorKind::Block,
                    "bar" => CursorKind::Bar,
                    "underline" => CursorKind::Underline,
                    "hidden" => CursorKind::Hidden,
                    _ => anyhow::bail!("Unable to match cursor kind from string: {}", shape),
                };

                config.update(mode, kind);
                drop(config);
                Ok(value)
            },
        );

    module
        .register_fn("raw-file-picker", FilePickerConfig::default)
        .register_fn("register-file-picker", HelixConfiguration::file_picker)
        .register_fn("fp-hidden", fp_hidden)
        .register_fn("fp-follow-symlinks", fp_follow_symlinks)
        .register_fn("fp-deduplicate-links", fp_deduplicate_links)
        .register_fn("fp-parents", fp_parents)
        .register_fn("fp-ignore", fp_ignore)
        .register_fn("fp-git-ignore", fp_git_ignore)
        .register_fn("fp-git-global", fp_git_global)
        .register_fn("fp-git-exclude", fp_git_exclude)
        .register_fn("fp-max-depth", fp_max_depth);

    module
        .register_fn("raw-soft-wrap", SoftWrap::default)
        .register_fn("register-soft-wrap", HelixConfiguration::soft_wrap)
        .register_fn("sw-enable", sw_enable)
        .register_fn("sw-max-wrap", sw_max_wrap)
        .register_fn("sw-max-indent-retain", sw_max_indent_retain)
        .register_fn("sw-wrap-indicator", sw_wrap_indicator)
        .register_fn("sw-wrap-at-text-width", wrap_at_text_width);

    module
        .register_fn("raw-whitespace", || WhitespaceConfig::default())
        .register_fn("register-whitespace", HelixConfiguration::whitespace)
        .register_fn("ws-visible", ws_visible)
        .register_fn("ws-chars", ws_chars)
        .register_fn("ws-render", ws_render);

    module
        .register_fn("raw-indent-guides", || IndentGuidesConfig::default())
        .register_fn("register-indent-guides", HelixConfiguration::indent_guides)
        .register_fn("ig-render", ig_render)
        .register_fn("ig-character", ig_character)
        .register_fn("ig-skip-levels", ig_skip_levels);

    module
        .register_fn("scrolloff", HelixConfiguration::scrolloff)
        .register_fn("scroll_lines", HelixConfiguration::scroll_lines)
        .register_fn("mouse", HelixConfiguration::mouse)
        .register_fn("shell", HelixConfiguration::shell)
        .register_fn(
            "jump-label-alphabet",
            HelixConfiguration::jump_label_alphabet,
        )
        .register_fn("line-number", HelixConfiguration::line_number)
        .register_fn("cursorline", HelixConfiguration::cursorline)
        .register_fn("cursorcolumn", HelixConfiguration::cursorcolumn)
        .register_fn("middle-click-paste", HelixConfiguration::middle_click_paste)
        .register_fn("auto-pairs", HelixConfiguration::auto_pairs)
        // Specific constructors for the auto pairs configuration
        .register_fn("auto-pairs-default", |enabled: bool| {
            AutoPairConfig::Enable(enabled)
        })
        .register_fn("auto-pairs-map", |map: HashMap<char, char>| {
            AutoPairConfig::Pairs(map)
        })
        // TODO: Finish this up
        .register_fn("auto-save-default", AutoSave::default)
        .register_fn(
            "auto-save-after-delay-enable",
            HelixConfiguration::auto_save_after_delay_enable,
        )
        .register_fn(
            "inline-diagnostics-cursor-line-enable",
            HelixConfiguration::inline_diagnostics_cursor_line_enable,
        )
        .register_fn(
            "inline-diagnostics-other-lines-enable",
            HelixConfiguration::inline_diagnostics_other_lines_enable,
        )
        .register_fn(
            "inline-diagnostics-end-of-line-enable",
            HelixConfiguration::inline_diagnostics_end_of_line_enable,
        )
        .register_fn(
            "inline-diagnostics-min-diagnostics-width",
            HelixConfiguration::inline_diagnostics_min_diagnostic_width,
        )
        .register_fn(
            "inline-diagnostics-prefix-len",
            HelixConfiguration::inline_diagnostics_prefix_len,
        )
        .register_fn(
            "inline-diagnostics-max-wrap",
            HelixConfiguration::inline_diagnostics_max_wrap,
        )
        .register_fn(
            "inline-diagnostics-max-diagnostics",
            HelixConfiguration::inline_diagnostics_max_diagnostics,
        )
        .register_fn("auto-completion", HelixConfiguration::auto_completion)
        .register_fn("auto-format", HelixConfiguration::auto_format)
        .register_fn("auto-save", HelixConfiguration::auto_save)
        .register_fn("text-width", HelixConfiguration::text_width)
        .register_fn("idle-timeout", HelixConfiguration::idle_timeout)
        .register_fn("completion-timeout", HelixConfiguration::completion_timeout)
        .register_fn(
            "preview-completion-insert",
            HelixConfiguration::preview_completion_insert,
        )
        .register_fn(
            "completion-trigger-len",
            HelixConfiguration::completion_trigger_len,
        )
        .register_fn("completion-replace", HelixConfiguration::completion_replace)
        .register_fn("auto-info", HelixConfiguration::auto_info)
        .register_fn("#%raw-cursor-shape", HelixConfiguration::cursor_shape)
        .register_fn("true-color", HelixConfiguration::true_color)
        .register_fn(
            "insert-final-newline",
            HelixConfiguration::insert_final_newline,
        )
        .register_fn("color-modes", HelixConfiguration::color_modes)
        .register_fn("gutters", HelixConfiguration::gutters)
        // .register_fn("file-picker", HelixConfiguration::file_picker)
        .register_fn("statusline", HelixConfiguration::statusline)
        .register_fn("undercurl", HelixConfiguration::undercurl)
        .register_fn("search", HelixConfiguration::search)
        .register_fn("lsp", HelixConfiguration::lsp)
        .register_fn("terminal", HelixConfiguration::terminal)
        .register_fn("rulers", HelixConfiguration::rulers)
        .register_fn("bufferline", HelixConfiguration::bufferline)
        .register_fn(
            "workspace-lsp-roots",
            HelixConfiguration::workspace_lsp_roots,
        )
        .register_fn(
            "default-line-ending",
            HelixConfiguration::default_line_ending,
        )
        .register_fn("smart-tab", HelixConfiguration::smart_tab)
        .register_fn("rainbow-brackets", HelixConfiguration::rainbow_brackets);

    // Keybinding stuff
    module
        .register_fn("keybindings", HelixConfiguration::keybindings)
        .register_fn("get-keybindings", HelixConfiguration::get_keybindings)
        .register_fn("set-keybindings!", HelixConfiguration::set_keybindings)
        .register_fn("set-option!", dynamic_set_option);

    if generate_sources {
        let mut builtin_configuration_module =
            r#"(require-builtin helix/core/configuration as helix.)

(provide register-lsp-notification-handler)

;;@doc
;; Register a callback to be called on LSP notifications sent from the server -> client
;; that aren't currently handled by Helix as a built in.
;;
;; ```scheme
;; (register-lsp-notification-handler lsp-name event-name handler)
;; ```
;;
;; * lsp-name : string?
;; * event-name : string?
;; * function : (-> hash? any?) ;; Function where the first argument is the parameters
;;
;; # Examples
;; ```
;; (register-lsp-notification-handler "dart"
;;                                    "dart/textDocument/publishClosingLabels"
;;                                    (lambda (args) (displayln args)))
;; ```
(define register-lsp-notification-handler helix.register-lsp-notification-handler)

(provide register-lsp-call-handler)

;;@doc
;; Register a callback to be called on LSP calls sent from the server -> client
;; that aren't currently handled by Helix as a built in.
;;
;; ```scheme
;; (register-lsp-call-handler lsp-name event-name handler)
;; ```
;;
;; * lsp-name : string?
;; * event-name : string?
;; * function : (-> hash? any?) ;; Function where the first argument is the parameters
;;
;; # Examples
;; ```
;; (register-lsp-call-handler "dart"
;;                                    "dart/textDocument/publishClosingLabels"
;;                                    (lambda (call-id args) (displayln args)))
;; ```
(define register-lsp-call-handler helix.register-lsp-call-handler)

(provide set-option!)
(define (set-option! key value)
    (helix.set-option! *helix.config* key value))
                
(provide define-lsp)
(define-syntax define-lsp
  (syntax-rules (#%crunch #%name #%conf)
    ;; Other generic keys
    [(_ #%crunch #%name name #%conf conf (key (inner-key value) ...))
     (set-lsp-config! name
                      (hash-insert conf
                                   (quote key)
                                   (transduce (list (list (quote inner-key) value) ...)
                                              (into-hashmap))))]

    [(_ #%crunch #%name name #%conf conf (key (inner-key value) ...) remaining ...)
     ;  ;; Crunch the remaining stuff
     (define-lsp #%crunch
          #%name
          name
          #%conf
          (hash-insert conf
                       (quote key)
                       (transduce (list (list (quote inner-key) value) ...) (into-hashmap)))
          remaining ...)]

    ;; Other generic keys
    [(_ #%crunch #%name name #%conf conf (key value))
     (set-lsp-config! name (hash-insert conf (quote key) value))]

    [(_ #%crunch #%name name #%conf conf (key value) remaining ...)
     ;  ;; Crunch the remaining stuff
     (define-lsp #%crunch #%name name #%conf (hash-insert conf (quote key) value) remaining ...)]

    [(_ name (key value ...) ...)
     (define-lsp #%crunch #%name name #%conf (hash "name" name) (key value ...) ...)]

    [(_ name (key value)) (define-lsp #%crunch #%name name #%conf (hash "name" name) (key value))]

    [(_ name (key value) ...) (define-lsp #%crunch #%name name #%conf (hash "name" name) (key value) ...)]))

(provide define-language)
(define-syntax define-language
  (syntax-rules (#%crunch #%name #%conf)

    ;; Other generic keys
    [(_ #%crunch #%name name #%conf conf (key (inner-key value) ...))
     (update-language-config! name
                              (hash-insert conf
                                           (quote key)
                                           (transduce (list (list (quote inner-key) value) ...)
                                                      (into-hashmap))))]

    [(_ #%crunch #%name name #%conf conf (key (inner-key value) ...) remaining ...)
     ;  ;; Crunch the remaining stuff
     (define-language #%crunch
               #%name
               name
               #%conf
               (hash-insert conf
                            (quote key)
                            (transduce (list (list (quote inner-key) value) ...) (into-hashmap)))
               remaining ...)]

    ;; Other generic keys
    [(_ #%crunch #%name name #%conf conf (key value))
     (update-language-config! name (hash-insert conf (quote key) value))]

    [(_ #%crunch #%name name #%conf conf (key value) remaining ...)
     ;  ;; Crunch the remaining stuff
     (define-language #%crunch #%name name #%conf (hash-insert conf (quote key) value) remaining ...)]

    [(_ name (key value ...) ...)
     (define-language #%crunch #%name name #%conf (hash "name" name) (key value ...) ...)]

    [(_ name (key value)) (language #%crunch #%name name #%conf (hash "name" name) (key value))]

    [(_ name (key value) ...)
     (define-language #%crunch #%name name #%conf (hash "name" name) (key value) ...)]))
"#
            .to_string();

        builtin_configuration_module.push_str(
            r#"
(provide cursor-shape)
;;@doc
;; Shape for cursor in each mode
;;
;; (cursor-shape #:normal (normal 'block)
;;               #:select (select 'block)
;;               #:insert (insert 'block))
;;
;; # Examples
;; 
;; ```scheme
;; (cursor-shape #:normal 'block #:select 'underline #:insert 'bar)
;; ```
(define (cursor-shape #:normal (normal 'block)
                      #:select (select 'block)
                      #:insert (insert 'block))
    (define cursor-shape-config (helix.raw-cursor-shape))
    (helix.raw-cursor-shape-set! cursor-shape-config 'normal normal)
    (helix.raw-cursor-shape-set! cursor-shape-config 'select select)
    (helix.raw-cursor-shape-set! cursor-shape-config 'insert insert)
    (helix.#%raw-cursor-shape *helix.config* cursor-shape-config))                
            "#,
        );

        builtin_configuration_module.push_str(
            r#"
(provide refresh-all-language-configs!)
(define (refresh-all-language-configs!)
    (helix.refresh-all-language-configs! *helix.cx*))
            "#,
        );

        builtin_configuration_module.push_str(
            r#"
(provide update-configuration!)
(define (update-configuration!)
    (helix.update-configuration! *helix.config*))
"#,
        );

        builtin_configuration_module.push_str(
            r#"
(provide get-config-option-value)
(define (get-config-option-value arg)
    (helix.get-config-option-value *helix.cx* arg))
"#,
        );

        builtin_configuration_module.push_str(
            r#"
(provide set-configuration-for-file!)
(define (set-configuration-for-file! path config)
    (helix.set-configuration-for-file! *helix.cx* path config))
"#,
        );

        builtin_configuration_module.push_str(
            r#"
(provide set-lsp-config!)
;;@doc
;; Sets the language server config for a specific language server.
;;
;; ```scheme
;; (set-lsp-config! lsp config)
;; ```
;; * lsp : string?
;; * config: hash?
;;
;; This will overlay the existing configuration, much like the existing
;; toml definition does.
;;
;; Available options for the config hash are:
;; ```scheme
;; (hash "command" "<command>"
;;       "args" (list "args" ...)
;;       "environment" (hash "ENV" "VAR" ...)
;;       "config" (hash ...)
;;       "timeout" 100 ;; number
;;       "required-root-patterns" (listof "pattern" ...))
;;
;; ```
;;
;; # Examples
;; ```
;; (set-lsp-config! "jdtls"
;;    (hash "args" (list "-data" "/home/matt/code/java-scratch/workspace")))
;; ```
(define (set-lsp-config! lsp config)
    (helix.set-lsp-config! *helix.config* lsp config))
"#,
        );

        builtin_configuration_module.push_str(
            r#"
(provide update-language-config!)
(define (update-language-config! lsp config)
    (helix.update-language-config! *helix.config* lsp config)
    (refresh-all-language-configs!))
"#,
        );

        // Register the get keybindings function
        builtin_configuration_module.push_str(
            r#"
(provide get-keybindings)
(define (get-keybindings)
    (helix.get-keybindings *helix.config*))
"#,
        );

        let mut template_whitespace = |name: &str| {
            builtin_configuration_module.push_str(&format!(
                r#"
(provide {})
(define ({} arg)
    (lambda (picker) 
            (helix.{} picker arg)
            picker))
"#,
                name, name, name
            ))
        };
        let whitespace_functions = &["ws-visible", "ws-chars", "ws-render"];

        for name in whitespace_functions {
            template_whitespace(name);
        }

        let mut template_indent_guides = |name: &str| {
            builtin_configuration_module.push_str(&format!(
                r#"
(provide {})
(define ({} arg)
    (lambda (picker) 
            (helix.{} picker arg)
            picker))
"#,
                name, name, name
            ))
        };
        let indent_guides_functions = &["ig-render", "ig-character", "ig-skip-levels"];

        for name in indent_guides_functions {
            template_indent_guides(name);
        }

        let mut template_soft_wrap = |name: &str| {
            builtin_configuration_module.push_str(&format!(
                r#"
(provide {})
(define ({} arg)
    (lambda (picker) 
            (helix.{} picker arg)
            picker))
"#,
                name, name, name
            ));
        };

        let soft_wrap_functions = &[
            "sw-enable",
            "sw-max-wrap",
            "sw-max-indent-retain",
            "sw-wrap-indicator",
            "sw-wrap-at-text-width",
        ];

        for name in soft_wrap_functions {
            template_soft_wrap(name);
        }

        let mut template_file_picker_function = |name: &str| {
            builtin_configuration_module.push_str(&format!(
                r#"
(provide {})
(define ({} arg)
    (lambda (picker) 
            (helix.{} picker arg)
            picker))
"#,
                name, name, name
            ));
        };

        let file_picker_functions = &[
            "fp-hidden",
            "fp-follow-symlinks",
            "fp-deduplicate-links",
            "fp-parents",
            "fp-ignore",
            "fp-git-ignore",
            "fp-git-global",
            "fp-git-exclude",
            "fp-max-depth",
        ];

        for name in file_picker_functions {
            template_file_picker_function(name);
        }

        builtin_configuration_module.push_str(
            r#"
(provide file-picker-kw)
;;@doc
;; Sets the configuration for the file picker using keywords.
;;
;; ```scheme
;; (file-picker-kw #:hidden #t
;;                 #:follow-symlinks #t
;;                 #:deduplicate-links #t
;;                 #:parents #t
;;                 #:ignore #t
;;                 #:git-ignore #t
;;                 #:git-exclude #t
;;                 #:git-global #t
;;                 #:max-depth #f) ;; Expects either #f or an int?
;; ```
;; By default, max depth is `#f` while everything else is an int?
;;
;; To use this, call this in your `init.scm` or `helix.scm`:
;;
;; # Examples
;; ```scheme
;; (file-picker-kw #:hidden #f)
;; ```
(define (file-picker-kw
            #:hidden [hidden #t]
            #:follow-symlinks [follow-symlinks #t]
            #:deduplicate-links [deduplicate-links #t]
            #:parents [parents #t]
            #:ignore [ignore #t]
            #:git-ignore [git-ignore #t]
            #:git-global [git-global #t]
            #:git-exclude [git-exclude #t]
            #:max-depth [max-depth #f])

    (define picker (helix.raw-file-picker))
    (unless hidden (helix.fp-hidden picker hidden))
    (unless follow-symlinks (helix.fp-follow-symlinks picker follow-symlinks))
    (unless deduplicate-links (helix.fp-deduplicate-links picker deduplicate-links))
    (unless parents (helix.fp-parents picker parents))
    (unless ignore (helix.fp-ignore picker ignore))
    (unless git-ignore (helix.fp-git-ignore picker git-ignore))
    (unless git-global (helix.fp-git-global picker git-global))
    (unless git-exclude (helix.fp-git-exclude picker git-exclude))
    (when max-depth (helix.fp-max-depth picker max-depth))
    (helix.register-file-picker *helix.config* picker))
            "#,
        );

        builtin_configuration_module.push_str(
            r#"
(provide file-picker)
;;@doc
;; Sets the configuration for the file picker using var args.
;;
;; ```scheme
;; (file-picker . args)
;; ```
;;
;; The args are expected to be something of the value:
;; ```scheme
;; (-> FilePickerConfiguration? bool?)    
;; ```
;;
;; These other functions in this module which follow this behavior are all
;; prefixed `fp-`, and include:
;;
;; * fp-hidden
;; * fp-follow-symlinks
;; * fp-deduplicate-links
;; * fp-parents
;; * fp-ignore
;; * fp-git-ignore
;; * fp-git-global
;; * fp-git-exclude
;; * fp-max-depth
;; 
;; By default, max depth is `#f` while everything else is an int?
;;
;; To use this, call this in your `init.scm` or `helix.scm`:
;;
;; # Examples
;; ```scheme
;; (file-picker (fp-hidden #f) (fp-parents #f))
;; ```
(define (file-picker . args)
    (helix.register-file-picker
        *helix.config*
        (foldl (lambda (func config) (func config)) (helix.raw-file-picker) args)))
"#,
        );

        builtin_configuration_module.push_str(
            r#"
(provide soft-wrap-kw)
;;@doc
;; Sets the configuration for soft wrap using keyword args.
;;
;; ```scheme
;; (soft-wrap-kw #:enable #f
;;               #:max-wrap 20
;;               #:max-indent-retain 40
;;               #:wrap-indicator "↪"
;;               #:wrap-at-text-width #f)
;; ```
;;
;; The options are as follows:
;;
;; * #:enable:
;;   Soft wrap lines that exceed viewport width. Default to off
;; * #:max-wrap:
;;   Maximum space left free at the end of the line.
;;   This space is used to wrap text at word boundaries. If that is not possible within this limit
;;   the word is simply split at the end of the line.
;;
;;   This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
;;
;;   Default to 20
;; * #:max-indent-retain
;;   Maximum number of indentation that can be carried over from the previous line when softwrapping.
;;   If a line is indented further then this limit it is rendered at the start of the viewport instead.
;;
;;   This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
;; 
;;   Default to 40
;; * #:wrap-indicator
;;   Indicator placed at the beginning of softwrapped lines
;; 
;;   Defaults to ↪
;; * #:wrap-at-text-width
;;   Softwrap at `text_width` instead of viewport width if it is shorter
;; 
;; # Examples
;; ```scheme
;; (soft-wrap-kw #:sw-enable #t)
;; ```
(define (soft-wrap-kw #:enable [enable #f]
                      #:max-wrap [max-wrap 20]
                      #:max-indent-retain [max-indent-retain 40]
                      #:wrap-indicator [wrap-indicator 4]
                      #:wrap-at-text-width [wrap-at-text-width #f])
    (define sw (helix.raw-soft-wrap))
    (helix.sw-enable sw enable)
    (helix.sw-max-wrap sw max-wrap)
    (helix.sw-max-indent-retain sw max-indent-retain)
    (helix.sw-wrap-indicator sw wrap-indicator)
    (helix.sw-wrap-at-text-width sw wrap-at-text-width)
    (helix.register-soft-wrap *helix.config* sw))
"#,
        );

        builtin_configuration_module.push_str(
            r#"

(provide soft-wrap)
;;@doc
;; Sets the configuration for soft wrap using var args.
;;
;; ```scheme
;; (soft-wrap . args)
;; ```
;;
;; The args are expected to be something of the value:
;; ```scheme
;; (-> SoftWrapConfiguration? bool?)    
;; ```
;; The options are as follows:
;;
;; * sw-enable:
;;   Soft wrap lines that exceed viewport width. Default to off
;; * sw-max-wrap:
;;   Maximum space left free at the end of the line.
;;   This space is used to wrap text at word boundaries. If that is not possible within this limit
;;   the word is simply split at the end of the line.
;;
;;   This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
;;
;;   Default to 20
;; * sw-max-indent-retain
;;   Maximum number of indentation that can be carried over from the previous line when softwrapping.
;;   If a line is indented further then this limit it is rendered at the start of the viewport instead.
;;
;;   This is automatically hard-limited to a quarter of the viewport to ensure correct display on small views.
;; 
;;   Default to 40
;; * sw-wrap-indicator
;;   Indicator placed at the beginning of softwrapped lines
;; 
;;   Defaults to ↪
;; * sw-wrap-at-text-width
;;   Softwrap at `text_width` instead of viewport width if it is shorter
;;
;; # Examples
;; ```scheme
;; (soft-wrap (sw-enable #t))
;; ```
(define (soft-wrap . args)
    (helix.register-soft-wrap
        *helix.config*
        (foldl (lambda (func config) (func config)) (helix.raw-soft-wrap) args)))
"#,
        );

        builtin_configuration_module.push_str(&format!(
            r#"

(provide whitespace)
;;@doc
;; Sets the configuration for whitespace using var args.
;;
;; ```scheme
;; (whitespace . args)
;; ```
;;
;; The args are expected to be something of the value:
;; ```scheme
;; (-> WhitespaceConfiguration? bool?)    
;; ```
;; The options are as follows:
;;
;; * ws-visible:
;;   Show all visible whitespace, defaults to false
;; * ws-render:
;;   manually disable or enable characters
;;   render options (specified in hashmap):
;;```scheme
;;   (hash
;;     'space #f
;;     'nbsp #f
;;     'nnbsp #f
;;     'tab #f
;;     'newline #f)
;;```
;; * ws-chars:
;;   manually set visible whitespace characters with a hashmap
;;   character options (specified in hashmap):
;;```scheme
;;   (hash
;;     'space #\·
;;     'nbsp #\⍽
;;     'nnbsp #\␣
;;     'tab #\→
;;     'newline #\⏎
;;     ; Tabs will look like "→···" (depending on tab width)
;;     'tabpad #\·)
;;```
;; # Examples
;; ```scheme
;; (whitespace (ws-visible #t) (ws-chars (hash 'space #\·)) (ws-render (hash 'tab #f)))
;; ```
(define (whitespace . args)
    (helix.register-whitespace
        *helix.config*
        (foldl (lambda (func config) (func config)) (helix.raw-whitespace) args)))
"#,
        ));

        builtin_configuration_module.push_str(&format!(
            r#"

(provide indent-guides)
;;@doc
;; Sets the configuration for indent-guides using args
;;
;; ```scheme
;; (indent-guides . args)
;; ```
;;
;; The args are expected to be something of the value:
;; ```scheme
;; (-> IndentGuidesConfig? bool?)
;; ```
;; The options are as follows:
;;
;; * ig-render:
;;   Show indent guides, defaults to false
;; * ig-character:
;;   character used for indent guides, defaults to "╎"
;; * ig-skip-levels:
;;   amount of levels to skip, defaults to 1
;;
;; # Examples
;; ```scheme
;; (indent-guides (ig-render #t) (ig-character #\|) (ig-skip-levels 1))
;; ```
(define (indent-guides . args)
    (helix.register-indent-guides
        *helix.config*
        (foldl (lambda (func config) (func config)) (helix.raw-indent-guides) args)))
"#,
        ));

        let mut template_function_arity_1 = |name: &str, doc: &str| {
            let doc = format_docstring(doc);
            builtin_configuration_module.push_str(&format!(
                r#"
(provide {})
;;@doc
;;{}
(define ({} arg)
    (helix.{} *helix.config* arg))
"#,
                name, doc, name, name
            ));
        };

        let functions = &[
            ("scrolloff", "Padding to keep between the edge of the screen and the cursor when scrolling. Defaults to 5."),
            ("scroll_lines", "Number of lines to scroll at once. Defaults to 3
"),
            ("mouse", "Mouse support. Defaults to true."),
            ("shell", r#"Shell to use for shell commands. Defaults to ["cmd", "/C"] on Windows and ["sh", "-c"] otherwise."#),
            ("jump-label-alphabet", r#"The characters that are used to generate two character jump labels. Characters at the start of the alphabet are used first. Defaults to "abcdefghijklmnopqrstuvwxyz""#),
            ("line-number", "Line number mode. Defaults to 'absolute, set to 'relative for relative line numbers"),
            ("cursorline", "Highlight the lines cursors are currently on. Defaults to false"),
            ("cursorcolumn", "Highlight the columns cursors are currently on. Defaults to false"),
            ("middle-click-paste", "Middle click paste support. Defaults to true"),
            ("auto-pairs", r#"
Automatic insertion of pairs to parentheses, brackets,
etc. Optionally, this can be a list of 2-tuples to specify a
global list of characters to pair. Defaults to true."#),
            ("auto-completion", "Automatic auto-completion, automatically pop up without user trigger. Defaults to true."),
            // TODO: Put in path_completion
            ("auto-format", "Automatic formatting on save. Defaults to true."),
            ("auto-save", r#"Automatic save on focus lost and/or after delay.
Time delay in milliseconds since last edit after which auto save timer triggers.
Time delay defaults to false with 3000ms delay. Focus lost defaults to false.
                "#),
            ("text-width", "Set a global text_width"),
            ("idle-timeout", r#"Time in milliseconds since last keypress before idle timers trigger.
Used for various UI timeouts. Defaults to 250ms."#),
            ("completion-timeout", r#"
Time in milliseconds after typing a word character before auto completions
are shown, set to 5 for instant. Defaults to 250ms.
                "#),
            ("preview-completion-insert", "Whether to insert the completion suggestion on hover. Defaults to true."),
            ("completion-trigger-len", "Length to trigger completions"),
            ("completion-replace", r#"Whether to instruct the LSP to replace the entire word when applying a completion
 or to only insert new text
"#),
            ("auto-info", "Whether to display infoboxes. Defaults to true."),
            // ("cursor-shape", "Shape for cursor in each mode"),
            ("true-color", "Set to `true` to override automatic detection of terminal truecolor support in the event of a false negative. Defaults to `false`."),
            ("insert-final-newline", "Whether to automatically insert a trailing line-ending on write if missing. Defaults to `true`"),
            ("color-modes", "Whether to color modes with different colors. Defaults to `false`."),
            ("gutters", "Gutter configuration"),
            ("statusline", "Configuration of the statusline elements"),
            ("undercurl", "Set to `true` to override automatic detection of terminal undercurl support in the event of a false negative. Defaults to `false`."),
            ("search", "Search configuration"),
            ("lsp", "Lsp config"),
            ("terminal", "Terminal config"),
            ("rulers", "Column numbers at which to draw the rulers. Defaults to `[]`, meaning no rulers"),
            ("bufferline", "Persistently display open buffers along the top"),
            ("workspace-lsp-roots", "Workspace specific lsp ceiling dirs"),
            ("default-line-ending", "Which line ending to choose for new documents. Defaults to `native`. i.e. `crlf` on Windows, otherwise `lf`."),
            ("smart-tab", "Enables smart tab"),
            ("rainbow-brackets", "Enabled rainbow brackets"),
            ("keybindings", "Keybindings config"),
            ("set-keybindings!", "Override the global keybindings with the provided keymap"),
            ("inline-diagnostics-cursor-line-enable", "Inline diagnostics cursor line"),
            ("inline-diagnostics-other-lines-enable", "Inline diagnostics other lines"),
            ("inline-diagnostics-end-of-line-enable", "Inline diagnostics end of line"),
            ("inline-diagnostics-min-diagnostics-width", "Inline diagnostics min diagnostics width"),
            ("inline-diagnostics-prefix-len", "Inline diagnostics prefix length"),
            ("inline-diagnostics-max-wrap", "Inline diagnostics maximum wrap"),
            ("inline-diagnostics-max-diagnostics", "Inline diagnostics max diagnostics"),
            // language configuration functions
            ("get-language-config", "Get the configuration for a specific language"),
            // ("get-language-config-by-filename", "Get the language configuration for a specific file"),
            ("set-language-config!", "Set the language configuration"),
        ];

        for (func, doc) in functions {
            template_function_arity_1(func, doc);
        }

        if let Some(mut target_directory) = alternative_runtime_search_path() {
            if !target_directory.exists() {
                std::fs::create_dir_all(&target_directory).unwrap();
            }

            target_directory.push("configuration.scm");

            std::fs::write(target_directory, &builtin_configuration_module).unwrap();
        }

        engine.register_steel_module(
            "helix/configuration.scm".to_string(),
            builtin_configuration_module,
        );
    }

    if generate_sources {
        configure_lsp_builtins("configuration", &module);
    }

    engine.register_module(module);
}

fn _languages_api(_engine: &mut Engine, _generate_sources: bool) {
    // TODO: Just look at the `cx.editor.syn_loader` for how to
    // manipulate the languages bindings
    todo!()
}

// TODO:
// This isn't the best API since it pretty much requires deserializing
// the whole theme model each time. While its not _horrible_, it is
// certainly not as efficient as it could be. If we could just edit
// the loaded theme in memory already, then it would be a bit nicer.
fn load_theme_api(engine: &mut Engine, generate_sources: bool) {
    let mut module = BuiltInModule::new("helix/core/themes");
    module
        .register_fn("hashmap->theme", theme_from_json_string)
        .register_fn("add-theme!", add_theme)
        .register_fn("theme-style", get_style)
        .register_fn("theme-set-style!", set_style)
        .register_fn("string->color", string_to_color);

    if generate_sources {
        configure_lsp_builtins("themes", &module);
    }

    engine.register_module(module);
}

fn load_high_level_keymap_api(engine: &mut Engine, generate_sources: bool) {
    let keymap = include_str!("keymaps.scm");

    if generate_sources {
        if let Some(mut target_directory) = alternative_runtime_search_path() {
            if !target_directory.exists() {
                std::fs::create_dir_all(&target_directory).unwrap();
            }

            target_directory.push("keymaps.scm");

            std::fs::write(target_directory, keymap).unwrap();
        }
    }

    engine.register_steel_module("helix/keymaps.scm".to_string(), keymap.to_string());
}

fn load_high_level_theme_api(engine: &mut Engine, generate_sources: bool) {
    let theme = include_str!("themes.scm");

    if generate_sources {
        if let Some(mut target_directory) = alternative_runtime_search_path() {
            if !target_directory.exists() {
                std::fs::create_dir_all(&target_directory).unwrap();
            }

            target_directory.push("themes.scm");

            std::fs::write(target_directory, theme).unwrap();
        }
    }

    engine.register_steel_module("helix/themes.scm".to_string(), theme.to_string());
}

#[derive(Clone)]
struct SteelTheme(Theme);
impl Custom for SteelTheme {}

fn theme_from_json_string(name: String, value: SteelVal) -> Result<SteelTheme, anyhow::Error> {
    // TODO: Really don't love this at all. The deserialization should be a bit more elegant
    let json_value = serde_json::Value::try_from(value)?;
    let value: toml::Value = serde_json::from_str(&serde_json::to_string(&json_value)?)?;

    let (mut theme, _) = Theme::from_toml(value);
    theme.set_name(name);
    Ok(SteelTheme(theme))
}

// Mutate the theme?
fn add_theme(cx: &mut Context, theme: SteelTheme) {
    Arc::make_mut(&mut cx.editor.theme_loader)
        .add_dynamic_theme(theme.0.name().to_owned(), theme.0);
}

fn get_style(theme: &SteelTheme, name: SteelString) -> helix_view::theme::Style {
    theme.0.get(name.as_str())
}

fn set_style(theme: &mut SteelTheme, name: String, style: helix_view::theme::Style) {
    theme.0.set(name, style)
}

fn string_to_color(string: SteelString) -> Result<Color, anyhow::Error> {
    // TODO: Don't expose this directly
    helix_view::theme::ThemePalette::string_to_rgb(string.as_str()).map_err(anyhow::Error::msg)
}

fn current_buffer_area(cx: &mut Context) -> Option<helix_view::graphics::Rect> {
    let focus = cx.editor.tree.focus;
    cx.editor.tree.view_id_area(focus)
}

fn load_editor_api(engine: &mut Engine, generate_sources: bool) {
    let mut module = BuiltInModule::new("helix/core/editor");

    let mut builtin_editor_command_module =
        "(require-builtin helix/core/editor as helix.)".to_string();

    let mut template_function_arity_0 = |name: &str, doc: &str| {
        let doc = format_docstring(doc);
        builtin_editor_command_module.push_str(&format!(
            r#"
(provide {})
;;@doc
{}
(define ({})
    (helix.{} *helix.cx*))
"#,
            name, doc, name, name
        ));
    };

    macro_rules! register_0 {
        ($name:expr, $func:expr, $doc:expr) => {
            module.register_fn($name, $func);
            template_function_arity_0($name, $doc);
        };
    }

    // Types
    module.register_fn("Action/Load", || Action::Load);
    module.register_fn("Action/Replace", || Action::Replace);
    module.register_fn("Action/HorizontalSplit", || Action::HorizontalSplit);
    module.register_fn("Action/VerticalSplit", || Action::VerticalSplit);

    // Arity 0
    register_0!(
        "editor-focus",
        cx_current_focus,
        r#"
Get the current focus of the editor, as a `ViewId`.

```scheme
(editor-focus) -> ViewId
```
        "#
    );
    register_0!(
        "editor-mode",
        cx_get_mode,
        r#"
Get the current mode of the editor

```scheme
(editor-mode) -> Mode?
```
        "#
    );

    register_0!(
        "cx->themes",
        get_themes,
        "DEPRECATED: Please use `themes->list`"
    );

    register_0!(
        "themes->list",
        get_themes,
        r#"
Get the current themes as a list of strings.

```scheme
(themes->list) -> (listof string?)
```
        "#
    );

    register_0!(
        "editor-all-documents",
        cx_editor_all_documents,
        r#"
Get a list of all of the document ids that are currently open.

```scheme
(editor-all-documents) -> (listof DocumentId?)
```
        "#
    );
    register_0!(
        "cx->cursor",
        |cx: &mut Context| cx.editor.cursor(),
        r#"DEPRECATED: Please use `current-cursor`"#
    );

    register_0!(
        "current-cursor",
        |cx: &mut Context| cx.editor.cursor(),
        r#"Gets the primary cursor position in screen coordinates,
or `#false` if the primary cursor is not visible on screen.

```scheme
(current-cursor) -> (listof? (or Position? #false) CursorKind)
```
        "#
    );

    register_0!(
        "editor-focused-buffer-area",
        current_buffer_area,
        r#"
Get the `Rect` associated with the currently focused buffer.

```scheme
(editor-focused-buffer-area) -> (or Rect? #false)
```
        "#
    );
    register_0!(
        "selected-register!",
        |cx: &mut Context| cx
            .editor
            .selected_register
            .unwrap_or(cx.editor.config().default_yank_register),
        r#"Get currently selected register"#
    );

    // Arity 1
    module.register_fn("editor->doc-id", cx_get_document_id);
    module.register_fn("editor-switch!", cx_switch);
    module.register_fn("editor-set-focus!", |cx: &mut Context, view_id: ViewId| {
        cx.editor.focus(view_id)
    });
    module.register_fn("editor-set-mode!", cx_set_mode);
    module.register_fn("editor-doc-in-view?", cx_is_document_in_view);
    module.register_fn("set-scratch-buffer-name!", set_scratch_buffer_name);

    // Get the last saved time of the document
    module.register_fn(
        "editor-document-last-saved",
        |cx: &mut Context, doc: DocumentId| -> Option<SystemTime> {
            cx.editor.documents.get(&doc).map(|x| x.last_saved_time())
        },
    );

    module.register_fn("editor-document->language", cx_get_document_language);

    module.register_fn(
        "editor-document-dirty?",
        |cx: &mut Context, doc: DocumentId| -> Option<bool> {
            cx.editor.documents.get(&doc).map(|x| x.is_modified())
        },
    );

    module.register_fn(
        "editor-document-reload",
        |cx: &mut Context, doc: DocumentId| -> anyhow::Result<()> {
            for (view, _) in cx.editor.tree.views_mut() {
                if let Some(x) = cx.editor.documents.get_mut(&doc) {
                    x.reload(view, &cx.editor.diff_providers)?;
                }
            }
            Ok(())
        },
    );

    module.register_fn("set-buffer-uri!", set_buffer_uri);

    module.register_fn("editor-doc-exists?", cx_document_exists);

    // Arity 2
    module.register_fn("editor-switch-action!", cx_switch_action);
    module.register_fn(
        "set-register!",
        |cx: &mut Context, name: char, value: Vec<String>| cx.editor.registers.write(name, value),
    );

    // Arity 1
    module.register_fn("editor->text", document_id_to_text);
    module.register_fn("editor-document->path", document_path);
    module.register_fn("register->value", cx_register_value);

    module.register_fn("set-editor-clip-right!", |cx: &mut Context, right: u16| {
        cx.editor.editor_clipping.right = Some(right);
    });
    module.register_fn("set-editor-clip-left!", |cx: &mut Context, left: u16| {
        cx.editor.editor_clipping.left = Some(left);
    });
    module.register_fn("set-editor-clip-top!", |cx: &mut Context, top: u16| {
        cx.editor.editor_clipping.top = Some(top);
    });
    module.register_fn(
        "set-editor-clip-bottom!",
        |cx: &mut Context, bottom: u16| {
            cx.editor.editor_clipping.bottom = Some(bottom);
        },
    );

    if generate_sources {
        let mut template_function_type_constructor = |name: &str| {
            builtin_editor_command_module.push_str(&format!(
                r#"
(provide {})
(define {} helix.{})
"#,
                name, name, name
            ));
        };

        template_function_type_constructor("Action/Load");
        template_function_type_constructor("Action/Replace");
        template_function_type_constructor("Action/HorizontalSplit");
        template_function_type_constructor("Action/VerticalSplit");

        let mut template_function_arity_1 = |name: &str, doc: &str| {
            if generate_sources {
                let docstring = format_docstring(doc);
                builtin_editor_command_module.push_str(&format!(
                    r#"
(provide {})
;;@doc
{}
(define ({} arg)
    (helix.{} *helix.cx* arg))
"#,
                    name, docstring, name, name
                ));
            }
        };

        template_function_arity_1("editor->doc-id", "Get the document from a given view.");
        template_function_arity_1("editor-switch!", "Open the document in a vertical split.");
        template_function_arity_1("editor-set-focus!", "Set focus on the view.");
        template_function_arity_1("editor-set-mode!", "Set the editor mode.");
        template_function_arity_1(
            "editor-doc-in-view?",
            "Check whether the current view contains a document.",
        );
        template_function_arity_1(
            "set-scratch-buffer-name!",
            "Set the name of a scratch buffer.",
        );

        // TODO: Lift this up
        template_function_arity_1("set-buffer-uri!", "Set the URI of the buffer");
        template_function_arity_1("editor-doc-exists?", "Check if a document exists.");

        template_function_arity_1(
            "editor-document-last-saved",
            "Check when a document was last saved (returns a `SystemTime`)",
        );

        template_function_arity_1(
            "editor-document->language",
            "Get the language for the document",
        );

        template_function_arity_1(
            "editor-document-dirty?",
            "Check if a document has unsaved changes",
        );

        template_function_arity_1("editor-document-reload", "Reload a document.");

        template_function_arity_1("editor->text", "Get the document as a rope.");
        template_function_arity_1("editor-document->path", "Get the path to a document.");
        template_function_arity_1(
            "register->value",
            "Get register value as a list of strings.",
        );
        template_function_arity_1(
            "set-editor-clip-top!",
            "Set the editor clipping at the top.",
        );
        template_function_arity_1(
            "set-editor-clip-right!",
            "Set the editor clipping at the right.",
        );
        template_function_arity_1(
            "set-editor-clip-left!",
            "Set the editor clipping at the left.",
        );
        template_function_arity_1(
            "set-editor-clip-bottom!",
            "Set the editor clipping at the bottom.",
        );

        let mut template_function_arity_2 = |name: &str| {
            builtin_editor_command_module.push_str(&format!(
                r#"
(provide {})
(define ({} arg1 arg2)
    (helix.{} *helix.cx* arg1 arg2))
"#,
                name, name, name
            ));
        };

        template_function_arity_2("editor-switch-action!");
        template_function_arity_2("set-register!");

        if let Some(mut target_directory) = alternative_runtime_search_path() {
            if !target_directory.exists() {
                std::fs::create_dir_all(&target_directory).unwrap_or_else(|err| {
                    panic!("Failed to create directory {:?}: {}", target_directory, err)
                });
            }

            target_directory.push("editor.scm");

            std::fs::write(target_directory, &builtin_editor_command_module).unwrap();
        }

        engine.register_steel_module(
            "helix/editor.scm".to_string(),
            builtin_editor_command_module,
        );
    }

    // Generate the lsp configuration
    if generate_sources {
        configure_lsp_builtins("editor", &module);
    }

    engine.register_module(module);
}

pub struct SteelScriptingEngine;

impl super::PluginSystem for SteelScriptingEngine {
    fn initialize(&self) {
        std::thread::spawn(initialize_engine);
    }

    fn reinitialize(&self) {
        reload_engine();
    }

    fn engine_name(&self) -> super::PluginSystemKind {
        super::PluginSystemKind::Steel
    }

    fn run_initialization_script(
        &self,
        cx: &mut Context,
        configuration: Arc<ArcSwapAny<Arc<Config>>>,
        language_configuration: Arc<ArcSwap<syntax::Loader>>,
        event_reader: TerminalEventReaderHandle,
    ) {
        run_initialization_script(cx, configuration, language_configuration, event_reader);
    }

    fn handle_keymap_event(
        &self,
        editor: &mut ui::EditorView,
        mode: Mode,
        cxt: &mut Context,
        event: KeyEvent,
    ) -> Option<KeymapResult> {
        SteelScriptingEngine::handle_keymap_event_impl(self, editor, mode, cxt, event)
    }

    fn call_function_by_name(&self, cx: &mut Context, name: &str, args: &[Cow<str>]) -> bool {
        if enter_engine(|x| x.global_exists(name)) {
            let mut args = args
                .iter()
                .map(|x| x.clone().into_steelval().unwrap())
                .collect::<Vec<_>>();

            if let Err(e) = enter_engine(|guard| {
                {
                    // Install the interrupt handler, in the event this thing
                    // is blocking for too long.
                    with_interrupt_handler(|| {
                        guard.with_mut_reference::<Context, Context>(cx).consume(
                            move |engine, arguments| {
                                let context = arguments[0].clone();
                                engine.update_value("*helix.cx*", context);
                                engine
                                    .call_function_by_name_with_args_from_mut_slice(name, &mut args)
                            },
                        )
                    })
                }
            }) {
                cx.editor.set_error(format!("{}", e));
            }
            true
        } else {
            false
        }
    }

    fn call_typed_command<'a>(
        &self,
        cx: &mut compositor::Context,
        command: &'a str,
        parts: &'a [&'a str],
        event: PromptEvent,
    ) -> bool {
        if enter_engine(|x| x.global_exists(command)) {
            let args = parts;

            // We're finalizing the event - we actually want to call the function
            if event == PromptEvent::Validate {
                if let Err(e) = enter_engine(|guard| {
                    let args = args
                        .iter()
                        .map(|x| x.into_steelval().unwrap())
                        .collect::<Vec<_>>();

                    let res = {
                        let mut ctx = Context {
                            register: None,
                            count: std::num::NonZeroUsize::new(1),
                            editor: cx.editor,
                            callback: Vec::new(),
                            on_next_key_callback: None,
                            jobs: cx.jobs,
                        };

                        // Install interrupt handler here during the duration
                        // of the function call
                        match with_interrupt_handler(|| {
                            guard
                                .with_mut_reference(&mut ctx)
                                .consume(move |engine, arguments| {
                                    let context = arguments[0].clone();
                                    engine.update_value("*helix.cx*", context);
                                    // TODO: Fix this clone
                                    engine.call_function_by_name_with_args(command, args.clone())
                                })
                        }) {
                            Ok(res) => {
                                match &res {
                                    SteelVal::Void => {}
                                    SteelVal::StringV(s) => {
                                        cx.editor.set_status(s.as_str().to_owned());
                                    }
                                    _ => {
                                        cx.editor.set_status(res.to_string());
                                    }
                                }

                                Ok(res)
                            }
                            Err(e) => Err(e),
                        }
                    };

                    res
                }) {
                    let mut ctx = Context {
                        register: None,
                        count: None,
                        editor: cx.editor,
                        callback: Vec::new(),
                        on_next_key_callback: None,
                        jobs: cx.jobs,
                    };

                    enter_engine(|x| present_error_inside_engine_context(&mut ctx, x, e));
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
        try_enter_engine(|engine| get_doc_for_global(engine, ident)).unwrap_or_default()
    }

    // Just dump docs for all top level values?
    fn available_commands<'a>(&self) -> Vec<Cow<'a, str>> {
        try_enter_engine(|engine| {
            engine
                .readable_globals(GLOBAL_OFFSET.load(std::sync::atomic::Ordering::Relaxed))
                .iter()
                .map(|x| x.resolve().to_string().into())
                .collect()
        })
        .unwrap_or_default()
    }

    fn generate_sources(&self) {
        fn format_markdown_doc<W: Write>(writer: &mut W, doc: &str) {
            for line in doc.lines() {
                if line.starts_with("# ") {
                    write!(writer, "###").unwrap();
                }
                writeln!(writer, "{}", line).unwrap();
            }
        }

        // Generate sources directly with a fresh engine
        let mut engine = Engine::new();
        configure_builtin_sources(&mut engine, true);
        // Generate documentation as well
        if let Some(target) = alternative_runtime_search_path() {
            let mut writer =
                std::io::BufWriter::new(std::fs::File::create("steel-docs.md").unwrap());

            // Generate markdown docs
            steel_doc::walk_dir(&mut writer, target, &mut engine).unwrap();

            // Also generate docs for the built in modules
            let module = engine.builtin_modules().get("helix/core/text").unwrap();

            writeln!(&mut writer, "# helix/core/text").unwrap();
            writeln!(
                &mut writer,
                "To use, you can include with `(require-builtin helix/core/text)`"
            )
            .unwrap();

            let mut found_definitions = std::collections::HashSet::new();

            let mut exported_functions: Vec<_> = module
                .names()
                .into_iter()
                .filter(|name| !name.starts_with("#%"))
                .collect();

            exported_functions.sort();

            for name in &exported_functions {
                if let Some(value) = module.documentation().get(name) {
                    found_definitions.insert(name.to_string());

                    if let steel::steel_vm::builtin::Documentation::Markdown(m) = value {
                        let escaped = name.replace("*", "\\*");
                        writeln!(&mut writer, "### **{}**", escaped).unwrap();

                        format_markdown_doc(&mut writer, &m.0);
                    }
                }
            }

            for name in exported_functions {
                if !found_definitions.contains(&name) {
                    writeln!(&mut writer, "### **{}**", name).unwrap();
                }
            }
        }
    }

    // TODO: Should this just be a hook / event instead of a function like this?
    // Handle an LSP notification, assuming its been sent through
    fn handle_lsp_call(
        &self,
        cx: &mut compositor::Context,
        server_id: helix_lsp::LanguageServerId,
        event_name: String,
        call_id: jsonrpc::Id,
        params: helix_lsp::jsonrpc::Params,
    ) -> Option<Result<serde_json::Value, jsonrpc::Error>> {
        let mut ctx = Context {
            register: None,
            count: None,
            editor: cx.editor,
            callback: Vec::new(),
            on_next_key_callback: None,
            jobs: cx.jobs,
        };

        let language_server_name = ctx
            .editor
            .language_servers
            .get_by_id(server_id)
            .map(|x| x.name().to_owned());

        let Some(language_server_name) = language_server_name else {
            ctx.editor.set_error("Unable to find language server");
            return None;
        };

        let mut pass_call_id = false;

        let id = LspCallRegistryId {
            lsp_name: language_server_name,
            event_name,
            generation: load_generation(),
        };

        let function = LSP_CALL_REGISTRY
            .read()
            .unwrap()
            .map
            .get(&id)
            .map(|x| match x {
                LspKind::Call(rooted_steel_val) => {
                    pass_call_id = true;
                    rooted_steel_val.value()
                }
                LspKind::Notification(rooted_steel_val) => rooted_steel_val.value(),
            })
            .cloned();

        let result = if let Some(function) = function {
            enter_engine(|guard| {
                // Install the interrupt handler, in the event this thing
                // is blocking for too long.
                with_interrupt_handler(|| {
                    guard
                        .with_mut_reference::<Context, Context>(&mut ctx)
                        .consume(move |engine, arguments| {
                            let context = arguments[0].clone();
                            engine.update_value("*helix.cx*", context);

                            let params = serde_json::to_value(&params)
                                .map_err(|e| SteelErr::new(ErrorKind::Generic, e.to_string()))
                                .and_then(|x| x.into_steelval())?;

                            if pass_call_id {
                                let call_id = serde_json::to_value(&call_id)
                                    .map_err(|e| SteelErr::new(ErrorKind::Generic, e.to_string()))
                                    .and_then(|x| x.into_steelval())?;

                                let mut arguments = [call_id, params];

                                engine.call_function_with_args_from_mut_slice(
                                    function.clone(),
                                    &mut arguments,
                                )
                            } else {
                                let mut arguments = [params];
                                engine.call_function_with_args_from_mut_slice(
                                    function.clone(),
                                    &mut arguments,
                                )
                            }
                        })
                })
            })
        } else {
            Ok(SteelVal::Void)
        };

        let value = match result {
            Err(e) => {
                cx.editor.set_error(format!("{}", e));
                Some(SteelVal::Void)
            }
            Ok(value) => Some(value),
        }?;

        match value {
            SteelVal::Void => None,
            value => {
                let serde_value: Result<serde_json::Value, ::steel::SteelErr> = value.try_into();
                match serde_value {
                    Ok(serialized_value) => Some(Ok(serialized_value)),
                    Err(error) => {
                        log::warn!("Failed to serialize a SteelVal: {}", error);
                        None
                    }
                }
            }
        }
    }
}

impl SteelScriptingEngine {
    fn handle_keymap_event_impl(
        &self,
        editor: &mut ui::EditorView,
        mode: Mode,
        cx: &mut Context,
        event: KeyEvent,
    ) -> Option<KeymapResult> {
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
            &view.doc
        };

        if let Some(extension) = extension {
            let map = get_extension_keymap();
            let keymap = map.get_extension(extension);

            if let Some(keymap) = keymap {
                return Some(editor.keymaps.get_with_map(&keymap.0, mode, event));
            }
        }

        let map = get_extension_keymap();

        if let Some(keymap) = map.get_doc_id(document_id_to_usize(doc_id)) {
            return Some(editor.keymaps.get_with_map(&keymap.0, mode, event));
        }

        None
    }
}

pub fn initialize_engine() {
    enter_engine(|x| x.globals().first().copied());
}

pub fn present_error_inside_engine_context(cx: &mut Context, engine: &mut Engine, e: SteelErr) {
    cx.editor.set_error(e.to_string());

    let backtrace = engine.raise_error_to_string(e);

    let callback = async move {
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                if let Some(backtrace) = backtrace {
                    let contents = ui::Markdown::new(
                        format!("```\n{}\n```", backtrace),
                        editor.syn_loader.clone(),
                    );
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

pub fn present_error_inside_engine_context_with_callback(
    cx: &mut Context,
    engine: &mut Engine,
    e: SteelErr,
    mut callback: impl FnMut(&mut Compositor) + Send + Sync + 'static,
) {
    cx.editor.set_error(e.to_string());

    let backtrace = engine.raise_error_to_string(e);

    let callback = async move {
        let call: job::Callback = Callback::EditorCompositor(Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor| {
                if let Some(backtrace) = backtrace {
                    let contents = ui::Markdown::new(
                        format!("```\n{}\n```", backtrace),
                        editor.syn_loader.clone(),
                    );
                    let popup = Popup::new("engine", contents).position(Some(
                        helix_core::Position::new(editor.cursor().0.unwrap_or_default().row, 2),
                    ));
                    compositor.replace_or_push("engine", popup);

                    callback(compositor);
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

pub fn update_documentation(map: &mut EmbeddedKeyMap, docs: HashMap<String, String>) {
    let mut func = move |command: &mut MappableCommand| {
        if let Some(steel_doc) = docs.get(command.name()) {
            if let Some(doc) = command.doc_mut() {
                *doc = steel_doc.to_owned()
            }
        }
    };

    for trie in map.0.values_mut() {
        trie.apply(&mut func)
    }
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

pub fn string_to_embedded_keymap(value: String) -> anyhow::Result<EmbeddedKeyMap> {
    Ok(EmbeddedKeyMap(serde_json::from_str(&value)?))
}

pub fn merge_keybindings(left: &mut EmbeddedKeyMap, right: EmbeddedKeyMap) {
    merge_keys(&mut left.0, right.0)
}

pub fn is_keymap(keymap: SteelVal) -> bool {
    if let SteelVal::Custom(underlying) = keymap {
        as_underlying_type::<EmbeddedKeyMap>(underlying.read().as_ref()).is_some()
    } else {
        false
    }
}

fn local_config_exists() -> bool {
    let local_helix = find_workspace().0.join(".helix");
    local_helix.join("helix.scm").exists() && local_helix.join("init.scm").exists()
}

fn preferred_config_path(file_name: &str) -> PathBuf {
    if let Ok(steel_config_dir) = std::env::var("HELIX_STEEL_CONFIG") {
        PathBuf::from(steel_config_dir).join(file_name)
    } else if local_config_exists() {
        find_workspace().0.join(".helix").join(file_name)
    } else {
        helix_loader::config_dir().join(file_name)
    }
}

pub fn helix_module_file() -> PathBuf {
    preferred_config_path("helix.scm")
}

pub fn steel_init_file() -> PathBuf {
    preferred_config_path("init.scm")
}

struct HelixConfiguration {
    configuration: Arc<ArcSwapAny<Arc<Config>>>,
    language_configuration: Arc<ArcSwap<helix_core::syntax::Loader>>,
}

#[derive(Clone)]
struct IndividualLanguageConfiguration {
    // Lets go ahead and just deserialize it that way.
    // It's ugly and annoying.
    config: LanguageConfiguration,
}

// TODO: @Matt 5/19/2025 - Finish up writing these bindings.
impl Custom for IndividualLanguageConfiguration {}

impl Custom for HelixConfiguration {}

fn update_configuration_for_all_open_documents(ctx: &mut Context) {
    for document in ctx.editor.documents.values_mut() {
        if let Some(name) = document.language_name() {
            let config_for_file = ctx
                .editor
                .syn_loader
                .load()
                .language_configs()
                .find(|x| x.language_id == name)
                .cloned()
                .map(Arc::new);
            document.language = config_for_file;
        }
    }
}

fn set_configuration_for_file(
    ctx: &mut Context,
    file_name: SteelString,
    configuration: IndividualLanguageConfiguration,
) {
    if let Some(document) = ctx.editor.document_by_path_mut(file_name.as_str()) {
        document.language = Some(Arc::new(configuration.config));
    }
}

impl HelixConfiguration {
    fn _store_language_configuration(&self, language_config: syntax::Loader) {
        self.language_configuration.store(Arc::new(language_config))
    }

    fn get_language_config(&self, language: SteelString) -> Option<SteelVal> {
        self.language_configuration
            .load()
            .language_configs()
            .find(|x| x.language_id == language.as_str())
            .and_then(|x| {
                let config = serde_json::json!(x);

                SteelVal::try_from(config).ok()
            })
    }

    fn update_language_config(
        &mut self,
        language: SteelString,
        config: SteelVal,
    ) -> anyhow::Result<()> {
        // Do some gross json -> toml conversion
        let mut value = serde_json::Value::try_from(config)?;

        fn filter_null_values(value: &mut Value) {
            match value {
                Value::Object(map) => {
                    map.retain(|_, v| {
                        if v.is_null() {
                            false
                        } else {
                            filter_null_values(v);
                            true
                        }
                    });
                }
                Value::Array(arr) => {
                    arr.retain_mut(|v| {
                        if v.is_null() {
                            false
                        } else {
                            filter_null_values(v);
                            true
                        }
                    });
                }
                Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        *n = (f.round() as i64).into();
                    }
                }
                _ => {}
            }
        }

        filter_null_values(&mut value);

        // Horrendous, disgusting
        let mut toml_value: toml::Value = serde_json::from_str(&serde_json::to_string(&value)?)?;

        let auto_format_present = toml_value.get("auto-format").is_some();
        let diagnostic_severity_present = toml_value.get("diagnostic-severity").is_some();
        let language_servers_present = toml_value.get("language-servers").is_some();
        let persistent_diagnostic_sources_present =
            toml_value.get("persistent-diagnostic-sources").is_some();

        // Existing language config:
        let mut existing_config = self
            .language_configuration
            .load()
            .language_configs()
            .find(|x| x.language_id == language.as_str())
            .unwrap()
            .clone();

        if toml_value.get("scope").is_none() {
            toml_value
                .as_table_mut()
                .and_then(|x| x.insert("scope".to_string(), existing_config.scope.into()));
        }

        for need_empty in ["file-types", "shebangs", "roots"] {
            if toml_value.get(need_empty).is_none() {
                toml_value.as_table_mut().and_then(|x| {
                    x.insert(need_empty.to_owned(), <Vec<toml::Value>>::new().into())
                });
            }
        }

        let new_config: LanguageConfiguration = toml_value.try_into()?;

        if let Some(id) = new_config.language_server_language_id {
            existing_config.language_server_language_id = Some(id);
        }

        // Take the new scope, since its already set to the old one as a default.
        existing_config.scope = new_config.scope;

        if !new_config.file_types.is_empty() {
            existing_config.file_types = new_config.file_types;
        }

        if !new_config.shebangs.is_empty() {
            existing_config.shebangs = new_config.shebangs;
        }

        if !new_config.roots.is_empty() {
            existing_config.roots = new_config.roots;
        }

        if let Some(comment_tokens) = new_config.comment_tokens {
            existing_config.comment_tokens = Some(comment_tokens);
        }

        if let Some(block_comment_tokens) = new_config.block_comment_tokens {
            existing_config.block_comment_tokens = Some(block_comment_tokens);
        }

        if let Some(text_width) = new_config.text_width {
            existing_config.text_width = Some(text_width);
        }

        if let Some(soft_wrap) = new_config.soft_wrap {
            existing_config.soft_wrap = Some(soft_wrap);
        }

        if auto_format_present {
            existing_config.auto_format = new_config.auto_format;
        }

        if let Some(formatter) = new_config.formatter {
            existing_config.formatter = Some(formatter);
        }

        if let Some(path_complation) = new_config.path_completion {
            existing_config.path_completion = Some(path_complation);
        }

        if diagnostic_severity_present {
            existing_config.diagnostic_severity = new_config.diagnostic_severity;
        }

        if let Some(grammar) = new_config.grammar {
            existing_config.grammar = Some(grammar);
        }

        if let Some(injection_regex) = new_config.injection_regex {
            existing_config.injection_regex = Some(injection_regex);
        }

        if language_servers_present {
            existing_config.language_servers = new_config.language_servers;
        }

        if let Some(indent) = new_config.indent {
            existing_config.indent = Some(indent);
        }

        if let Some(debugger) = new_config.debugger {
            existing_config.debugger = Some(debugger);
        }

        if let Some(auto_pairs) = new_config.auto_pairs {
            existing_config.auto_pairs = Some(auto_pairs);
        }

        if let Some(rulers) = new_config.rulers {
            existing_config.rulers = Some(rulers);
        }

        if let Some(workspace_lsp_roots) = new_config.workspace_lsp_roots {
            existing_config.workspace_lsp_roots = Some(workspace_lsp_roots);
        }

        if let Some(rainbow) = new_config.rainbow_brackets {
            existing_config.rainbow_brackets = Some(rainbow);
        }

        if persistent_diagnostic_sources_present {
            existing_config.persistent_diagnostic_sources =
                new_config.persistent_diagnostic_sources;
        }

        self.update_individual_language_config(IndividualLanguageConfiguration {
            config: existing_config,
        });

        Ok(())
    }

    fn update_language_server_config(
        &mut self,
        lsp: SteelString,
        map: HashMap<String, SteelVal>,
    ) -> anyhow::Result<()> {
        let mut loader = (*(*self.language_configuration.load())).clone();
        let lsp_configs = loader.language_server_configs_mut();

        let individual_config = lsp_configs.get_mut(lsp.as_str());

        if let Some(config) = individual_config {
            if let Some(args) = map.get("args") {
                config.args = <Vec<String>>::from_steelval(args)?;
            }

            if let Some(command) = map.get("command") {
                config.command = String::from_steelval(command)?;
            }

            if let Some(environment) = map.get("environment") {
                config.environment = <HashMap<String, String>>::from_steelval(environment)?;
            }

            if let Some(config_json) = map.get("config") {
                let serialized = serde_json::Value::try_from(config_json.clone())?;
                config.config = Some(serialized);
            }

            if let Some(timeout) = map.get("timeout") {
                config.timeout = u64::from_steelval(timeout)?;
            }

            if let Some(required_root_patterns) = map.get("required-root-patterns") {
                let patterns = <Vec<String>>::from_steelval(required_root_patterns)?;

                if !patterns.is_empty() {
                    let mut builder = globset::GlobSetBuilder::new();
                    for pattern in patterns {
                        let glob = globset::Glob::new(&pattern)?;
                        builder.add(glob);
                    }
                    config.required_root_patterns = Some(builder.build()?);
                }
            }
        } else {
            let command = if let Some(command) = map.get("command") {
                String::from_steelval(command)?
            } else {
                anyhow::bail!("LSP config missing required `command` field.");
            };

            let mut config = LanguageServerConfiguration {
                command,
                args: Vec::new(),
                environment: HashMap::new(),
                config: None,
                timeout: default_timeout(),
                required_root_patterns: None,
            };

            if let Some(args) = map.get("args") {
                config.args = <Vec<String>>::from_steelval(args)?;
            }

            if let Some(environment) = map.get("environment") {
                config.environment = <HashMap<String, String>>::from_steelval(environment)?;
            }

            if let Some(config_json) = map.get("config") {
                let serialized = serde_json::Value::try_from(config_json.clone())?;
                config.config = Some(serialized);
            }

            if let Some(timeout) = map.get("timeout") {
                config.timeout = u64::from_steelval(timeout)?;
            }

            if let Some(required_root_patterns) = map.get("required-root-patterns") {
                let patterns = <Vec<String>>::from_steelval(required_root_patterns)?;

                if !patterns.is_empty() {
                    let mut builder = globset::GlobSetBuilder::new();
                    for pattern in patterns {
                        let glob = globset::Glob::new(&pattern)?;
                        builder.add(glob);
                    }
                    config.required_root_patterns = Some(builder.build()?);
                }
            }

            lsp_configs.insert(lsp.as_str().to_owned(), config);
        }

        self.language_configuration.store(Arc::new(loader));

        Ok(())
    }

    // Update the language config - this does not immediately flush it
    // to the actual config.
    fn update_individual_language_config(&mut self, config: IndividualLanguageConfiguration) {
        // TODO: Try to opportunistically load the ref counts
        // of the inner values - if the documents haven't been opened yet, we
        // don't need to clone the _whole_ loader.
        let mut loader = (*(*self.language_configuration.load())).clone();
        let config = config.config;

        for lconfig in loader.language_configs_mut() {
            if lconfig.language_id == config.language_id {
                if let Some(inner) = Arc::get_mut(lconfig) {
                    *inner = config;
                } else {
                    *lconfig = Arc::new(config);
                }
                break;
            }
        }

        self.language_configuration.store(Arc::new(loader));
    }

    fn load_config(&self) -> Config {
        (*self.configuration.load().clone()).clone()
    }

    fn store_config(&self, config: Config) {
        self.configuration.store(Arc::new(config));
    }

    // Overlay new keybindings
    fn keybindings(&self, keybindings: EmbeddedKeyMap) {
        let mut app_config = self.load_config();
        merge_keys(&mut app_config.keys, keybindings.0);
        self.store_config(app_config);
    }

    fn set_keybindings(&self, keybindings: EmbeddedKeyMap) {
        let mut app_config = self.load_config();
        app_config.keys = keybindings.0;
        self.store_config(app_config);
    }

    fn get_keybindings(&self) -> EmbeddedKeyMap {
        EmbeddedKeyMap(self.load_config().keys.clone())
    }

    fn scrolloff(&self, lines: usize) {
        let mut app_config = self.load_config();
        app_config.editor.scrolloff = lines;
        self.store_config(app_config);
    }

    fn scroll_lines(&self, lines: isize) {
        let mut app_config = self.load_config();
        app_config.editor.scroll_lines = lines;
        self.store_config(app_config);
    }

    fn mouse(&self, m: bool) {
        let mut app_config = self.load_config();
        app_config.editor.mouse = m;
        self.store_config(app_config);
    }

    fn shell(&self, shell: Vec<String>) {
        let mut app_config = self.load_config();
        app_config.editor.shell = shell;
        self.store_config(app_config);
    }

    fn jump_label_alphabet(&self, alphabet: String) {
        let mut app_config = self.load_config();
        app_config.editor.jump_label_alphabet = alphabet.chars().collect();
        self.store_config(app_config);
    }

    fn line_number(&self, mode_config: SteelVal) -> anyhow::Result<()> {
        let config = match mode_config {
            SteelVal::StringV(s) | SteelVal::SymbolV(s) => match s.as_str() {
                "relative" => LineNumber::Relative,
                "absolute" => LineNumber::Absolute,
                other => anyhow::bail!("Unrecognized line-number option: {}", other),
            },
            other => anyhow::bail!("Unrecognized line-number option: {}", other),
        };

        let mut app_config = self.load_config();
        app_config.editor.line_number = config;
        self.store_config(app_config);
        Ok(())
    }

    fn cursorline(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.cursorline = option;
        self.store_config(app_config);
    }

    fn cursorcolumn(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.cursorcolumn = option;
        self.store_config(app_config);
    }

    fn middle_click_paste(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.middle_click_paste = option;
        self.store_config(app_config);
    }

    fn auto_pairs(&self, config: AutoPairConfig) {
        let mut app_config = self.load_config();
        app_config.editor.auto_pairs = config;
        self.store_config(app_config);
    }

    fn auto_completion(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.auto_completion = option;
        self.store_config(app_config);
    }

    fn auto_format(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.auto_format = option;
        self.store_config(app_config);
    }

    fn auto_save(&self, option: AutoSave) {
        let mut app_config = self.load_config();
        app_config.editor.auto_save = option;
        self.store_config(app_config);
    }

    // TODO: Finish the auto save options!
    fn auto_save_after_delay_enable(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.auto_save.after_delay.enable = option;
        self.store_config(app_config);
    }

    fn inline_diagnostics_cursor_line_enable(&self, severity: String) {
        let mut app_config = self.load_config();
        let severity = match severity.as_str() {
            "hint" => Severity::Hint,
            "info" => Severity::Info,
            "warning" => Severity::Warning,
            "error" => Severity::Error,
            _ => return,
        };
        app_config.editor.inline_diagnostics.cursor_line = DiagnosticFilter::Enable(severity);
        self.store_config(app_config);
    }

    fn inline_diagnostics_other_lines_enable(&self, severity: String) {
        let mut app_config = self.load_config();
        let severity = match severity.as_str() {
            "hint" => Severity::Hint,
            "info" => Severity::Info,
            "warning" => Severity::Warning,
            "error" => Severity::Error,
            _ => return,
        };
        app_config.editor.inline_diagnostics.other_lines = DiagnosticFilter::Enable(severity);
        self.store_config(app_config);
    }

    fn inline_diagnostics_min_diagnostic_width(&self, min_diagnostic_width: u16) {
        let mut app_config = self.load_config();
        app_config.editor.inline_diagnostics.min_diagnostic_width = min_diagnostic_width;
        self.store_config(app_config);
    }

    fn inline_diagnostics_prefix_len(&self, prefix_len: u16) {
        let mut app_config = self.load_config();
        app_config.editor.inline_diagnostics.prefix_len = prefix_len;
        self.store_config(app_config);
    }

    fn inline_diagnostics_max_wrap(&self, max_wrap: u16) {
        let mut app_config = self.load_config();
        app_config.editor.inline_diagnostics.max_wrap = max_wrap;
        self.store_config(app_config);
    }

    fn inline_diagnostics_max_diagnostics(&self, max_diagnostics: usize) {
        let mut app_config = self.load_config();
        app_config.editor.inline_diagnostics.max_diagnostics = max_diagnostics;
        self.store_config(app_config);
    }

    fn inline_diagnostics_end_of_line_enable(&self, severity: String) {
        let mut app_config = self.load_config();
        let severity = match severity.as_str() {
            "hint" => Severity::Hint,
            "info" => Severity::Info,
            "warning" => Severity::Warning,
            "error" => Severity::Error,
            _ => return,
        };
        app_config.editor.end_of_line_diagnostics = DiagnosticFilter::Enable(severity);
        self.store_config(app_config);
    }

    fn text_width(&self, width: usize) {
        let mut app_config = self.load_config();
        app_config.editor.text_width = width;
        self.store_config(app_config);
    }

    fn idle_timeout(&self, ms: usize) {
        let mut app_config = self.load_config();
        app_config.editor.idle_timeout = Duration::from_millis(ms as u64);
        self.store_config(app_config);
    }

    fn completion_timeout(&self, ms: usize) {
        let mut app_config = self.load_config();
        app_config.editor.completion_timeout = Duration::from_millis(ms as u64);
        self.store_config(app_config);
    }

    fn preview_completion_insert(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.preview_completion_insert = option;
        self.store_config(app_config);
    }

    // TODO: Make sure this conversion works automatically
    fn completion_trigger_len(&self, length: u8) {
        let mut app_config = self.load_config();
        app_config.editor.completion_trigger_len = length;
        self.store_config(app_config);
    }

    fn completion_replace(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.completion_replace = option;
        self.store_config(app_config);
    }

    fn auto_info(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.auto_info = option;
        self.store_config(app_config);
    }

    fn cursor_shape(&self, config: CursorShapeConfig) {
        let mut app_config = self.load_config();
        app_config.editor.cursor_shape = config;
        self.store_config(app_config);
    }

    fn true_color(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.true_color = option;
        self.store_config(app_config);
    }

    fn insert_final_newline(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.insert_final_newline = option;
        self.store_config(app_config);
    }

    fn color_modes(&self, option: bool) {
        let mut app_config = self.load_config();
        app_config.editor.color_modes = option;
        self.store_config(app_config);
    }

    fn gutters(&self, config: GutterConfig) {
        let mut app_config = self.load_config();
        app_config.editor.gutters = config;
        self.store_config(app_config);
    }

    fn file_picker(&self, picker: FilePickerConfig) {
        let mut app_config = self.load_config();
        app_config.editor.file_picker = picker;
        self.store_config(app_config);
    }

    fn statusline(&self, config: StatusLineConfig) {
        let mut app_config = self.load_config();
        app_config.editor.statusline = config;
        self.store_config(app_config);
    }

    fn undercurl(&self, undercurl: bool) {
        let mut app_config = self.load_config();
        app_config.editor.undercurl = undercurl;
        self.store_config(app_config);
    }

    fn search(&self, config: SearchConfig) {
        let mut app_config = self.load_config();
        app_config.editor.search = config;
        self.store_config(app_config);
    }

    fn lsp(&self, config: LspConfig) {
        let mut app_config = self.load_config();
        app_config.editor.lsp = config;
        self.store_config(app_config);
    }

    fn terminal(&self, config: Option<TerminalConfig>) {
        let mut app_config = self.load_config();
        app_config.editor.terminal = config;
        self.store_config(app_config);
    }

    fn rulers(&self, cols: Vec<u16>) {
        let mut app_config = self.load_config();
        app_config.editor.rulers = cols;
        self.store_config(app_config);
    }

    fn whitespace(&self, config: WhitespaceConfig) {
        let mut app_config = self.load_config();
        app_config.editor.whitespace = config;
        self.store_config(app_config);
    }

    fn bufferline(&self, buffer_config: SteelVal) -> anyhow::Result<()> {
        let config = match buffer_config {
            SteelVal::StringV(s) | SteelVal::SymbolV(s) => match s.as_str() {
                "never" => BufferLine::Never,
                "always" => BufferLine::Always,
                "multiple" => BufferLine::Multiple,
                other => anyhow::bail!("Unrecognized bufferline option: {}", other),
            },
            other => anyhow::bail!("Unrecognized bufferline option: {}", other),
        };

        let mut app_config = self.load_config();
        app_config.editor.bufferline = config;
        self.store_config(app_config);

        Ok(())
    }

    fn indent_guides(&self, config: IndentGuidesConfig) {
        let mut app_config = self.load_config();
        app_config.editor.indent_guides = config;
        self.store_config(app_config);
    }

    fn soft_wrap(&self, config: SoftWrap) {
        let mut app_config = self.load_config();
        app_config.editor.soft_wrap = config;
        self.store_config(app_config);
    }

    fn workspace_lsp_roots(&self, roots: Vec<PathBuf>) {
        let mut app_config = self.load_config();
        app_config.editor.workspace_lsp_roots = roots;
        self.store_config(app_config);
    }

    fn default_line_ending(&self, config: LineEndingConfig) {
        let mut app_config = self.load_config();
        app_config.editor.default_line_ending = config;
        self.store_config(app_config);
    }

    fn smart_tab(&self, config: Option<SmartTabConfig>) {
        let mut app_config = self.load_config();
        app_config.editor.smart_tab = config;
        self.store_config(app_config);
    }

    fn rainbow_brackets(&self, config: bool) {
        let mut app_config = self.load_config();
        app_config.editor.rainbow_brackets = config;
        self.store_config(app_config);
    }
}

// Get doc from function ptr table, hack
fn get_doc_for_global(engine: &mut Engine, ident: &str) -> Option<String> {
    if engine.global_exists(ident) {
        let readable_globals = engine.readable_globals(GLOBAL_OFFSET.load(Ordering::Relaxed));

        for global in readable_globals {
            if global.resolve() == ident {
                return engine.get_doc_for_identifier(ident);
            }
        }

        None
    } else {
        None
    }
}

/// Run the initialization script located at `$helix_config/init.scm`
/// This runs the script in the global environment, and does _not_ load it as a module directly
fn run_initialization_script(
    cx: &mut Context,
    configuration: Arc<ArcSwapAny<Arc<Config>>>,
    language_configuration: Arc<ArcSwap<syntax::Loader>>,
    event_reader: TerminalEventReaderHandle,
) {
    install_event_reader(event_reader);

    // Hack:
    // This might be fussed with, and under re initialization we want
    // to reset this back to what it was before.
    cx.editor.editor_clipping = ClippingConfiguration::default();

    log::info!("Loading init.scm...");

    let helix_module_path = helix_module_file();

    // TODO: Report the error from requiring the file!
    enter_engine(|guard| {
        // Embed the configuration so we don't have to communicate over the refresh
        // channel. The state is still stored within the `Application` struct, but
        // now we can just access it and signal a refresh of the config when we need to.
        guard.update_value(
            "*helix.config*",
            HelixConfiguration {
                configuration,
                language_configuration,
            }
            .into_steelval()
            .unwrap(),
        );

        if helix_module_path.exists() {
            let res = guard.run_with_reference(
                cx,
                "*helix.cx*",
                &format!(r#"(require {:?})"#, helix_module_path.to_str().unwrap()),
            );

            // Present the error in the helix.scm loading
            if let Err(e) = res {
                present_error_inside_engine_context(cx, guard, e);
                return;
            }
        } else {
            println!("Unable to find the `helix.scm` file, creating....");
            std::fs::write(helix_module_path, "").ok();
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
                Err(e) => present_error_inside_engine_context(cx, guard, e),
            }

            log::info!("Finished loading init.scm!")
        } else {
            log::info!("No init.scm found, skipping loading.");
            std::fs::write(helix_module_path, "").ok();
        }
    });
}

impl Custom for PromptEvent {}

impl<'a> CustomReference for Context<'a> {}

steel::custom_reference!(Context<'a>);

fn get_themes(cx: &mut Context) -> Vec<String> {
    ui::completers::theme(cx.editor, "")
        .into_iter()
        .map(|x| x.1.content.to_string())
        .collect()
}

/// A dynamic component, used for rendering thing
impl Custom for compositor::EventResult {}

pub struct WrappedDynComponent {
    pub(crate) inner: Option<Box<dyn Component + Send + Sync + 'static>>,
}

impl Custom for WrappedDynComponent {}

pub struct BoxDynComponent {
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
        Some(self.inner.type_name())
    }

    fn name(&self) -> Option<&str> {
        self.inner.name()
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

#[derive(Debug, Clone, Copy)]
struct OnModeSwitchEvent {
    old_mode: Mode,
    new_mode: Mode,
}

impl OnModeSwitchEvent {
    pub fn get_old_mode(&self) -> Mode {
        self.old_mode
    }

    pub fn get_new_mode(&self) -> Mode {
        self.new_mode
    }
}

impl Custom for OnModeSwitchEvent {}
impl Custom for MappableCommand {}

// Don't take the function name, just take the function itself?
fn register_hook(event_kind: String, callback_fn: SteelVal) -> steel::UnRecoverableResult {
    let rooted = callback_fn.as_rooted();
    let generation = load_generation();

    match event_kind.as_str() {
        "on-mode-switch" => {
            register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
                if let Err(e) = enter_engine(|guard| {
                    if !is_current_generation(generation) {
                        return Ok(SteelVal::Void);
                    }

                    let minimized_event = OnModeSwitchEvent {
                        old_mode: event.old_mode,
                        new_mode: event.new_mode,
                    };

                    guard.with_mut_reference(event.cx).consume(|engine, args| {
                        let context = args[0].clone();
                        engine.update_value("*helix.cx*", context);
                        let mut args = [minimized_event.into_steelval().unwrap()];
                        engine.call_function_with_args_from_mut_slice(
                            rooted.value().clone(),
                            &mut args,
                        )
                    })
                }) {
                    event.cx.editor.set_error(e.to_string());
                }

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }
        "post-insert-char" => {
            register_hook!(move |event: &mut PostInsertChar<'_, '_>| {
                if let Err(e) = enter_engine(|guard| {
                    if !is_current_generation(generation) {
                        return Ok(SteelVal::Void);
                    }

                    guard.with_mut_reference(event.cx).consume(|engine, args| {
                        let context = args[0].clone();
                        engine.update_value("*helix.cx*", context);
                        let mut args = [event.c.into()];
                        engine.call_function_with_args_from_mut_slice(
                            rooted.value().clone(),
                            &mut args,
                        )
                    })
                }) {
                    event.cx.editor.set_error(e.to_string());
                }

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }
        // Register hook - on save?
        "post-command" => {
            register_hook!(move |event: &mut PostCommand<'_, '_>| {
                if let Err(e) = enter_engine(|guard| {
                    if !is_current_generation(generation) {
                        return Ok(SteelVal::Void);
                    }

                    guard.with_mut_reference(event.cx).consume(|engine, args| {
                        let context = args[0].clone();
                        engine.update_value("*helix.cx*", context);
                        let mut args = [event.command.name().into_steelval().unwrap()];
                        engine.call_function_with_args_from_mut_slice(
                            rooted.value().clone(),
                            &mut args,
                        )
                    })
                }) {
                    event.cx.editor.set_error(e.to_string());
                }

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }

        "document-focus-lost" => {
            // TODO: Pass the information from the event in here - the doc id
            // is probably the most helpful so that way we can look the document up
            // and act accordingly?
            register_hook!(move |event: &mut DocumentFocusLost<'_>| {
                let cloned_func = rooted.value().clone();
                let doc_id = event.doc;

                let callback = move |editor: &mut Editor,
                                     _compositor: &mut Compositor,
                                     jobs: &mut job::Jobs| {
                    let mut ctx = Context {
                        register: None,
                        count: None,
                        editor,
                        callback: Vec::new(),
                        on_next_key_callback: None,
                        jobs,
                    };
                    enter_engine(|guard| {
                        if !is_current_generation(generation) {
                            return;
                        }

                        if let Err(e) = guard
                            .with_mut_reference::<Context, Context>(&mut ctx)
                            .consume(move |engine, args| {
                                let context = args[0].clone();
                                engine.update_value("*helix.cx*", context);
                                let mut args = [doc_id.into_steelval().unwrap()];

                                // TODO: Do something with this error!
                                engine.call_function_with_args_from_mut_slice(
                                    cloned_func.clone(),
                                    &mut args,
                                )
                            })
                        {
                            present_error_inside_engine_context(&mut ctx, guard, e);
                        }
                    })
                };
                job::dispatch_blocking_jobs(callback);

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }

        "selection-did-change" => {
            // TODO: Pass the information from the event in here - the doc id
            // is probably the most helpful so that way we can look the document up
            // and act accordingly?
            register_hook!(move |event: &mut SelectionDidChange<'_>| {
                let cloned_func = rooted.value().clone();
                let view_id = event.view;

                let callback = move |editor: &mut Editor,
                                     _compositor: &mut Compositor,
                                     jobs: &mut job::Jobs| {
                    let mut ctx = Context {
                        register: None,
                        count: None,
                        editor,
                        callback: Vec::new(),
                        on_next_key_callback: None,
                        jobs,
                    };
                    enter_engine(|guard| {
                        if !is_current_generation(generation) {
                            return;
                        }

                        if let Err(e) = guard
                            .with_mut_reference::<Context, Context>(&mut ctx)
                            .consume(move |engine, args| {
                                let context = args[0].clone();
                                engine.update_value("*helix.cx*", context);
                                // TODO: Reuse this allocation
                                let mut args = [view_id.into_steelval().unwrap()];
                                engine.call_function_with_args_from_mut_slice(
                                    cloned_func.clone(),
                                    &mut args,
                                )
                            })
                        {
                            present_error_inside_engine_context(&mut ctx, guard, e);
                        }
                    })
                };
                job::dispatch_blocking_jobs(callback);

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }

        "document-opened" => {
            // TODO: Share this code with the above since most of it is
            // exactly the same
            register_hook!(move |event: &mut DocumentDidOpen<'_>| {
                let cloned_func = rooted.value().clone();
                let doc_id = event.doc;

                let callback = move |editor: &mut Editor,
                                     _compositor: &mut Compositor,
                                     jobs: &mut job::Jobs| {
                    let mut ctx = Context {
                        register: None,
                        count: None,
                        editor,
                        callback: Vec::new(),
                        on_next_key_callback: None,
                        jobs,
                    };
                    enter_engine(|guard| {
                        if !is_current_generation(generation) {
                            return;
                        }

                        if let Err(e) = guard
                            .with_mut_reference::<Context, Context>(&mut ctx)
                            .consume(move |engine, args| {
                                let context = args[0].clone();
                                engine.update_value("*helix.cx*", context);
                                // TODO: Reuse this allocation if possible
                                let mut args = [doc_id.into_steelval().unwrap()];
                                engine.call_function_with_args_from_mut_slice(
                                    cloned_func.clone(),
                                    &mut args,
                                )
                            })
                        {
                            present_error_inside_engine_context(&mut ctx, guard, e);
                        }
                    })
                };
                job::dispatch_blocking_jobs(callback);

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }

        "document-saved" => {
            // TODO: Share this code with the above since most of it is
            // exactly the same
            register_hook!(move |event: &mut DocumentSaved<'_>| {
                let cloned_func = rooted.value().clone();
                let doc_id = event.doc;

                let callback = move |editor: &mut Editor,
                                     _compositor: &mut Compositor,
                                     jobs: &mut job::Jobs| {
                    let mut ctx = Context {
                        register: None,
                        count: None,
                        editor,
                        callback: Vec::new(),
                        on_next_key_callback: None,
                        jobs,
                    };
                    enter_engine(|guard| {
                        if !is_current_generation(generation) {
                            return;
                        }

                        if let Err(e) = guard
                            .with_mut_reference::<Context, Context>(&mut ctx)
                            .consume(move |engine, args| {
                                let context = args[0].clone();
                                engine.update_value("*helix.cx*", context);
                                // TODO: Reuse this allocation if possible
                                let mut args = [doc_id.into_steelval().unwrap()];
                                engine.call_function_with_args_from_mut_slice(
                                    cloned_func.clone(),
                                    &mut args,
                                )
                            })
                        {
                            present_error_inside_engine_context(&mut ctx, guard, e);
                        }
                    })
                };
                job::dispatch_blocking_jobs(callback);

                Ok(())
            });

            Ok(SteelVal::Void).into()
        }

        _ => steelerr!(Generic => "Unable to register hook: Unknown event type: {}", event_kind)
            .into(),
    }
}

fn configure_lsp_globals() {
    use std::fmt::Write;
    let mut path = steel_lsp_home_dir();
    path.push("_helix-global-builtins.scm");

    let mut output = String::new();

    let names = &[
        "*helix.cx*",
        "*helix.config*",
        "*helix.id*",
        "register-hook!",
        "log::info!",
        "fuzzy-match",
        "helix-find-workspace",
        "find-workspace",
        "doc-id->usize",
        "new-component!",
        "acquire-context-lock",
        "SteelDynamicComponent?",
        "prompt",
        "picker",
        "#%exp-picker",
        "Component::Text",
        "hx.create-directory",
    ];

    for value in names {
        writeln!(&mut output, "(#%register-global '{})", value).unwrap();
    }

    writeln!(&mut output).unwrap();
    let search_path = helix_loader::config_dir();
    let search_path_str = search_path.to_str().unwrap();

    #[cfg(target_os = "windows")]
    let search_path_str: String = search_path_str.escape_default().collect();

    writeln!(
        &mut output,
        "(#%register-additional-search-path \"{}\")",
        search_path_str
    )
    .unwrap();

    for dir in helix_loader::runtime_dirs() {
        let dir = dir.to_str().unwrap();

        #[cfg(target_os = "windows")]
        let dir: String = dir.escape_default().collect();

        writeln!(
            &mut output,
            "(#%register-additional-search-path \"{}\")",
            dir
        )
        .unwrap();
    }

    std::fs::write(path, output).unwrap();
}

fn configure_lsp_builtins(name: &str, module: &BuiltInModule) {
    use std::fmt::Write;
    let mut path = steel_lsp_home_dir();
    path.push(format!("_helix-{}-builtins.scm", name));

    let mut output = String::new();

    output.push_str(&format!(
        r#"(define #%helix-{}-module (#%module "{}"))

(define (register-values module values)
  (map (lambda (ident) (#%module-add module (symbol->string ident) void)) values))
"#,
        name,
        module.name()
    ));

    output.push_str(&format!(r#"(register-values #%helix-{}-module '("#, name));

    for value in module.names() {
        writeln!(&mut output, "{}", value).unwrap();
    }

    output.push_str("))");

    for value in module.names() {
        if let Some(doc) = module.get_documentation(&value) {
            output.push_str(&format!(
                "(#%module-add-doc #%helix-{}-module {:?} {:?})\n",
                name, value, doc
            ));
        }
    }

    std::fs::write(path, output).unwrap();
}

fn load_rope_api(engine: &mut Engine, generate_sources: bool) {
    // Wrap the rope module?
    let rope_slice_module = rope_module();

    if generate_sources {
        configure_lsp_builtins("rope", &rope_slice_module);
    }

    engine.register_module(rope_slice_module);
}

fn load_misc_api(engine: &mut Engine, generate_sources: bool) {
    let mut module = BuiltInModule::new("helix/core/misc");

    let mut builtin_misc_module = if generate_sources {
        "(require-builtin helix/core/misc as helix.)".to_string()
    } else {
        "".to_string()
    };

    let mut template_function_arity_0 = |name: &str, doc: &str| {
        if generate_sources {
            let doc = format_docstring(doc);
            builtin_misc_module.push_str(&format!(
                r#"
(provide {})
;;@doc
{}
(define ({})
    (helix.{} *helix.cx*))
"#,
                name, doc, name, name
            ));
        }
    };

    // Arity 0
    module.register_fn("hx.cx->pos", cx_pos_within_text);
    module.register_fn("cursor-position", cx_pos_within_text);
    module.register_fn("mode-switch-old", OnModeSwitchEvent::get_old_mode);
    module.register_fn("mode-switch-new", OnModeSwitchEvent::get_new_mode);

    template_function_arity_0("hx.cx->pos", "DEPRECATED: Please use `cursor-position`");
    template_function_arity_0(
        "cursor-position",
        "Returns the cursor position within the current buffer as an integer",
    );

    module.register_fn("get-active-lsp-clients", get_active_lsp_clients);
    template_function_arity_0(
        "get-active-lsp-clients",
        "Get all language servers, that are attached to the current buffer",
    );

    let mut template_function_no_context = |name: &str, doc: &str| {
        if generate_sources {
            let docstring = format_docstring(doc);

            builtin_misc_module.push_str(&format!(
                r#"
(provide {})
;;@doc
{}
(define {} helix.{})                
            "#,
                name, docstring, name, name
            ))
        }
    };

    template_function_no_context(
        "mode-switch-old",
        "Return the old mode from the event payload",
    );
    template_function_no_context(
        "mode-switch-new",
        "Return the new mode from the event payload",
    );

    module.register_fn("lsp-client-initialized?", is_lsp_client_initialized);
    template_function_no_context(
        "lsp-client-initialized?",
        "Return if the lsp client is initialized",
    );
    module.register_fn("lsp-client-name", lsp_client_name);
    template_function_no_context("lsp-client-name", "Get the name of the lsp client");
    module.register_fn("lsp-client-offset-encoding", lsp_client_offset_encoding);
    template_function_no_context(
        "lsp-client-offset-encoding",
        "Get the offset encoding of the lsp client",
    );

    let mut template_function_arity_1 = |name: &str, doc: &str| {
        let doc = format_docstring(doc);
        if generate_sources {
            builtin_misc_module.push_str(&format!(
                r#"
(provide {})
;;@doc
{}
(define ({} arg)
    (helix.{} *helix.cx* arg))
"#,
                name, doc, name, name
            ));
        }
    };

    macro_rules! register_1 {
        ($name:expr, $func:expr, $doc:expr) => {{
            module.register_fn($name, $func);
            template_function_arity_1($name, $doc);
        }};
    }

    // TODO: Get rid of the `hx.` prefix
    register_1!(
        "hx.custom-insert-newline",
        custom_insert_newline,
        "DEPRECATED: Please use `insert-newline-hook`"
    );
    register_1!(
        "insert-newline-hook",
        custom_insert_newline,
        r#"Inserts a new line with the provided indentation.

```scheme
(insert-newline-hook indent-string)
```

indent-string : string?

"#
    );
    register_1!(
        "push-component!",
        push_component,
        r#"
Push a component on to the top of the stack.

```scheme
(push-component! component)
```

component : WrappedDynComponent?
        "#
    );

    // Arity 1
    register_1!(
        "pop-last-component!",
        pop_last_component_by_name,
        "DEPRECATED: Please use `pop-last-component-by-name!`"
    );
    register_1!(
        "pop-last-component-by-name!",
        pop_last_component_by_name,
        r#"Pops the last component off of the stack by name. In other words,
it removes the component matching this name from the stack.

```scheme
(pop-last-component-by-name! name)
```

name : string?
        "#
    );

    register_1!(
        "enqueue-thread-local-callback",
        enqueue_command,
        r#"
Enqueue a function to be run following this context of execution. This could
be useful for yielding back to the editor in the event you want updates to happen
before your function is run.

```scheme
(enqueue-thread-local-callback callback)
```

callback : (-> any?)
    Function with no arguments.

# Examples

```scheme
(enqueue-thread-local-callback (lambda () (theme "focus_nova")))
```
        "#
    );

    register_1!(
        "set-status!",
        set_status,
        "Sets the content of the status line, with the info severity"
    );

    register_1!(
        "set-warning!",
        set_warning,
        "Sets the content of the status line, with the warning severity"
    );

    register_1!(
        "set-error!",
        set_error,
        "Sets the content of the status line, with the error severity"
    );

    module.register_fn("send-lsp-command", send_arbitrary_lsp_command);
    module.register_fn("send-lsp-notification", send_arbitrary_lsp_notification);
    if generate_sources {
        builtin_misc_module.push_str(
            r#"
    (provide send-lsp-command)
    ;;@doc
    ;; Send an lsp command. The `lsp-name` must correspond to an active lsp.
    ;; The method name corresponds to the method name that you'd expect to see
    ;; with the lsp, and the params can be passed as a hash table. The callback
    ;; provided will be called with whatever result is returned from the LSP,
    ;; deserialized from json to a steel value.
    ;; 
    ;; # Example
    ;; ```scheme
    ;; (define (view-crate-graph)
    ;;   (send-lsp-command "rust-analyzer"
    ;;                     "rust-analyzer/viewCrateGraph"
    ;;                     (hash "full" #f)
    ;;                     ;; Callback to run with the result
    ;;                     (lambda (result) (displayln result))))
    ;; ```
    (define (send-lsp-command lsp-name method-name params callback)
        (helix.send-lsp-command *helix.cx* lsp-name method-name params callback))
            "#,
        );
    }

    if generate_sources {
        builtin_misc_module.push_str(
            r#"
    (provide send-lsp-notification)
    ;;@doc
    ;; Send an LSP notification. The `lsp-name` must correspond to an active LSP.
    ;; The method name corresponds to the method name that you'd expect to see
    ;; with the LSP, and the params can be passed as a hash table. Unlike
    ;; `send-lsp-command`, this does not expect a response and is used for
    ;; fire-and-forget notifications.
    ;; 
    ;; # Example
    ;; ```scheme
    ;; (send-lsp-notification "copilot"
    ;;                        "textDocument/didShowCompletion"
    ;;                        (hash "item"
    ;;                              (hash "insertText" "a helpful suggestion"
    ;;                                    "range" (hash "start" (hash "line" 1 "character" 0)
    ;;                                                  "end" (hash "line" 1 "character" 2)))))
    ;; ```
    (define (send-lsp-notification lsp-name method-name params)
        (helix.send-lsp-notification *helix.cx* lsp-name method-name params))
            "#,
        );
    }

    module.register_fn("lsp-reply-ok", lsp_reply_ok);
    if generate_sources {
        builtin_misc_module.push_str(
            r#"
    (provide lsp-reply-ok)
    ;;@doc
    ;; Send a successful reply to an LSP request with the given result.
    ;;
    ;; ```scheme
    ;; (lsp-reply-ok lsp-name request-id result)
    ;; ```
    ;; 
    ;; * lsp-name : string? - Name of the language server
    ;; * request-id : string? - ID of the request to respond to  
    ;; * result : any? - The result value to send back
    ;;
    ;; # Examples
    ;; ```scheme
    ;; ;; Reply to a request with id "123" from rust-analyzer
    ;; (lsp-reply-ok "rust-analyzer" "123" (hash "result" "value"))
    ;; ```
    (define (lsp-reply-ok lsp-name request-id result)
        (helix.lsp-reply-ok *helix.cx* lsp-name request-id result))    
            "#,
        );
    }

    macro_rules! register_2_no_context {
        ($name:expr, $func:expr, $doc:expr) => {{
            module.register_fn($name, $func);
            if generate_sources {
                let doc = format_docstring($doc);
                builtin_misc_module.push_str(&format!(
                    r#"
(provide {})
;;@doc
{}
(define ({} arg1 arg2)
    (helix.{} arg1 arg2))
"#,
                    $name, doc, $name, $name
                ));
            }
        }};
    }

    register_2_no_context!(
        "acquire-context-lock",
        acquire_context_lock,
        r#"
Schedule a function to run on the main thread. This is a fairly low level function, and odds are
you'll want to use some abstractions on top of this.

The provided function will get enqueued to run on the main thread, and during the duration of the functions
execution, the provided mutex will be locked.

```scheme
(acquire-context-lock callback-fn mutex)
```

callback-fn : (-> void?)
    Function with no arguments

mutex : mutex?
"#
    );

    let mut template_function_arity_2 = |name: &str, doc: &str| {
        if generate_sources {
            let doc = format_docstring(doc);
            builtin_misc_module.push_str(&format!(
                r#"
(provide {})
;;@doc
{}
(define ({} arg1 arg2)
    (helix.{} *helix.cx* arg1 arg2))
"#,
                name, doc, name, name
            ));
        }
    };

    macro_rules! register_2 {
        ($name:expr, $func:expr, $doc:expr) => {{
            module.register_fn($name, $func);
            template_function_arity_2($name, $doc);
        }};
    }

    // Arity 2
    register_2!(
        "enqueue-thread-local-callback-with-delay",
        enqueue_command_with_delay,
        r#"
Enqueue a function to be run following this context of execution, after a delay. This could
be useful for yielding back to the editor in the event you want updates to happen
before your function is run.

```scheme
(enqueue-thread-local-callback-with-delay delay callback)
```

delay : int?
    Time to delay the callback by in milliseconds

callback : (-> any?)
    Function with no arguments.

# Examples

```scheme
(enqueue-thread-local-callback-with-delay 1000 (lambda () (theme "focus_nova"))) ;; Run after 1 second
``
        "#
    );

    register_2!(
        "helix-await-callback",
        await_value,
        "DEPRECATED: Please use `await-callback`"
    );

    // Arity 2
    register_2!(
        "await-callback",
        await_value,
        r#"
Await the given value, and call the callback function on once the future is completed.

```scheme
(await-callback future callback)
```

* future : future?
* callback (-> any?)
    Function with no arguments"#
    );

    register_2!(
        "add-inlay-hint",
        add_inlay_hint,
        r#"
Warning: this is experimental

Adds an inlay hint at the given character index. Returns the (first-line, last-line) list
associated with this snapshot of the inlay hints. Use this pair of line numbers to invalidate
the inlay hints.

```scheme
(add-inlay-hint char-index completion) -> (list int? int?)
```

char-index : int?
completion : string?

"#
    );
    register_2!(
        "remove-inlay-hint",
        remove_inlay_hint,
        r#"
Warning: this is experimental and should not be used.
This will most likely be removed soon.

Removes an inlay hint at the given character index. Note - to remove
properly, the completion must match what was already there.

```scheme
(remove-inlay-hint char-index completion)
```

char-index : int?
completion : string?

"#
    );

    register_2!(
        "remove-inlay-hint-by-id",
        remove_inlay_hint_by_id,
        r#"
Warning: this is experimental

Removes an inlay hint by the id that was associated with the added inlay hints.

```scheme
(remove-inlay-hint first-line last-line)
```

first-line : int?
last-line : int?

"#
    );

    if generate_sources {
        if let Some(mut target_directory) = alternative_runtime_search_path() {
            if !target_directory.exists() {
                std::fs::create_dir_all(&target_directory).unwrap();
            }

            target_directory.push("misc.scm");

            std::fs::write(target_directory, &builtin_misc_module).unwrap();
        }

        engine.register_steel_module("helix/misc.scm".to_string(), builtin_misc_module);
    }

    if generate_sources {
        configure_lsp_builtins("misc", &module);
    }

    engine.register_module(module);
}

// TODO: Generate sources into the cogs directory, so that the
// LSP can go find it. When it comes to loading though, it'll look
// up internally.
pub fn alternative_runtime_search_path() -> Option<PathBuf> {
    steel_home().map(|path| PathBuf::from(path).join("cogs").join("helix"))
}

pub fn generate_cog_file() {
    if let Some(path) = alternative_runtime_search_path() {
        std::fs::write(
            path.join("cog.scm"),
            r#"(define package-name 'helix)
            (define version "0.1.0")"#,
        )
        .unwrap();
    }
}

pub fn load_ext_api(engine: &mut Engine, generate_sources: bool) {
    let ext_api = r#"
(require "helix/editor.scm")
(require "helix/misc.scm")
(require-builtin helix/core/text as text.)
(require "steel/sync")

(provide eval-buffer
         evalp
         running-on-main-thread?
         hx.with-context
         hx.block-on-task)

(define (get-document-as-slice)
  (let* ([focus (editor-focus)]
         [focus-doc-id (editor->doc-id focus)])
    (text.rope->string (editor->text focus-doc-id))))

;;@doc
;; Eval the current buffer, morally equivalent to load-buffer!
(define (eval-buffer)
  (eval-string (get-document-as-slice)))

;;@doc
;; Eval prompt
(define (evalp)
  (push-component! (prompt "" (lambda (expr) (set-status! (eval-string expr))))))

;;@doc
;; Check what the main thread id is, compare to the main thread
(define (running-on-main-thread?)
  (= (current-thread-id) *helix.id*))

;;@doc
;; If running on the main thread already, just do nothing.
;; Check the ID of the engine, and if we're already on the
;; main thread, just continue as is - i.e. just block. This does
;; not block on the function if this is running on another thread.
;;
;; ```scheme
;; (hx.with-context thunk)
;; ```
;; thunk : (-> any?) ;; Function that has no arguments
;;
;; # Examples
;; ```scheme
;; (spawn-native-thread
;;   (lambda () 
;;     (hx.with-context (lambda () (theme "nord")))))
;; ```
(define (hx.with-context thunk)
  (if (running-on-main-thread?)
      (thunk)
      (begin
        (define task (task #f))
        ;; Send on the main thread
        (acquire-context-lock thunk task)
        task)))

;;@doc
;; Block on the given function.
;; ```scheme
;; (hx.block-on-task thunk)
;; ```
;; thunk : (-> any?) ;; Function that has no arguments
;;
;; # Examples
;; ```scheme
;; (define thread
;;   (spawn-native-thread
;;     (lambda () 
;;       (hx.block-on-task (lambda () (theme "nord") 10)))))
;;
;; ;; Some time later, in a different context - if done at the same time,
;; ;; this will deadline, since the join depends on the callback previously
;; ;; executing.
;; (equal? (thread-join! thread) 10) ;; => #true
;; ```
(define (hx.block-on-task thunk)
  (if (running-on-main-thread?) (thunk) (block-on-task (hx.with-context thunk))))
    "#;

    if let Some(mut target_directory) = alternative_runtime_search_path() {
        if generate_sources {
            if !target_directory.exists() {
                std::fs::create_dir_all(&target_directory).unwrap_or_else(|err| {
                    panic!("Failed to create directory {:?}: {}", target_directory, err)
                });
            }

            target_directory.push("ext.scm");

            std::fs::write(target_directory, ext_api).unwrap();
        }
    }

    engine.register_steel_module("helix/ext.scm".to_string(), ext_api.to_string());
}

// Note: This implementation is aligned with what the steel language server
// expects. This shouldn't stay here, but for alpha purposes its fine.
pub fn steel_lsp_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("STEEL_LSP_HOME") {
        return PathBuf::from(home);
    }

    let mut home_directory =
        PathBuf::from(steel_home().expect("Unable to find steel home location"));
    home_directory.push("lsp");

    if !home_directory.exists() {
        std::fs::create_dir_all(&home_directory).expect("Unable to create the lsp home directory");
    }

    home_directory
}

// Embed them in the binary... first
pub fn configure_builtin_sources(engine: &mut Engine, generate_sources: bool) {
    load_editor_api(engine, generate_sources);
    load_theme_api(engine, generate_sources);
    load_configuration_api(engine, generate_sources);
    load_typed_commands(engine, generate_sources);
    load_static_commands(engine, generate_sources);
    // Note: This is going to be completely revamped soon.
    load_keymap_api(engine, generate_sources);
    load_rope_api(engine, generate_sources);
    load_misc_api(engine, generate_sources);
    load_component_api(engine, generate_sources);

    // This depends on the components and theme api, so should
    // be loaded after.
    load_high_level_theme_api(engine, generate_sources);
    load_high_level_keymap_api(engine, generate_sources);
    load_ext_api(engine, generate_sources);

    // TODO: Remove this once all of the globals have been moved into their own modules
    if generate_sources {
        configure_lsp_globals();

        // Generate cog file for the stubs
        // that are generated and written to the $STEEL_HOME directory
        generate_cog_file()
    }
}

fn acquire_context_lock(
    callback_fn: SteelVal,
    place: Option<SteelVal>,
) -> steel::rvals::Result<()> {
    static TASK_DONE: Lazy<SteelVal> = Lazy::new(|| SteelVal::SymbolV("done".into()));

    match (&callback_fn, &place) {
        (SteelVal::Closure(_), Some(SteelVal::CustomStruct(_))) => {}
        _ => {
            steel::stop!(TypeMismatch => "acquire-context-lock expected a 
                        callback function and a task object")
        }
    }

    let rooted = callback_fn.as_rooted();
    let rooted_place = place.map(|x| x.as_rooted());

    let callback = move |editor: &mut Editor,
                         _compositor: &mut Compositor,
                         jobs: &mut job::Jobs| {
        let mut ctx = Context {
            register: None,
            count: None,
            editor,
            callback: Vec::new(),
            on_next_key_callback: None,
            jobs,
        };

        let cloned_func = rooted.value();
        let cloned_place = rooted_place.as_ref().map(|x| x.value());

        enter_engine(|guard| {
            if let Err(e) = guard
                .with_mut_reference::<Context, Context>(&mut ctx)
                // Block until the other thread is finished in its critical
                // section...
                .consume(move |engine, args| {
                    let context = args[0].clone();
                    engine.update_value("*helix.cx*", context);

                    let mut lock = None;

                    if let Some(SteelVal::CustomStruct(s)) = cloned_place {
                        let mutex = s.get_mut_index(0).unwrap();
                        lock = Some(mutex_lock(&mutex).unwrap());
                    }

                    // Acquire lock, wait until its done
                    let result = engine.call_function_with_args(cloned_func.clone(), Vec::new());

                    if let Some(SteelVal::CustomStruct(s)) = cloned_place {
                        match result {
                            Ok(result) => {
                                // Store the result of the callback so that the
                                // next downstream user can handle it.
                                s.set_index(2, result);

                                // Set the task to be done
                                s.set_index(1, (*TASK_DONE).clone());

                                mutex_unlock(&lock.unwrap()).unwrap();
                            }

                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }

                    Ok(())
                })
            {
                present_error_inside_engine_context(&mut ctx, guard, e);
            }
        })
    };
    job::dispatch_blocking_jobs(callback);

    Ok(())
}

fn configure_engine_impl(mut engine: Engine) -> Engine {
    log::info!("Loading engine!");

    // Engine: Add search directories.
    engine.add_search_directory(helix_loader::config_dir());

    for dir in helix_loader::runtime_dirs() {
        engine.add_search_directory(dir.to_owned());
    }

    engine.register_value("*helix.cx*", SteelVal::Void);
    engine.register_value("*helix.config*", SteelVal::Void);
    engine.register_value(
        "*helix.id*",
        SteelVal::IntV(engine.engine_id().as_usize() as _),
    );

    configure_builtin_sources(&mut engine, true);

    // Hooks
    engine.register_fn("register-hook!", register_hook);
    engine.register_fn("log::info!", |message: SteelVal| {
        if let SteelVal::StringV(s) = &message {
            log::info!("{}", s)
        } else {
            log::info!("{}", message)
        }
    });

    engine.register_fn("fuzzy-match", |pattern: SteelString, items: SteelVal| {
        if let SteelVal::ListV(l) = items {
            let res = helix_core::fuzzy::fuzzy_match(
                pattern.as_str(),
                l.iter().filter_map(|x| x.as_string().map(|x| x.as_str())),
                false,
            );

            return res
                .into_iter()
                .map(|x| x.0.to_string().into())
                .collect::<Vec<SteelVal>>();
        }

        Vec::new()
    });

    // Find the workspace
    engine.register_fn("helix-find-workspace", || {
        helix_core::find_workspace().0.to_str().unwrap().to_string()
    });

    // TODO: Deprecate the above
    engine.register_fn("find-workspace", || {
        helix_core::find_workspace().0.to_str().unwrap().to_string()
    });

    engine.register_fn("doc-id->usize", document_id_to_usize);

    // TODO: Remove that this is now in helix/core/misc
    engine.register_fn("acquire-context-lock", acquire_context_lock);

    engine.register_fn("SteelDynamicComponent?", |object: SteelVal| {
        if let SteelVal::Custom(v) = object {
            if let Some(wrapped) = v.read().as_any_ref().downcast_ref::<BoxDynComponent>() {
                return wrapped.inner.as_any().is::<SteelDynamicComponent>();
            } else {
                false
            }
        } else {
            false
        }
    });

    engine.register_fn(
        "prompt",
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

                    with_interrupt_handler(|| {
                        enter_engine(|guard| {
                            if let Err(e) = guard
                                .with_mut_reference::<Context, Context>(&mut ctx)
                                .consume(move |engine, args| {
                                    let context = args[0].clone();

                                    engine.update_value("*helix.cx*", context);

                                    engine.call_function_with_args(
                                        cloned_func.clone(),
                                        vec![input.into_steelval().unwrap()],
                                    )
                                })
                            {
                                present_error_inside_engine_context(&mut ctx, guard, e);
                            }
                        })
                    })
                },
            );

            WrappedDynComponent {
                inner: Some(Box::new(prompt)),
            }
        },
    );

    engine.register_fn("picker", |values: Vec<String>| -> WrappedDynComponent {
        let columns = [PickerColumn::new(
            "path",
            |item: &PathBuf, root: &PathBuf| {
                item.strip_prefix(root)
                    .unwrap_or(item)
                    .to_string_lossy()
                    .into()
            },
        )];
        let cwd = helix_stdx::env::current_working_dir();

        let picker = ui::Picker::new(columns, 0, [], cwd, move |cx, path: &PathBuf, action| {
            if let Err(e) = cx.editor.open(path, action) {
                let err = if let Some(err) = e.source() {
                    format!("{}", err)
                } else {
                    format!("unable to open \"{}\"", path.display())
                };
                cx.editor.set_error(err);
            }
        })
        .with_preview(|_editor, path| Some((PathOrId::Path(path), None)));

        let injector = picker.injector();

        for file in values {
            if injector.push(PathBuf::from(file)).is_err() {
                break;
            }
        }

        WrappedDynComponent {
            inner: Some(Box::new(ui::overlay::overlaid(picker))),
        }
    });

    // Experimental - use at your own risk.
    engine.register_fn(
        "#%exp-picker",
        |values: Vec<String>, callback_fn: SteelVal| -> WrappedDynComponent {
            let columns = [PickerColumn::new(
                "path",
                |item: &PathBuf, root: &PathBuf| {
                    item.strip_prefix(root)
                        .unwrap_or(item)
                        .to_string_lossy()
                        .into()
                },
            )];
            let cwd = helix_stdx::env::current_working_dir();

            let rooted = callback_fn.as_rooted();

            let picker = ui::Picker::new(columns, 0, [], cwd, move |cx, path: &PathBuf, action| {
                let result = cx.editor.open(path, action);
                match result {
                    Err(e) => {
                        let err = if let Some(err) = e.source() {
                            format!("{}", err)
                        } else {
                            format!("unable to open \"{}\"", path.display())
                        };
                        cx.editor.set_error(err);
                    }
                    Ok(_) => {
                        let mut ctx = Context {
                            register: None,
                            count: None,
                            editor: cx.editor,
                            callback: Vec::new(),
                            on_next_key_callback: None,
                            jobs: cx.jobs,
                        };

                        let cloned_func = rooted.value();

                        enter_engine(|guard| {
                            if let Err(e) = guard
                                .with_mut_reference::<Context, Context>(&mut ctx)
                                .consume(move |engine, args| {
                                    let context = args[0].clone();
                                    engine.update_value("*helix.cx*", context);
                                    engine.call_function_with_args(cloned_func.clone(), Vec::new())
                                })
                            {
                                present_error_inside_engine_context(&mut ctx, guard, e);
                            }
                        })
                    }
                }
            })
            .with_preview(|_editor, path| Some((PathOrId::Path(path), None)));

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

    // Create directory since we can't do that in the current state
    engine.register_fn("hx.create-directory", create_directory);

    GLOBAL_OFFSET.store(engine.globals().len(), Ordering::Relaxed);

    engine
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

fn push_range_to_selection(cx: &mut Context, range: Range) {
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id).clone();
    doc.set_selection(view.id, selection.push(range))
}

fn set_selection_primary_index(cx: &mut Context, primary_index: usize) {
    let (view, doc) = current!(cx.editor);
    let mut selection = doc.selection(view.id).clone();
    selection.set_primary_index(primary_index);
    doc.set_selection(view.id, selection)
}

fn remove_selection_range(cx: &mut Context, index: usize) {
    let (view, doc) = current!(cx.editor);
    let selection = doc.selection(view.id).clone();
    doc.set_selection(view.id, selection.remove(index))
}

fn current_line_number(cx: &mut Context) -> usize {
    let (view, doc) = current_ref!(cx.editor);
    doc.text().char_to_line(
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
    )
}

fn current_column_number(cx: &mut Context) -> usize {
    let (view, doc) = current_ref!(cx.editor);
    helix_core::coords_at_pos(
        doc.text().slice(..),
        doc.selection(view.id)
            .primary()
            .cursor(doc.text().slice(..)),
    )
    .col
}

fn current_line_character(cx: &mut Context, encoding: SteelString) -> anyhow::Result<usize> {
    let (view, doc) = current_ref!(cx.editor);

    let encoding = match &***encoding {
        "utf-8" => helix_lsp::OffsetEncoding::Utf8,
        "utf-16" => helix_lsp::OffsetEncoding::Utf16,
        "utf-32" => helix_lsp::OffsetEncoding::Utf32,
        _ => anyhow::bail!("invalid encoding {encoding:?}"),
    };

    Ok(doc.position(view.id, encoding).character as usize)
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

// TODO: Replace with eval-string
pub fn run_expression_in_engine(cx: &mut Context, text: String) -> anyhow::Result<()> {
    let callback = async move {
        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut ctx = Context {
                    register: None,
                    count: None,
                    editor,
                    callback: Vec::new(),
                    on_next_key_callback: None,
                    jobs,
                };

                let output = enter_engine(|guard| {
                    guard
                        .with_mut_reference::<Context, Context>(&mut ctx)
                        .consume(move |engine, args| {
                            let context = args[0].clone();
                            engine.update_value("*helix.cx*", context);

                            engine.compile_and_run_raw_program(text.clone())
                        })
                });

                match output {
                    Ok(output) => {
                        let (output, _success) = (Tendril::from(format!("{:?}", output)), true);

                        let contents = ui::Markdown::new(
                            format!("```\n{}\n```", output),
                            editor.syn_loader.clone(),
                        );
                        let popup = Popup::new("engine", contents).position(Some(
                            helix_core::Position::new(editor.cursor().0.unwrap_or_default().row, 2),
                        ));
                        compositor.replace_or_push("engine", popup);
                    }
                    Err(e) => enter_engine(|x| present_error_inside_engine_context(&mut ctx, x, e)),
                }
            },
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);

    Ok(())
}

pub fn load_buffer(cx: &mut Context) -> anyhow::Result<()> {
    let (text, path) = {
        let (_, doc) = current!(cx.editor);

        let text = doc.text().to_string();
        let path = current_path(cx);

        (text, path)
    };

    let callback = async move {
        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut ctx = Context {
                    register: None,
                    count: None,
                    editor,
                    callback: Vec::new(),
                    on_next_key_callback: None,
                    jobs,
                };

                let output = enter_engine(|guard| {
                    guard
                        .with_mut_reference::<Context, Context>(&mut ctx)
                        .consume(move |engine, args| {
                            let context = args[0].clone();
                            engine.update_value("*helix.cx*", context);

                            match path.clone() {
                                Some(path) => engine.compile_and_run_raw_program_with_path(
                                    // TODO: Figure out why I have to clone this text here.
                                    text.clone(),
                                    PathBuf::from(path),
                                ),
                                None => engine.compile_and_run_raw_program(text.clone()),
                            }
                        })
                });

                match output {
                    Ok(output) => {
                        let (output, _success) = (Tendril::from(format!("{:?}", output)), true);

                        let contents = ui::Markdown::new(
                            format!("```\n{}\n```", output),
                            editor.syn_loader.clone(),
                        );
                        let popup = Popup::new("engine", contents).position(Some(
                            helix_core::Position::new(editor.cursor().0.unwrap_or_default().row, 2),
                        ));
                        compositor.replace_or_push("engine", popup);
                    }
                    Err(e) => enter_engine(|x| present_error_inside_engine_context(&mut ctx, x, e)),
                }
            },
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);

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

fn set_buffer_uri(cx: &mut Context, uri: SteelString) -> anyhow::Result<()> {
    let current_focus = cx.editor.tree.focus;
    let view = cx.editor.tree.get(current_focus);
    let doc = &view.doc;
    // Lifetime of this needs to be tied to the existing document
    let current_doc = cx.editor.documents.get_mut(doc);

    if let Some(current_doc) = current_doc {
        if let Ok(url) = url::Url::from_str(uri.as_str()) {
            current_doc.uri = Some(Box::new(url));
        } else {
            anyhow::bail!("Unable to parse uri: {:?}", uri);
        }
    }

    Ok(())
}

fn cx_current_focus(cx: &mut Context) -> helix_view::ViewId {
    cx.editor.tree.focus
}

fn cx_get_document_id(cx: &mut Context, view_id: helix_view::ViewId) -> DocumentId {
    cx.editor.tree.get(view_id).doc
}

fn document_id_to_text(cx: &mut Context, doc_id: DocumentId) -> Option<SteelRopeSlice> {
    cx.editor
        .documents
        .get(&doc_id)
        .map(|x| SteelRopeSlice::new(x.text().clone()))
}

fn cx_is_document_in_view(cx: &mut Context, doc_id: DocumentId) -> Option<helix_view::ViewId> {
    cx.editor
        .tree
        .traverse()
        .find(|(_, v)| v.doc == doc_id)
        .map(|(id, _)| id)
}

fn cx_register_value(cx: &mut Context, name: char) -> Vec<String> {
    cx.editor
        .registers
        .read(name, cx.editor)
        .map_or(Vec::new(), |reg| reg.collect())
        .into_iter()
        .map(|value| value.to_string())
        .collect()
}

fn cx_document_exists(cx: &mut Context, doc_id: DocumentId) -> bool {
    cx.editor.documents.get(&doc_id).is_some()
}

fn document_path(cx: &mut Context, doc_id: DocumentId) -> Option<String> {
    cx.editor
        .documents
        .get(&doc_id)
        .and_then(|doc| doc.path().and_then(|x| x.to_str()).map(|x| x.to_string()))
}

fn cx_editor_all_documents(cx: &mut Context) -> Vec<DocumentId> {
    cx.editor.documents.keys().copied().collect()
}

fn cx_get_document_language(cx: &mut Context, doc_id: DocumentId) -> Option<String> {
    cx.editor
        .documents
        .get(&doc_id)
        .and_then(|d| Some(d.language_name()?.to_string()))
}

fn cx_switch(cx: &mut Context, doc_id: DocumentId) {
    cx.editor.switch(doc_id, Action::VerticalSplit)
}

fn cx_switch_action(cx: &mut Context, doc_id: DocumentId, action: Action) {
    cx.editor.switch(doc_id, action)
}

fn cx_get_mode(cx: &mut Context) -> Mode {
    cx.editor.mode
}

fn cx_set_mode(cx: &mut Context, mode: Mode) {
    cx.editor.mode = mode
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

fn pop_last_component_by_name(cx: &mut Context, name: SteelString) {
    let callback = async move {
        let call: Box<dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs)> = Box::new(
            move |_editor: &mut Editor, compositor: &mut Compositor, _jobs: &mut job::Jobs| {
                compositor.remove_by_dynamic_name(&name);
            },
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);
}

fn set_status(cx: &mut Context, value: SteelVal) {
    match value {
        SteelVal::StringV(s) => cx.editor.set_status(s.as_ref().to_owned()),
        _ => cx.editor.set_status(value.to_string()),
    }
}

fn set_warning(cx: &mut Context, value: SteelVal) {
    match value {
        SteelVal::StringV(s) => cx.editor.set_warning(s.as_ref().to_owned()),
        _ => cx.editor.set_warning(value.to_string()),
    }
}

fn set_error(cx: &mut Context, value: SteelVal) {
    match value {
        SteelVal::StringV(s) => cx.editor.set_error(s.as_ref().to_owned()),
        _ => cx.editor.set_error(value.to_string()),
    }
}

fn enqueue_command(cx: &mut Context, callback_fn: SteelVal) {
    let rooted = callback_fn.as_rooted();
    let current_gen = load_generation();

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

                enter_engine(|guard| {
                    if !is_current_generation(current_gen) {
                        return;
                    }

                    if let Err(e) = guard
                        .with_mut_reference::<Context, Context>(&mut ctx)
                        .consume(move |engine, args| {
                            let context = args[0].clone();
                            engine.update_value("*helix.cx*", context);

                            engine.call_function_with_args(cloned_func.clone(), Vec::new())
                        })
                    {
                        present_error_inside_engine_context(&mut ctx, guard, e);
                    }
                })
            },
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);
}

// Apply arbitrary delay for update rate...
fn enqueue_command_with_delay(cx: &mut Context, delay: SteelVal, callback_fn: SteelVal) {
    let rooted = callback_fn.as_rooted();
    let current_gen = load_generation();

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

                enter_engine(|guard| {
                    if !is_current_generation(current_gen) {
                        return;
                    }

                    if let Err(e) = guard
                        .with_mut_reference::<Context, Context>(&mut ctx)
                        .consume(move |engine, args| {
                            let context = args[0].clone();
                            engine.update_value("*helix.cx*", context);

                            engine.call_function_with_args(cloned_func.clone(), Vec::new())
                        })
                    {
                        present_error_inside_engine_context(&mut ctx, guard, e);
                    }
                })
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
    let current_gen = load_generation();

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
                        let callback = move |engine: &mut Engine, args: Vec<SteelVal>| {
                            let context = args[0].clone();
                            engine.update_value("*helix.cx*", context);
                            engine.call_function_with_args(cloned_func.clone(), vec![inner])
                        };

                        enter_engine(|guard| {
                            if !is_current_generation(current_gen) {
                                return;
                            }

                            if let Err(e) = guard
                                .with_mut_reference::<Context, Context>(&mut ctx)
                                .consume_once(callback)
                            {
                                present_error_inside_engine_context(&mut ctx, guard, e);
                            }
                        })
                    }
                    Err(e) => enter_engine(|x| present_error_inside_engine_context(&mut ctx, x, e)),
                }
            },
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);
}
// Check that we successfully created a directory?
fn create_directory(path: String) {
    let path = helix_stdx::path::canonicalize(&path);
    if !path.exists() {
        std::fs::create_dir(path).unwrap();
    }
}

pub fn cx_pos_within_text(cx: &mut Context) -> usize {
    let (view, doc) = current_ref!(cx.editor);

    let text = doc.text().slice(..);

    let selection = doc.selection(view.id).clone();

    selection.primary().cursor(text)
}

pub fn get_helix_cwd(_cx: &mut Context) -> Option<String> {
    helix_stdx::env::current_working_dir()
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

// fn search_in_directory(cx: &mut Context, directory: String) {
//     let buf = PathBuf::from(directory);
//     let search_path = expand_tilde(&buf);
//     let path = search_path.to_path_buf();
//     crate::commands::search_in_directory(cx, path);
// }

// TODO: Result should create unrecoverable result, and should have a special
// recoverable result - that way we can handle both, not one in particular
fn regex_selection(cx: &mut Context, regex: String) {
    if let Ok(regex) = helix_stdx::rope::Regex::new(&regex) {
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

// TODO: Remove this!
fn move_window_to_the_left(cx: &mut Context) {
    while cx
        .editor
        .tree
        .swap_split_in_direction(helix_view::tree::Direction::Left)
        .is_some()
    {}
}

// TODO: Remove this!
fn move_window_to_the_right(cx: &mut Context) {
    while cx
        .editor
        .tree
        .swap_split_in_direction(helix_view::tree::Direction::Right)
        .is_some()
    {}
}

#[derive(Debug, Clone)]
struct LspClient(Weak<helix_lsp::Client>);

impl LspClient {
    fn new(client: Arc<helix_lsp::Client>) -> Self {
        LspClient(Arc::downgrade(&client))
    }
}

impl Custom for LspClient {}

fn get_active_lsp_clients(cx: &mut Context) -> SteelVal {
    let (_, doc) = current!(cx.editor);
    SteelVal::ListV(
        doc.arc_language_servers()
            .map(|client| LspClient::new(client).into_steelval().unwrap())
            .collect(),
    )
}

fn is_lsp_client_initialized(client: LspClient) -> bool {
    let client = client.0.upgrade();
    client.is_some_and(|client| client.is_initialized())
}

fn lsp_client_name(client: LspClient) -> Option<String> {
    let client = client.0.upgrade();
    client.map(|client| client.name().to_owned())
}

fn lsp_client_offset_encoding(client: LspClient) -> Option<&'static str> {
    let client = client.0.upgrade();
    client
        .filter(|client| client.is_initialized())
        .map(|client| match client.offset_encoding() {
            helix_lsp::OffsetEncoding::Utf8 => "utf-8",
            helix_lsp::OffsetEncoding::Utf16 => "utf-16",
            helix_lsp::OffsetEncoding::Utf32 => "utf-32",
        })
}

fn send_arbitrary_lsp_command(
    cx: &mut Context,
    name: SteelString,
    command: SteelString,
    // Arguments - these will be converted to some json stuff
    json_argument: Option<SteelVal>,
    callback_fn: SteelVal,
) -> anyhow::Result<()> {
    let argument = json_argument.map(|x| serde_json::Value::try_from(x).unwrap());

    let (_view, doc) = current!(cx.editor);

    let language_server_id = anyhow::Context::context(
        doc.language_servers().find(|x| x.name() == name.as_str()),
        "Unable to find the language server specified!",
    )?
    .id();

    let future = match cx
        .editor
        .language_server_by_id(language_server_id)
        .and_then(|language_server| {
            language_server.non_standard_extension(command.to_string(), argument)
        }) {
        Some(future) => future,
        None => {
            // TODO: Come up with a better message once we check the capabilities for
            // the arbitrary thing you're trying to do, since for now the above actually
            // always returns a `Some`
            cx.editor.set_error(
                "Language server does not support whatever command you just tried to do",
            );
            return Ok(());
        }
    };

    let rooted = callback_fn.as_rooted();

    create_callback(cx, future, rooted)?;

    Ok(())
}

fn lsp_reply_ok(
    cx: &mut Context,
    name: SteelString,
    id: SteelString,
    result: SteelVal,
) -> anyhow::Result<()> {
    let serde_value: Result<serde_json::Value, steel::SteelErr> = result.try_into();
    let value = match serde_value {
        Ok(serialized_value) => serialized_value,
        Err(error) => {
            log::warn!("Failed to serialize a SteelVal: {}", error);
            serde_json::Value::Null
        }
    };

    let (_view, doc) = current!(cx.editor);

    let language_server_id = anyhow::Context::context(
        doc.language_servers().find(|x| x.name() == name.as_str()),
        "Unable to find the language server specified!",
    )?
    .id();

    cx.editor
        .language_server_by_id(language_server_id)
        .ok_or(anyhow::anyhow!("Failed to find a language server by id"))
        .and_then(|language_server| {
            language_server
                .reply(jsonrpc::Id::Str(id.to_string()), Ok(value))
                .map_err(Into::into)
        })
}

fn create_callback<T: TryInto<SteelVal, Error = SteelErr> + 'static>(
    cx: &mut Context,
    future: impl std::future::Future<Output = Result<T, helix_lsp::Error>> + 'static,
    rooted: steel::RootedSteelVal,
) -> Result<(), anyhow::Error> {
    let callback = async move {
        // Result of the future - this will be whatever we get back
        // from the lsp call
        let res = future.await?;

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

                enter_engine(move |guard| match TryInto::<SteelVal>::try_into(res) {
                    Ok(result) => {
                        if let Err(e) = guard
                            .with_mut_reference::<Context, Context>(&mut ctx)
                            .consume(move |engine, args| {
                                let context = args[0].clone();
                                engine.update_value("*helix.cx*", context);

                                engine.call_function_with_args(
                                    cloned_func.clone(),
                                    vec![result.clone()],
                                )
                            })
                        {
                            present_error_inside_engine_context(&mut ctx, guard, e);
                        }
                    }
                    Err(e) => {
                        present_error_inside_engine_context(&mut ctx, guard, e);
                    }
                })
            },
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);
    Ok(())
}

// "add-inlay-hint",
pub fn add_inlay_hint(
    cx: &mut Context,
    char_index: usize,
    completion: SteelString,
) -> Option<(usize, usize)> {
    let view_id = cx.editor.tree.focus;
    if !cx.editor.tree.contains(view_id) {
        return None;
    }
    let view = cx.editor.tree.get(view_id);
    let doc_id = cx.editor.tree.get(view_id).doc;
    let doc = cx.editor.documents.get_mut(&doc_id)?;
    let mut new_inlay_hints = doc.inlay_hints(view_id).cloned().unwrap_or_else(|| {
        let doc_text = doc.text();
        let len_lines = doc_text.len_lines();

        let view_height = view.inner_height();
        let first_visible_line =
            doc_text.char_to_line(doc.view_offset(view_id).anchor.min(doc_text.len_chars()));
        let first_line = first_visible_line.saturating_sub(view_height);
        let last_line = first_visible_line
            .saturating_add(view_height.saturating_mul(2))
            .min(len_lines);

        let new_doc_inlay_hints_id = DocumentInlayHintsId {
            first_line,
            last_line,
        };

        DocumentInlayHints::empty_with_id(new_doc_inlay_hints_id)
    });

    // TODO: The inlay hints should actually instead return the id?
    new_inlay_hints
        .other_inlay_hints
        .push(InlineAnnotation::new(char_index, completion.to_string()));

    let id = new_inlay_hints.id;

    doc.set_inlay_hints(view_id, new_inlay_hints);

    Some((id.first_line, id.last_line))
}

pub fn remove_inlay_hint_by_id(
    cx: &mut Context,
    first_line: usize,
    last_line: usize,
) -> Option<()> {
    // let text = completion.to_string();
    let view_id = cx.editor.tree.focus;
    if !cx.editor.tree.contains(view_id) {
        return None;
    }
    let view = cx.editor.tree.get(view_id);
    let doc_id = cx.editor.tree.get(view_id).doc;
    let doc = cx.editor.documents.get_mut(&doc_id)?;

    let inlay_hints = doc.inlay_hints(view_id)?;
    let id = DocumentInlayHintsId {
        first_line,
        last_line,
    };

    if inlay_hints.id == id {
        let doc_text = doc.text();
        let len_lines = doc_text.len_lines();

        let view_height = view.inner_height();
        let first_visible_line =
            doc_text.char_to_line(doc.view_offset(view_id).anchor.min(doc_text.len_chars()));
        let first_line = first_visible_line.saturating_sub(view_height);
        let last_line = first_visible_line
            .saturating_add(view_height.saturating_mul(2))
            .min(len_lines);

        let new_doc_inlay_hints_id = DocumentInlayHintsId {
            first_line,
            last_line,
        };

        doc.set_inlay_hints(
            view_id,
            DocumentInlayHints::empty_with_id(new_doc_inlay_hints_id),
        );

        return Some(());
    }

    None
}

// "remove-inlay-hint",
pub fn remove_inlay_hint(cx: &mut Context, char_index: usize, _completion: SteelString) -> bool {
    // let text = completion.to_string();
    let view_id = cx.editor.tree.focus;
    if !cx.editor.tree.contains(view_id) {
        return false;
    }
    let doc_id = cx.editor.tree.get_mut(view_id).doc;
    let doc = match cx.editor.documents.get_mut(&doc_id) {
        Some(x) => x,
        None => return false,
    };

    let inlay_hints = match doc.inlay_hints(view_id) {
        Some(inlay_hints) => inlay_hints,
        None => return false,
    };
    let mut new_inlay_hints = inlay_hints.clone();
    new_inlay_hints
        .other_inlay_hints
        .retain(|x| x.char_idx != char_index);
    doc.set_inlay_hints(view_id, new_inlay_hints);
    true
}

pub fn insert_string(cx: &mut Context, string: SteelString) {
    let (view, doc) = current!(cx.editor);

    let indent = Tendril::from(string.as_str());
    let transaction = Transaction::insert(
        doc.text(),
        &doc.selection(view.id).clone().cursors(doc.text().slice(..)),
        indent,
    );
    doc.apply(&transaction, view.id);
}

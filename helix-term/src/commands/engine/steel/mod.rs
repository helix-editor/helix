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
        SearchConfig, SmartTabConfig, StatusLineElement, TerminalConfig, WhitespaceConfig,
        WhitespaceRender, WhitespaceRenderValue,
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
    parser::interner::InternedString,
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
    collections::{HashMap, HashSet},
    error::Error,
    io::Write,
    num::{NonZeroU8, NonZeroUsize},
    ops::{Deref, DerefMut},
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
    job::{self, Callback, Jobs},
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

static IDENTIFIERS_AVAILABLE_AFTER_BOOT: Lazy<Mutex<HashSet<InternedString>>> =
    Lazy::new(|| Mutex::new(HashSet::default()));

static EVENT_READER: OnceCell<EventReader> = OnceCell::new();

static CTX: &str = "*helix.cx*";
static CONFIG: &str = "*helix.config*";

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

fn identifier_available_at_startup(ident: &str) -> bool {
    let interned: InternedString = ident.into();

    IDENTIFIERS_AVAILABLE_AFTER_BOOT
        .lock()
        .unwrap()
        .contains(&interned)
}

fn setup() -> Engine {
    let engine = steel::steel_vm::engine::Engine::new();

    {
        let mut guard = IDENTIFIERS_AVAILABLE_AFTER_BOOT.lock().unwrap();
        guard.clear();

        for identifier in engine.readable_globals(0) {
            guard.insert(identifier);
        }
    }

    let controller = engine.get_thread_state_controller();
    let running = Arc::new(AtomicBool::new(false));

    let current_generation = load_generation();

    fn is_event_available() -> std::io::Result<bool> {
        #[cfg(windows)]
        {
            crossterm::event::poll(Duration::from_millis(1))
        }

        #[cfg(unix)]
        {
            EVENT_READER
                .get()
                .unwrap()
                .poll(Some(Duration::from_millis(0)), |_| true)
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

/// Calls the the given function `func` in the engine, with an updated `Context`,
/// with the given arguments `args`. This will update the CTX global in place
/// so that functions can reference it without needing it passed in to the function
/// directly.
pub fn call_with_context_and_args(
    engine: &mut Engine,
    ctx: &mut Context,
    func: SteelVal,
    args: &mut [SteelVal],
) -> Result<SteelVal, SteelErr> {
    engine
        .with_mut_reference(ctx)
        .consume_once(|engine, inner_args| {
            let context = inner_args.into_iter().next().unwrap();
            engine.update_value(CTX, context);
            engine.call_function_with_args_from_mut_slice(func, args)
        })
}

/// Calls the given function `func` in the engine, provided that
/// the current generation matches the existing generation. THis will update
/// the CTX global in place so that functions can reference it without
/// needing it passed in to the function directly.
pub fn generation_call_with_args(
    generation: usize,
    ctx: &mut Context,
    func: SteelVal,
    args: &mut [SteelVal],
) {
    if let Err(e) = enter_engine(|guard| {
        if !is_current_generation(generation) {
            return Ok(SteelVal::Void);
        }

        call_with_context_and_args(guard, ctx, func, args)
    }) {
        ctx.editor.set_error(e.to_string());
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
    module.register_fn("query-keymap", query_keybindings);

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

    let mut builtin_static_command_module = include_str!("static.scm").to_string();

    for command in TYPABLE_COMMAND_LIST {
        let func = |cx: &mut Context| {
            let mut cx = compositor::Context {
                editor: cx.editor,
                scroll: None,
                jobs: cx.jobs,
            };

            (command.fun)(&mut cx, Args::default(), PromptEvent::Validate)
        };

        module.register_fn_with_ctx(CTX, command.name, func);
    }

    // Register everything in the static command list as well
    // These just accept the context, no arguments. This is templated
    // because we want to be able to pick up any new commands that
    // are added to the built in static command list without needing
    // to update the docs in two places.
    for command in MappableCommand::STATIC_COMMAND_LIST {
        if let MappableCommand::Static { name, fun, doc } = command {
            module.register_fn_with_ctx(CTX, name, fun);

            let docstring = format_docstring(doc);

            builtin_static_command_module.push_str(&format!(
                r#"
(provide {})
;;@doc
{}
(define {} helix.static.{})
"#,
                name, docstring, name, name
            ));
        }
    }

    module
        .register_fn_with_ctx(CTX, "insert_char", insert_char)
        .register_fn_with_ctx(CTX, "insert_string", insert_string)
        .register_fn_with_ctx(CTX, "set-current-selection-object!", set_selection)
        .register_fn_with_ctx(CTX, "push-range-to-selection!", push_range_to_selection)
        .register_fn_with_ctx(
            CTX,
            "set-current-selection-primary-index!",
            set_selection_primary_index,
        )
        .register_fn_with_ctx(
            CTX,
            "remove-current-selection-range!",
            remove_selection_range,
        )
        .register_fn_with_ctx(CTX, "regex-selection", regex_selection)
        .register_fn_with_ctx(CTX, "replace-selection-with", replace_selection)
        .register_fn_with_ctx(
            CTX,
            "enqueue-expression-in-engine",
            run_expression_in_engine,
        )
        .register_fn_with_ctx(CTX, "get-current-line-character", current_line_character)
        .register_fn_with_ctx(CTX, "cx->current-file", current_path)
        .register_fn_with_ctx(CTX, "current_selection", current_selection)
        .register_fn_with_ctx(CTX, "current-selection->string", get_selection)
        .register_fn_with_ctx(CTX, "load-buffer!", load_buffer)
        .register_fn_with_ctx(CTX, "current-highlighted-text!", get_highlighted_text)
        .register_fn_with_ctx(CTX, "get-current-line-number", current_line_number)
        .register_fn_with_ctx(CTX, "get-current-column-number", current_column_number)
        .register_fn_with_ctx(CTX, "current-selection-object", current_selection)
        .register_fn_with_ctx(CTX, "get-helix-cwd", get_helix_cwd)
        .register_fn_with_ctx(CTX, "move-window-far-left", move_window_to_the_left)
        .register_fn_with_ctx(CTX, "move-window-far-right", move_window_to_the_right);

    module
        .register_fn("selection->primary-index", |sel: Selection| {
            sel.primary_index()
        })
        .register_fn("selection->primary-range", |sel: Selection| sel.primary())
        .register_fn("selection->ranges", |sel: Selection| sel.ranges().to_vec())
        .register_fn("range-anchor", |range: Range| range.anchor)
        .register_fn("range->from", |range: Range| range.from())
        .register_fn("range-head", |range: Range| range.head)
        .register_fn("range->to", |range: Range| range.to())
        .register_fn("range->span", |range: Range| (range.from(), range.to()))
        .register_fn("range", Range::new)
        .register_fn("range->selection", |range: Range| Selection::from(range))
        .register_fn("get-helix-scm-path", get_helix_scm_path)
        .register_fn("get-init-scm-path", get_init_scm_path);

    if generate_sources {
        generate_module("static.scm", &builtin_static_command_module);
        configure_lsp_builtins("static", &module);
    }

    engine.register_steel_module(
        "helix/static.scm".to_string(),
        builtin_static_command_module,
    );

    engine.register_module(module);
}

fn goto_line_impl(cx: &mut Context, mut line: usize, extend: bool) {
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);

    if line > text.len_lines() {
        line = text.len_lines();
    }

    let line = line.saturating_sub(1);

    let selection = doc.selection(view.id).clone().transform(|range| {
        let line_start = text.line_to_char(line);
        range.put_cursor(text, line_start, extend)
    });
    crate::commands::push_jump(view, doc);
    doc.set_selection(view.id, selection);
}

fn goto_column_impl(cx: &mut Context, char_index: usize, extend: bool) {
    let count = cx.count();
    let (view, doc) = current!(cx.editor);
    let text = doc.text().slice(..);
    let selection = doc.selection(view.id).clone().transform(|range| {
        let line = range.cursor_line(text);
        let line_start = text.line_to_char(line) + char_index;
        let line_end = helix_core::line_ending::line_end_char_index(&text, line);
        let pos = graphemes::nth_next_grapheme_boundary(text, line_start, count - 1).min(line_end);
        range.put_cursor(text, pos, extend)
    });
    crate::commands::push_jump(view, doc);
    doc.set_selection(view.id, selection);
}

fn load_typed_commands(engine: &mut Engine, generate_sources: bool) {
    let mut module = BuiltInModule::new("helix/core/typable".to_string());

    let mut builtin_typable_command_module = include_str!("commands.scm").to_string();

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

        module.register_fn_with_ctx(CTX, command.name, func);

        builtin_typable_command_module.push_str(&format!(
            r#"
(provide {})

;;@doc
{}
(define ({} . args)
    (helix.{} args))
"#,
            command.name,
            format_docstring(command.doc),
            command.name,
            command.name
        ));
    }

    module
        .register_fn_with_ctx(CTX, "goto-column", goto_column_impl)
        .register_fn_with_ctx(CTX, "goto-line", goto_line_impl);

    if generate_sources {
        generate_module("commands.scm", &builtin_typable_command_module);
        configure_lsp_builtins("typed", &module);
    }

    engine.register_steel_module(
        "helix/commands.scm".to_string(),
        builtin_typable_command_module,
    );

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
            default: Some(v),
            space: Some(v),
            nbsp: Some(v),
            nnbsp: Some(v),
            tab: Some(v),
            newline: Some(v),
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

    module
        .register_fn(
            "register-lsp-notification-handler",
            register_lsp_notification_callback,
        )
        .register_fn("register-lsp-call-handler", register_lsp_call_callback)
        .register_fn_with_ctx(CTX, "update-configuration!", |ctx: &mut Context| {
            ctx.editor
                .config_events
                .0
                .send(ConfigEvent::Change)
                .unwrap();
        })
        .register_fn_with_ctx(CTX, "get-config-option-value", get_option_value)
        .register_fn_with_ctx(
            CTX,
            "set-configuration-for-file!",
            set_configuration_for_file,
        )
        .register_fn(
            "get-language-config",
            HelixConfiguration::get_language_config,
        )
        .register_fn(
            "set-language-config!",
            HelixConfiguration::update_individual_language_config,
        )
        .register_fn_with_ctx(
            CONFIG,
            "get-lsp-config",
            HelixConfiguration::get_language_server_config,
        )
        .register_fn_with_ctx(
            CONFIG,
            "set-lsp-config!",
            HelixConfiguration::update_language_server_config,
        )
        .register_fn_with_ctx(
            CONFIG,
            "update-language-config!",
            HelixConfiguration::update_language_config,
        )
        .register_fn_with_ctx(
            CTX,
            "refresh-all-language-configs!",
            update_configuration_for_all_open_documents,
        )
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
        .register_fn_with_ctx(
            CONFIG,
            "register-file-picker",
            HelixConfiguration::file_picker,
        )
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
        .register_fn_with_ctx(CONFIG, "register-soft-wrap", HelixConfiguration::soft_wrap)
        .register_fn("sw-enable", sw_enable)
        .register_fn("sw-max-wrap", sw_max_wrap)
        .register_fn("sw-max-indent-retain", sw_max_indent_retain)
        .register_fn("sw-wrap-indicator", sw_wrap_indicator)
        .register_fn("sw-wrap-at-text-width", wrap_at_text_width);

    module
        .register_fn("raw-whitespace", WhitespaceConfig::default)
        .register_fn("register-whitespace", HelixConfiguration::whitespace)
        .register_fn_with_ctx(
            CONFIG,
            "register-whitespace",
            HelixConfiguration::whitespace,
        )
        .register_fn("ws-visible", ws_visible)
        .register_fn("ws-chars", ws_chars)
        .register_fn("ws-render", ws_render);

    module
        .register_fn("raw-indent-guides", IndentGuidesConfig::default)
        .register_fn("register-indent-guides", HelixConfiguration::indent_guides)
        .register_fn_with_ctx(
            CONFIG,
            "register-indent-guides",
            HelixConfiguration::indent_guides,
        )
        .register_fn("ig-render", ig_render)
        .register_fn("ig-character", ig_character)
        .register_fn("ig-skip-levels", ig_skip_levels);

    module
        .register_fn_with_ctx(CONFIG, "scrolloff", HelixConfiguration::scrolloff)
        .register_fn_with_ctx(CONFIG, "scroll_lines", HelixConfiguration::scroll_lines)
        .register_fn_with_ctx(CONFIG, "mouse", HelixConfiguration::mouse)
        .register_fn_with_ctx(CONFIG, "shell", HelixConfiguration::shell)
        .register_fn_with_ctx(
            CONFIG,
            "jump-label-alphabet",
            HelixConfiguration::jump_label_alphabet,
        )
        .register_fn_with_ctx(CONFIG, "line-number", HelixConfiguration::line_number)
        .register_fn_with_ctx(CONFIG, "cursorline", HelixConfiguration::cursorline)
        .register_fn_with_ctx(CONFIG, "cursorcolumn", HelixConfiguration::cursorcolumn)
        .register_fn_with_ctx(
            CONFIG,
            "middle-click-paste",
            HelixConfiguration::middle_click_paste,
        )
        .register_fn_with_ctx(CONFIG, "auto-pairs", HelixConfiguration::auto_pairs)
        .register_fn_with_ctx(
            CTX,
            "#%editor-auto-pairs",
            |ctx: &mut Context, auto_pairs: AutoPairConfig| {
                ctx.editor.auto_pairs = auto_pairs.into();
            },
        )
        // Specific constructors for the auto pairs configuration
        .register_fn("auto-pairs-default", |enabled: bool| {
            AutoPairConfig::Enable(enabled)
        })
        .register_fn("auto-pairs-map", |map: HashMap<char, char>| {
            AutoPairConfig::Pairs(map)
        })
        // TODO: Finish this up
        .register_fn("auto-save-default", AutoSave::default)
        .register_fn_with_ctx(
            CONFIG,
            "auto-save-after-delay-enable",
            HelixConfiguration::auto_save_after_delay_enable,
        )
        .register_fn_with_ctx(
            CONFIG,
            "inline-diagnostics-cursor-line-enable",
            HelixConfiguration::inline_diagnostics_cursor_line_enable,
        )
        .register_fn_with_ctx(
            CONFIG,
            "inline-diagnostics-other-lines-enable",
            HelixConfiguration::inline_diagnostics_other_lines_enable,
        )
        .register_fn_with_ctx(
            CONFIG,
            "inline-diagnostics-end-of-line-enable",
            HelixConfiguration::inline_diagnostics_end_of_line_enable,
        )
        .register_fn_with_ctx(
            CONFIG,
            "inline-diagnostics-min-diagnostics-width",
            HelixConfiguration::inline_diagnostics_min_diagnostic_width,
        )
        .register_fn_with_ctx(
            CONFIG,
            "inline-diagnostics-prefix-len",
            HelixConfiguration::inline_diagnostics_prefix_len,
        )
        .register_fn_with_ctx(
            CONFIG,
            "inline-diagnostics-max-wrap",
            HelixConfiguration::inline_diagnostics_max_wrap,
        )
        .register_fn_with_ctx(
            CONFIG,
            "inline-diagnostics-max-diagnostics",
            HelixConfiguration::inline_diagnostics_max_diagnostics,
        )
        .register_fn_with_ctx(
            CONFIG,
            "auto-completion",
            HelixConfiguration::auto_completion,
        )
        .register_fn_with_ctx(CONFIG, "auto-format", HelixConfiguration::auto_format)
        .register_fn_with_ctx(CONFIG, "auto-save", HelixConfiguration::auto_save)
        .register_fn_with_ctx(CONFIG, "text-width", HelixConfiguration::text_width)
        .register_fn_with_ctx(CONFIG, "idle-timeout", HelixConfiguration::idle_timeout)
        .register_fn_with_ctx(
            CONFIG,
            "completion-timeout",
            HelixConfiguration::completion_timeout,
        )
        .register_fn_with_ctx(
            CONFIG,
            "preview-completion-insert",
            HelixConfiguration::preview_completion_insert,
        )
        .register_fn_with_ctx(
            CONFIG,
            "completion-trigger-len",
            HelixConfiguration::completion_trigger_len,
        )
        .register_fn_with_ctx(
            CONFIG,
            "completion-replace",
            HelixConfiguration::completion_replace,
        )
        .register_fn_with_ctx(CONFIG, "auto-info", HelixConfiguration::auto_info)
        .register_fn_with_ctx(
            CONFIG,
            "#%raw-cursor-shape",
            HelixConfiguration::cursor_shape,
        )
        .register_fn("true-color", HelixConfiguration::true_color)
        .register_fn_with_ctx(
            CONFIG,
            "insert-final-newline",
            HelixConfiguration::insert_final_newline,
        )
        .register_fn_with_ctx(CONFIG, "color-modes", HelixConfiguration::color_modes)
        .register_fn_with_ctx(CONFIG, "gutters", HelixConfiguration::gutters)
        .register_fn_with_ctx(CONFIG, "statusline", HelixConfiguration::statusline)
        .register_fn_with_ctx(CONFIG, "undercurl", HelixConfiguration::undercurl)
        .register_fn_with_ctx(CONFIG, "search", HelixConfiguration::search)
        .register_fn_with_ctx(CONFIG, "lsp", HelixConfiguration::lsp)
        .register_fn_with_ctx(CONFIG, "terminal", HelixConfiguration::terminal)
        .register_fn_with_ctx(CONFIG, "rulers", HelixConfiguration::rulers)
        .register_fn_with_ctx(CONFIG, "bufferline", HelixConfiguration::bufferline)
        .register_fn_with_ctx(
            CONFIG,
            "workspace-lsp-roots",
            HelixConfiguration::workspace_lsp_roots,
        )
        .register_fn_with_ctx(
            CONFIG,
            "default-line-ending",
            HelixConfiguration::default_line_ending,
        )
        .register_fn_with_ctx(CONFIG, "smart-tab", HelixConfiguration::smart_tab)
        .register_fn_with_ctx(
            CONFIG,
            "rainbow-brackets",
            HelixConfiguration::rainbow_brackets,
        );

    // Keybinding stuff
    module
        .register_fn_with_ctx(CONFIG, "keybindings", HelixConfiguration::keybindings)
        .register_fn_with_ctx(
            CONFIG,
            "get-keybindings",
            HelixConfiguration::get_keybindings,
        )
        .register_fn_with_ctx(
            CONFIG,
            "set-keybindings!",
            HelixConfiguration::set_keybindings,
        )
        .register_fn_with_ctx(CONFIG, "set-option!", dynamic_set_option);

    let builtin_configuration_module = include_str!("configuration.scm").to_string();

    if generate_sources {
        generate_module("configuration.scm", &builtin_configuration_module);
        configure_lsp_builtins("configuration", &module);
    }

    engine.register_steel_module(
        "helix/configuration.scm".to_string(),
        builtin_configuration_module,
    );

    engine.register_module(module);
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
        generate_module("keymaps.scm", keymap);
    }

    engine.register_steel_module("helix/keymaps.scm".to_string(), keymap.to_string());
}

fn generate_module(filename: &str, module: &str) {
    if let Some(mut target_directory) = alternative_runtime_search_path() {
        if !target_directory.exists() {
            std::fs::create_dir_all(&target_directory).unwrap();
        }

        target_directory.push(filename);

        std::fs::write(target_directory, module).unwrap();
    }
}

fn load_high_level_theme_api(engine: &mut Engine, generate_sources: bool) {
    let theme = include_str!("themes.scm");

    if generate_sources {
        generate_module("themes.scm", theme)
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

    let builtin_editor_command_module = include_str!("editor.scm").to_string();

    module.register_fn("register-hook", register_hook);

    module
        .register_fn("Action/Load", || Action::Load)
        .register_fn("Action/Replace", || Action::Replace)
        .register_fn("Action/HorizontalSplit", || Action::HorizontalSplit)
        .register_fn("Action/VerticalSplit", || Action::VerticalSplit);

    module
        .register_fn_with_ctx(CTX, "editor-focus", cx_current_focus)
        .register_fn_with_ctx(CTX, "editor-mode", cx_get_mode)
        .register_fn_with_ctx(CTX, "cx->themes", get_themes)
        .register_fn_with_ctx(CTX, "editor-count", |cx: &mut Context| {
            cx.editor.count.map(|x| x.get()).unwrap_or(1)
        })
        .register_fn_with_ctx(CTX, "themes->list", get_themes)
        .register_fn_with_ctx(CTX, "editor-all-documents", cx_editor_all_documents)
        .register_fn_with_ctx(CTX, "cx->cursor", |cx: &mut Context| cx.editor.cursor())
        .register_fn_with_ctx(CTX, "current-cursor", |cx: &mut Context| cx.editor.cursor())
        .register_fn_with_ctx(CTX, "editor-focused-buffer-area", current_buffer_area)
        .register_fn_with_ctx(CTX, "editor-focused-buffer-area", current_buffer_area)
        .register_fn_with_ctx(CTX, "selected-register!", |cx: &mut Context| {
            cx.editor
                .selected_register
                .unwrap_or(cx.editor.config().default_yank_register)
        })
        .register_fn_with_ctx(CTX, "editor->doc-id", cx_get_document_id)
        .register_fn_with_ctx(CTX, "editor-switch!", cx_switch)
        .register_fn_with_ctx(
            CTX,
            "editor-set-focus!",
            |cx: &mut Context, view_id: ViewId| cx.editor.focus(view_id),
        )
        .register_fn_with_ctx(CTX, "editor-set-mode!", cx_set_mode)
        .register_fn_with_ctx(CTX, "editor-doc-in-view?", cx_is_document_in_view)
        .register_fn_with_ctx(CTX, "set-scratch-buffer-name!", set_scratch_buffer_name)
        // Get the last saved time of the document
        .register_fn_with_ctx(
            CTX,
            "editor-document-last-saved",
            |cx: &mut Context, doc: DocumentId| -> Option<SystemTime> {
                cx.editor.documents.get(&doc).map(|x| x.last_saved_time())
            },
        )
        .register_fn_with_ctx(CTX, "editor-document->language", cx_get_document_language)
        .register_fn_with_ctx(
            CTX,
            "editor-document-dirty?",
            |cx: &mut Context, doc: DocumentId| -> Option<bool> {
                cx.editor.documents.get(&doc).map(|x| x.is_modified())
            },
        )
        .register_fn_with_ctx(
            CTX,
            "editor-document-reload",
            |cx: &mut Context, doc: DocumentId| -> anyhow::Result<()> {
                for (view, _) in cx.editor.tree.views_mut() {
                    if let Some(x) = cx.editor.documents.get_mut(&doc) {
                        x.reload(view, &cx.editor.diff_providers)?;
                    }
                }
                Ok(())
            },
        )
        .register_fn_with_ctx(CTX, "set-buffer-uri!", set_buffer_uri)
        .register_fn_with_ctx(CTX, "editor-doc-exists?", cx_document_exists)
        .register_fn_with_ctx(CTX, "editor-switch-action!", cx_switch_action)
        .register_fn_with_ctx(
            CTX,
            "set-register!",
            |cx: &mut Context, name: char, value: Vec<String>| {
                cx.editor.registers.write(name, value)
            },
        )
        .register_fn_with_ctx(CTX, "editor->text", document_id_to_text)
        .register_fn_with_ctx(CTX, "editor-document->path", document_path)
        .register_fn_with_ctx(CTX, "register->value", cx_register_value)
        .register_fn_with_ctx(
            CTX,
            "set-editor-clip-right!",
            |cx: &mut Context, right: u16| {
                cx.editor.editor_clipping.right = Some(right);
            },
        )
        .register_fn_with_ctx(
            CTX,
            "set-editor-clip-left!",
            |cx: &mut Context, left: u16| {
                cx.editor.editor_clipping.left = Some(left);
            },
        )
        .register_fn_with_ctx(CTX, "set-editor-clip-top!", |cx: &mut Context, top: u16| {
            cx.editor.editor_clipping.top = Some(top);
        })
        .register_fn_with_ctx(
            CTX,
            "set-editor-clip-bottom!",
            |cx: &mut Context, bottom: u16| {
                cx.editor.editor_clipping.bottom = Some(bottom);
            },
        )
        .register_fn_with_ctx(CTX, "string->editor-mode", string_to_mode)
        .register_fn_with_ctx(
            CTX,
            "set-editor-count!",
            |ctx: &mut Context, count: usize| {
                ctx.editor.count = NonZeroUsize::new(count);
                ctx.count = ctx.editor.count;
            },
        );

    if generate_sources {
        generate_module("editor.scm", &builtin_editor_command_module);
        configure_lsp_builtins("editor", &module);
    }

    engine.register_steel_module(
        "helix/editor.scm".to_string(),
        builtin_editor_command_module,
    );

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

            match enter_engine(|guard| {
                {
                    // Install the interrupt handler, in the event this thing
                    // is blocking for too long.
                    with_interrupt_handler(|| {
                        guard.with_mut_reference::<Context, Context>(cx).consume(
                            move |engine, arguments| {
                                let context = arguments.into_iter().next().unwrap();
                                engine.update_value(CTX, context);
                                engine
                                    .call_function_by_name_with_args_from_mut_slice(name, &mut args)
                            },
                        )
                    })
                }
            }) {
                Ok(res) => match &res {
                    SteelVal::Void => {}
                    SteelVal::StringV(s) => {
                        cx.editor.set_status(s.as_str().to_owned());
                    }
                    _ => {
                        cx.editor.set_status(res.to_string());
                    }
                },
                Err(e) => {
                    cx.editor.set_error(e.to_string());
                }
            };
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

            // Handle ties for built in implementations:
            if crate::commands::typed::TYPABLE_COMMAND_MAP.contains_key(command) {
                let should_prefer_builtin = identifier_available_at_startup(command);
                if should_prefer_builtin {
                    return false;
                }
            }

            // We're finalizing the event - we actually want to call the function
            if event == PromptEvent::Validate {
                if let Err(e) = enter_engine(|guard| {
                    let args = args
                        .iter()
                        .map(|x| x.into_steelval().unwrap())
                        .collect::<Vec<_>>();

                    let mut ctx = with_context_guard(cx);

                    // Install interrupt handler here during the duration
                    // of the function call
                    match with_interrupt_handler(|| {
                        guard.with_mut_reference(&mut ctx.ctx).consume_once(
                            move |engine, arguments| {
                                let context = arguments.into_iter().next().unwrap();
                                engine.update_value(CTX, context);
                                engine.call_function_by_name_with_args(command, args)
                            },
                        )
                    }) {
                        Ok(res) => {
                            match &res {
                                SteelVal::Void => {}
                                SteelVal::StringV(s) => {
                                    ctx.editor.set_status(s.as_str().to_owned());
                                }
                                _ => {
                                    ctx.editor.set_status(res.to_string());
                                }
                            }

                            Ok(res)
                        }
                        Err(e) => Err(e),
                    }
                }) {
                    let mut ctx = with_context_guard(cx);
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
        let mut ctx = make_ephemeral_context(cx);

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
                        .consume_once(move |engine, arguments| {
                            let context = arguments.into_iter().next().unwrap();
                            engine.update_value(CTX, context);

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

        patch_callbacks(&mut ctx);

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

fn patch_callbacks(ctx: &mut Context<'_>) {
    for callback in std::mem::take(&mut ctx.callback) {
        let callback = async move {
            let call: Box<LocalJobCallback> = Box::new(
                move |editor: &mut Editor, compositor: &mut Compositor, jobs| {
                    callback(
                        compositor,
                        &mut compositor::Context {
                            editor,
                            scroll: None,
                            jobs,
                        },
                    )
                },
            );
            Ok(call)
        };

        ctx.jobs.local_callback(callback);
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
                let res = editor.keymaps.get_with_map(&keymap.0, mode, event);

                if let KeymapResult::NotFound = res {
                    return None;
                }

                return Some(res);
            }
        }

        let map = get_extension_keymap();

        if let Some(keymap) = map.get_doc_id(document_id_to_usize(doc_id)) {
            let res = editor.keymaps.get_with_map(&keymap.0, mode, event);

            if let KeymapResult::NotFound = res {
                return None;
            }

            return Some(res);
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

pub fn query_keybindings(
    map: &mut EmbeddedKeyMap,
    mode: SteelString,
    keybindings: Vec<String>,
) -> anyhow::Result<SteelVal> {
    let mode = match mode.as_str() {
        "normal" => Mode::Normal,
        "select" => Mode::Select,
        "insert" => Mode::Insert,
        _ => anyhow::bail!("unknown mode: {}", mode),
    };

    let keymap = map.0.get(&mode);
    let bindings = keybindings
        .into_iter()
        .map(|x| KeyEvent::from_str(&x))
        .collect::<anyhow::Result<Vec<_>>>()?;

    if let Some(keymap) = keymap {
        let value = keymap.search(&bindings);
        match value {
            Some(KeyTrie::MappableCommand(k)) => Ok(SteelVal::StringV(k.name().into())),
            _ => Ok(SteelVal::BoolV(false)),
        }
    } else {
        Ok(SteelVal::BoolV(false))
    }
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

    fn get_language_server_config(&self, lsp: SteelString) -> Option<SteelVal> {
        let loader = (*(*self.language_configuration.load())).clone();
        let lsp_configs = loader.language_server_configs();
        let individual_config = lsp_configs.get(lsp.as_str())?;
        let mut json = serde_json::json!(individual_config);

        if let Some(config) = individual_config.config.clone() {
            json["config"] = config;
        }

        let hash = SteelVal::try_from(json);

        hash.ok()
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
                let mut serialized = serde_json::Value::try_from(config_json.clone())?;

                filter_null_values(&mut serialized);

                config.config = Some(serialized);
            }

            if let Some(timeout) = map.get("timeout") {
                config.timeout = match timeout {
                    SteelVal::IntV(i) => *i as u64,
                    SteelVal::NumV(n) if n.fract() == 0.0 => n.round() as u64,
                    _ => anyhow::bail!("Unable to convert timeout to integer, found: {}", timeout),
                };
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
        (*self.configuration.load_full().clone()).clone()
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
        EmbeddedKeyMap(self.configuration.load_full().keys.clone())
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

    fn statusline(&self, config: HashMap<String, SteelVal>) -> anyhow::Result<()> {
        let mut app_config = self.load_config();

        fn steel_to_elements(val: &SteelVal) -> anyhow::Result<StatusLineElement> {
            if let SteelVal::StringV(s) = val {
                let value = match s.as_str() {
                    "mode" => StatusLineElement::Mode,
                    "spinner" => StatusLineElement::Spinner,
                    "file-base-name" => StatusLineElement::FileBaseName,
                    "file-name" => StatusLineElement::FileName,
                    "file-absolute-path" => StatusLineElement::FileAbsolutePath,
                    "file-modification-indicator" => StatusLineElement::FileModificationIndicator,
                    "read-only-indicator" => StatusLineElement::ReadOnlyIndicator,
                    "file-encoding" => StatusLineElement::FileEncoding,
                    "file-line-ending" => StatusLineElement::FileLineEnding,
                    "file-indent-style" => StatusLineElement::FileIndentStyle,
                    "file-type" => StatusLineElement::FileType,
                    "diagnostics" => StatusLineElement::Diagnostics,
                    "workspace-diagnostics" => StatusLineElement::WorkspaceDiagnostics,
                    "selections" => StatusLineElement::Selections,
                    "primary-selection-length" => StatusLineElement::PrimarySelectionLength,
                    "position" => StatusLineElement::Position,
                    "separator" => StatusLineElement::Separator,
                    "position-percentage" => StatusLineElement::PositionPercentage,
                    "total-line-numbers" => StatusLineElement::TotalLineNumbers,
                    "spacer" => StatusLineElement::Spacer,
                    "version-control" => StatusLineElement::VersionControl,
                    "register" => StatusLineElement::Register,
                    "current-working-directory" => StatusLineElement::CurrentWorkingDirectory,
                    _ => anyhow::bail!("Unknown status line element: {}", s),
                };

                Ok(value)
            } else {
                anyhow::bail!("Cannot convert value to status line element: {}", val)
            }
        }

        fn steel_list_to_elements(val: &SteelVal) -> anyhow::Result<Vec<StatusLineElement>> {
            if let SteelVal::ListV(l) = val {
                l.iter().map(steel_to_elements).collect()
            } else {
                anyhow::bail!(
                    "Cannot convert value to vec of status line element: {}",
                    val
                )
            }
        }

        fn steel_to_severity(val: &SteelVal) -> anyhow::Result<Severity> {
            if let SteelVal::StringV(s) = val {
                let value = match s.as_str() {
                    "hint" => Severity::Hint,
                    "info" => Severity::Info,
                    "warning" => Severity::Warning,
                    "error" => Severity::Error,
                    _ => anyhow::bail!("Unknown severity label: {}", s),
                };

                Ok(value)
            } else {
                anyhow::bail!("Cannot convert value to severity: {}", val)
            }
        }

        fn steel_list_to_severity_vec(val: &SteelVal) -> anyhow::Result<Vec<Severity>> {
            if let SteelVal::ListV(l) = val {
                l.iter().map(steel_to_severity).collect()
            } else {
                anyhow::bail!(
                    "Cannot convert value to vec of status line element: {}",
                    val
                )
            }
        }

        if let Some(left) = config.get("left") {
            app_config.editor.statusline.left = steel_list_to_elements(left)?;
        }

        if let Some(center) = config.get("center") {
            app_config.editor.statusline.center = steel_list_to_elements(center)?;
        }

        if let Some(right) = config.get("right") {
            app_config.editor.statusline.right = steel_list_to_elements(right)?;
        }

        if let Some(separator) = config.get("separator") {
            app_config.editor.statusline.separator = String::from_steelval(separator)?;
        }

        if let Some(normal_mode) = config.get("mode-normal") {
            if let SteelVal::StringV(s) = normal_mode {
                app_config.editor.statusline.mode.normal = s.as_str().to_owned();
            } else {
                anyhow::bail!("mode normal expects a string, found: {}", normal_mode);
            }
        }

        if let Some(insert_mode) = config.get("mode-insert") {
            if let SteelVal::StringV(s) = insert_mode {
                app_config.editor.statusline.mode.insert = s.as_str().to_owned();
            } else {
                anyhow::bail!("mode insert expects a string, found: {}", insert_mode);
            }
        }

        if let Some(select_mode) = config.get("mode-select") {
            if let SteelVal::StringV(s) = select_mode {
                app_config.editor.statusline.mode.select = s.as_str().to_owned();
            } else {
                anyhow::bail!("mode normal expects a string, found: {}", select_mode);
            }
        }

        if let Some(diagnostics) = config.get("diagnostics") {
            app_config.editor.statusline.diagnostics = steel_list_to_severity_vec(diagnostics)?;
        }

        if let Some(diagnostics) = config.get("workspace-diagnostics") {
            app_config.editor.statusline.workspace_diagnostics =
                steel_list_to_severity_vec(diagnostics)?;
        }

        self.store_config(app_config);

        Ok(())
    }

    fn undercurl(&self, undercurl: bool) {
        let mut app_config = self.load_config();
        app_config.editor.undercurl = undercurl;
        self.store_config(app_config);
    }

    fn search(&self, smart_case: bool, wrap_around: bool) {
        let mut app_config = self.load_config();
        app_config.editor.search = SearchConfig {
            smart_case,
            wrap_around,
        };
        self.store_config(app_config);
    }

    fn lsp(&self, config: HashMap<String, SteelVal>) -> anyhow::Result<()> {
        let mut app_config = self.load_config();

        if let Some(enabled) = config.get("enable") {
            app_config.editor.lsp.enable = bool::from_steelval(enabled)?;
        }

        if let Some(display) = config.get("display-progress-messages") {
            app_config.editor.lsp.display_progress_messages = bool::from_steelval(display)?;
        }

        if let Some(display) = config.get("display-messages") {
            app_config.editor.lsp.display_messages = bool::from_steelval(display)?;
        }

        if let Some(auto) = config.get("auto-signature-help") {
            app_config.editor.lsp.auto_signature_help = bool::from_steelval(auto)?;
        }

        if let Some(display) = config.get("display-signature-help") {
            app_config.editor.lsp.display_signature_help_docs = bool::from_steelval(display)?;
        }

        if let Some(display) = config.get("display-inlay-hints") {
            app_config.editor.lsp.display_inlay_hints = bool::from_steelval(display)?;
        }

        if let Some(limit) = config.get("inlay-hints-length-limit") {
            let n = NonZeroU8::new(u8::from_steelval(limit)?);

            if let Some(n) = n {
                app_config.editor.lsp.inlay_hints_length_limit = Some(n)
            } else {
                anyhow::bail!("inlay hints length limit provided was zero")
            }
        }

        if let Some(display) = config.get("display-color-swatches") {
            app_config.editor.lsp.display_color_swatches = bool::from_steelval(display)?;
        }

        if let Some(snippets) = config.get("snippets") {
            app_config.editor.lsp.snippets = bool::from_steelval(snippets)?;
        }

        if let Some(goto) = config.get("goto-reference-include-declaration") {
            app_config.editor.lsp.goto_reference_include_declaration = bool::from_steelval(goto)?;
        }

        self.store_config(app_config);
        Ok(())
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
    let now = std::time::Instant::now();
    install_event_reader(event_reader);

    // Hack:
    // This might be fussed with, and under re initialization we want
    // to reset this back to what it was before.
    cx.editor.editor_clipping = ClippingConfiguration::default();

    log::info!("Loading init.scm...");

    let helix_module_path = helix_module_file();
    let helix_init_path = steel_init_file();

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
            log::info!("Loading helix.scm from context: {:?}", helix_init_path);
            let res = guard.run_with_reference_from_path(
                cx,
                CTX,
                &format!(r#"(require {:?})"#, helix_module_path.to_str().unwrap()),
                helix_init_path,
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
                CTX,
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

    patch_callbacks(cx);

    log::info!("Steel init time: {:?}", now.elapsed());
}

impl Custom for PromptEvent {}

impl CustomReference for Context<'_> {}

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

fn register_hook(event_kind: String, callback_fn: SteelVal) -> steel::UnRecoverableResult {
    let rooted = callback_fn.as_rooted();
    let generation = load_generation();

    match event_kind.as_str() {
        "on-mode-switch" => register_on_mode_switch(generation, rooted),
        "post-insert-char" => register_post_insert_char(generation, rooted),
        // Register hook - on save?
        "post-command" => register_post_command(generation, rooted),
        "document-focus-lost" => register_document_focus_lost(generation, rooted),
        "selection-did-change" => register_selection_did_change(generation, rooted),
        "document-opened" => register_document_opened(generation, rooted),
        "document-saved" => register_document_saved(generation, rooted),
        _ => steelerr!(Generic => "Unable to register hook: Unknown event type: {}", event_kind)
            .into(),
    }
}

fn construct_callback<const N: usize>(
    generation: usize,
    func: SteelVal,
    mut args: [SteelVal; N],
) -> impl FnOnce(&mut Editor, &mut Compositor, &mut Jobs) + Send + 'static {
    move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
        let mut compositor_context = compositor::Context {
            editor,
            jobs,
            scroll: None,
        };
        let mut ctx = with_context_guard(&mut compositor_context);

        enter_engine(|guard| {
            if !is_current_generation(generation) {
                return;
            }

            if let Err(e) = call_with_context_and_args(guard, &mut ctx, func, &mut args) {
                present_error_inside_engine_context(&mut ctx, guard, e);
            }
        });
    }
}

fn register_document_saved(
    generation: usize,
    rooted: RootedSteelVal,
) -> steel::UnRecoverableResult {
    // TODO: Share this code with the above since most of it is
    // exactly the same
    register_hook!(move |event: &mut DocumentSaved<'_>| {
        let cloned_func = rooted.value().clone();
        let doc_id = event.doc;
        let callback =
            construct_callback(generation, cloned_func, [doc_id.into_steelval().unwrap()]);
        job::dispatch_blocking_jobs(callback);

        Ok(())
    });
    Ok(SteelVal::Void).into()
}

fn register_document_opened(
    generation: usize,
    rooted: RootedSteelVal,
) -> steel::UnRecoverableResult {
    // TODO: Share this code with the above since most of it is
    // exactly the same
    register_hook!(move |event: &mut DocumentDidOpen<'_>| {
        let cloned_func = rooted.value().clone();
        let doc_id = event.doc;
        let callback =
            construct_callback(generation, cloned_func, [doc_id.into_steelval().unwrap()]);
        job::dispatch_blocking_jobs(callback);

        Ok(())
    });
    Ok(SteelVal::Void).into()
}

fn register_selection_did_change(
    generation: usize,
    rooted: RootedSteelVal,
) -> steel::UnRecoverableResult {
    // TODO: Pass the information from the event in here - the doc id
    // is probably the most helpful so that way we can look the document up
    // and act accordingly?
    register_hook!(move |event: &mut SelectionDidChange<'_>| {
        let cloned_func = rooted.value().clone();
        let view_id = event.view;
        let callback =
            construct_callback(generation, cloned_func, [view_id.into_steelval().unwrap()]);
        job::dispatch_blocking_jobs(callback);

        Ok(())
    });
    Ok(SteelVal::Void).into()
}

fn register_document_focus_lost(
    generation: usize,
    rooted: RootedSteelVal,
) -> steel::UnRecoverableResult {
    // TODO: Pass the information from the event in here - the doc id
    // is probably the most helpful so that way we can look the document up
    // and act accordingly?
    register_hook!(move |event: &mut DocumentFocusLost<'_>| {
        let cloned_func = rooted.value().clone();
        let doc_id = event.doc;
        let callback =
            construct_callback(generation, cloned_func, [doc_id.into_steelval().unwrap()]);
        job::dispatch_blocking_jobs(callback);

        Ok(())
    });
    Ok(SteelVal::Void).into()
}

fn register_post_command(generation: usize, rooted: RootedSteelVal) -> steel::UnRecoverableResult {
    register_hook!(move |event: &mut PostCommand<'_, '_>| {
        generation_call_with_args(
            generation,
            event.cx,
            rooted.value().clone(),
            &mut [event.command.name().into_steelval().unwrap()],
        );
        Ok(())
    });
    Ok(SteelVal::Void).into()
}

fn register_post_insert_char(
    generation: usize,
    rooted: RootedSteelVal,
) -> steel::UnRecoverableResult {
    register_hook!(move |event: &mut PostInsertChar<'_, '_>| {
        generation_call_with_args(
            generation,
            event.cx,
            rooted.value().clone(),
            &mut [event.c.into()],
        );

        Ok(())
    });

    Ok(SteelVal::Void).into()
}

fn register_on_mode_switch(
    generation: usize,
    rooted: RootedSteelVal,
) -> steel::UnRecoverableResult {
    register_hook!(move |event: &mut OnModeSwitch<'_, '_>| {
        let minimized_event = OnModeSwitchEvent {
            old_mode: event.old_mode,
            new_mode: event.new_mode,
        };

        generation_call_with_args(
            generation,
            event.cx,
            rooted.value().clone(),
            &mut [minimized_event.into_steelval().unwrap()],
        );

        Ok(())
    });

    Ok(SteelVal::Void).into()
}

fn configure_lsp_globals() {
    use std::fmt::Write;
    let mut path = steel_lsp_home_dir();
    path.push("_helix-global-builtins.scm");

    let mut output = String::new();

    let names = &[
        CTX,
        CONFIG,
        "*helix.id*",
        "register-hook!",
        "log::info!",
        "log::debug!",
        "log::warn!",
        "log::error!",
        "log::info",
        "log::debug",
        "log::warn",
        "log::error",
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
    let builtin_misc_module = include_str!("misc.scm").to_string();

    module
        .register_fn_with_ctx(CTX, "hx.cx->pos", cx_pos_within_text)
        .register_fn_with_ctx(CTX, "cursor-position", cx_pos_within_text)
        .register_fn("mode-switch-old", OnModeSwitchEvent::get_old_mode)
        .register_fn("mode-switch-new", OnModeSwitchEvent::get_new_mode)
        .register_fn_with_ctx(CTX, "get-active-lsp-clients", get_active_lsp_clients)
        .register_fn("lsp-client-initialized?", is_lsp_client_initialized)
        .register_fn("lsp-client-name", lsp_client_name)
        .register_fn("lsp-client-offset-encoding", lsp_client_offset_encoding)
        .register_fn_with_ctx(CTX, "hx.custom-insert-newline", custom_insert_newline)
        .register_fn_with_ctx(CTX, "insert-newline-hook", custom_insert_newline)
        .register_fn_with_ctx(CTX, "push-component!", push_component)
        .register_fn_with_ctx(CTX, "pop-last-component!", pop_last_component_by_name)
        .register_fn_with_ctx(
            CTX,
            "pop-last-component-by-name!",
            pop_last_component_by_name,
        )
        .register_fn_with_ctx(CTX, "on-key-callback", enqueue_on_next_key)
        .register_fn_with_ctx(CTX, "trigger-on-key-callback", trigger_callback)
        .register_fn_with_ctx(CTX, "enqueue-thread-local-callback", enqueue_command)
        .register_fn_with_ctx(CTX, "set-status!", set_status)
        .register_fn_with_ctx(CTX, "set-warning!", set_warning)
        .register_fn_with_ctx(CTX, "set-error!", set_error)
        .register_fn_with_ctx(CTX, "send-lsp-command", send_arbitrary_lsp_command)
        .register_fn_with_ctx(
            CTX,
            "send-lsp-notification",
            send_arbitrary_lsp_notification,
        )
        .register_fn_with_ctx(CTX, "lsp-reply-ok", lsp_reply_ok)
        .register_fn("acquire-context-lock", acquire_context_lock)
        .register_fn_with_ctx(
            CTX,
            "enqueue-thread-local-callback-with-delay",
            enqueue_command_with_delay,
        )
        .register_fn_with_ctx(CTX, "helix-await-callback", await_value)
        .register_fn_with_ctx(CTX, "await-callback", await_value)
        .register_fn_with_ctx(CTX, "add-inlay-hint", add_inlay_hint)
        .register_fn_with_ctx(CTX, "remove-inlay-hint", remove_inlay_hint)
        .register_fn_with_ctx(CTX, "remove-inlay-hint-by-id", remove_inlay_hint_by_id);

    if generate_sources {
        generate_module("misc.scm", &builtin_misc_module);
        configure_lsp_builtins("misc", &module);
    }

    engine.register_steel_module("helix/misc.scm".to_string(), builtin_misc_module);

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
    let ext_api = include_str!("ext.scm");
    if generate_sources {
        generate_module("ext.scm", &ext_api);
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

pub fn configure_builtin_sources(engine: &mut Engine, generate_sources: bool) {
    load_editor_api(engine, generate_sources);
    load_theme_api(engine, generate_sources);
    load_configuration_api(engine, generate_sources);
    load_typed_commands(engine, generate_sources);
    load_static_commands(engine, generate_sources);
    load_keymap_api(engine, generate_sources);
    load_rope_api(engine, generate_sources);
    load_misc_api(engine, generate_sources);
    load_component_api(engine, generate_sources);

    // This depends on the components and theme api, so should
    // be loaded after.
    load_high_level_theme_api(engine, generate_sources);
    load_high_level_keymap_api(engine, generate_sources);
    load_ext_api(engine, generate_sources);

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
        let mut compositor_context = compositor::Context {
            editor,
            jobs,
            scroll: None,
        };

        let mut ctx = with_context_guard(&mut compositor_context);

        let cloned_func = rooted.value();
        let cloned_place = rooted_place.as_ref().map(|x| x.value());

        enter_engine(|guard| {
            if let Err(e) = guard
                .with_mut_reference::<Context, Context>(&mut ctx)
                // Block until the other thread is finished in its critical
                // section...
                .consume(move |engine, args| {
                    let context = args.into_iter().next().unwrap();
                    engine.update_value(CTX, context);

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
                                s.set_index(3, e.clone().into_steelval().unwrap());
                                s.set_index(1, (*TASK_DONE).clone());
                                mutex_unlock(&lock.unwrap()).unwrap();
                                return Err(e);
                            }
                        }
                    }

                    Ok(())
                })
            {
                present_error_inside_engine_context(&mut ctx, guard, e);
            }
        });
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

    engine.register_value(CTX, SteelVal::Void);
    engine.register_value(CONFIG, SteelVal::Void);
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

    engine.register_fn("log::debug!", |message: SteelVal| {
        if let SteelVal::StringV(s) = &message {
            log::debug!("{}", s)
        } else {
            log::debug!("{}", message)
        }
    });

    engine.register_fn("log::warn!", |message: SteelVal| {
        if let SteelVal::StringV(s) = &message {
            log::warn!("{}", s)
        } else {
            log::warn!("{}", message)
        }
    });

    engine.register_fn("log::error!", |message: SteelVal| {
        if let SteelVal::StringV(s) = &message {
            log::error!("{}", s)
        } else {
            log::error!("{}", message)
        }
    });

    engine.register_fn("log::info", |message: SteelVal| {
        if let SteelVal::StringV(s) = &message {
            log::info!("{}", s)
        } else {
            log::info!("{}", message)
        }
    });

    engine.register_fn("log::debug", |message: SteelVal| {
        if let SteelVal::StringV(s) = &message {
            log::debug!("{}", s)
        } else {
            log::debug!("{}", message)
        }
    });

    engine.register_fn("log::warn", |message: SteelVal| {
        if let SteelVal::StringV(s) = &message {
            log::warn!("{}", s)
        } else {
            log::warn!("{}", message)
        }
    });

    engine.register_fn("log::error", |message: SteelVal| {
        if let SteelVal::StringV(s) = &message {
            log::error!("{}", s)
        } else {
            log::error!("{}", message)
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
                wrapped.inner.as_any().is::<SteelDynamicComponent>()
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

                                    engine.update_value(CTX, context);

                                    engine.call_function_with_args(
                                        cloned_func.clone(),
                                        vec![input.into_steelval().unwrap()],
                                    )
                                })
                            {
                                present_error_inside_engine_context(&mut ctx, guard, e);
                            }
                        })
                    });

                    patch_callbacks(&mut ctx);
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
                    Ok(_) => with_ephemeral_context(cx, |ctx| {
                        let cloned_func = rooted.value();
                        enter_engine(|guard| {
                            if let Err(e) = guard
                                .with_mut_reference::<Context, Context>(ctx)
                                .consume_once(move |engine, args| {
                                    let context = args.into_iter().next().unwrap();
                                    engine.update_value(CTX, context);
                                    engine.call_function_with_args(cloned_func.clone(), Vec::new())
                                })
                            {
                                present_error_inside_engine_context(ctx, guard, e);
                            }
                        });
                    }),
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

fn make_ephemeral_context<'a, 'b>(cx: &'a mut compositor::Context<'b>) -> Context<'a> {
    Context {
        register: None,
        count: std::num::NonZeroUsize::new(1),
        editor: cx.editor,
        callback: Vec::new(),
        on_next_key_callback: None,
        jobs: cx.jobs,
    }
}

fn with_context_guard<'a, 'b>(cx: &'a mut compositor::Context<'b>) -> ContextGuard<'a> {
    ContextGuard {
        ctx: Context {
            register: None,
            count: std::num::NonZeroUsize::new(1),
            editor: cx.editor,
            callback: Vec::new(),
            on_next_key_callback: None,
            jobs: cx.jobs,
        },
    }
}

struct ContextGuard<'a> {
    ctx: Context<'a>,
}

impl<'a> Drop for ContextGuard<'a> {
    fn drop(&mut self) {
        patch_callbacks(&mut self.ctx);
    }
}

impl<'a> Deref for ContextGuard<'a> {
    type Target = Context<'a>;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl<'a> DerefMut for ContextGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

/// Creates a command context from a compositor context, and patches any callbacks
/// created back on to the local job queue.
fn with_ephemeral_context<'a, 'b, O>(
    cx: &'a mut compositor::Context<'b>,
    thunk: impl FnOnce(&mut Context<'_>) -> O,
) -> O {
    let mut context = make_ephemeral_context(cx);
    let res = thunk(&mut context);
    patch_callbacks(&mut context);
    res
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
        let call: Box<LocalJobCallback> = Box::new(
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
                            engine.update_value(CTX, context);
                            engine.compile_and_run_raw_program(text.clone())
                        })
                });

                patch_callbacks(&mut ctx);

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
        let call: Box<LocalJobCallback> = Box::new(
            move |editor: &mut Editor, compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut cx = compositor::Context {
                    editor,
                    scroll: None,
                    jobs,
                };

                let output = with_ephemeral_context(&mut cx, move |ctx| {
                    enter_engine(move |guard| {
                        guard
                            .with_mut_reference::<Context, Context>(ctx)
                            .consume_once(move |engine, args| {
                                let context = args.into_iter().next().unwrap();
                                engine.update_value(CTX, context);

                                match path.clone() {
                                    Some(path) => engine.compile_and_run_raw_program_with_path(
                                        text,
                                        PathBuf::from(path),
                                    ),
                                    None => engine.compile_and_run_raw_program(text.clone()),
                                }
                            })
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
                    Err(e) => with_ephemeral_context(&mut cx, |ctx| {
                        enter_engine(|x| present_error_inside_engine_context(ctx, x, e))
                    }),
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
    cx.editor.documents.contains_key(&doc_id)
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

fn string_to_mode(_: &mut Context, value: SteelString) -> Option<Mode> {
    match value.as_str() {
        "normal" => Some(Mode::Normal),
        "insert" => Some(Mode::Insert),
        "select" => Some(Mode::Select),
        _ => None,
    }
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
        let call: Box<LocalJobCallback> = Box::new(
            move |_editor: &mut Editor, compositor: &mut Compositor, _| compositor.push(inner),
        );
        Ok(call)
    };
    cx.jobs.local_callback(callback);
}

fn pop_last_component_by_name(cx: &mut Context, name: SteelString) {
    let callback = async move {
        let call: Box<LocalJobCallback> = Box::new(
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

fn trigger_callback(cx: &mut Context, key: KeyEvent) {
    if let Some(callback) = cx.on_next_key_callback.take() {
        (callback.0)(cx, key);
    }
}

fn enqueue_on_next_key(cx: &mut Context, callback_fn: SteelVal) {
    let rooted = callback_fn.as_rooted();
    let current_gen = load_generation();

    cx.on_next_key(move |ctx, key| {
        let cloned_func = rooted.value();

        enter_engine(|guard| {
            if !is_current_generation(current_gen) {
                return;
            }

            if let Err(e) =
                guard
                    .with_mut_reference::<Context, Context>(ctx)
                    .consume(move |engine, args| {
                        let context = args[0].clone();
                        engine.update_value(CTX, context);

                        engine.call_function_with_args_from_mut_slice(
                            cloned_func.clone(),
                            &mut [key.into_steelval().unwrap()],
                        )
                    })
            {
                present_error_inside_engine_context(ctx, guard, e);
            }
        });
    });
}

fn enqueue_command(cx: &mut Context, callback_fn: SteelVal) {
    let rooted = callback_fn.as_rooted();
    let current_gen = load_generation();

    let callback = async move {
        let call: Box<LocalJobCallback> = Box::new(
            move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut compositor_context = compositor::Context {
                    editor,
                    jobs,
                    scroll: None,
                };

                let mut ctx = with_context_guard(&mut compositor_context);

                let cloned_func = rooted.value();

                enter_engine(|guard| {
                    if !is_current_generation(current_gen) {
                        return;
                    }

                    if let Err(e) = guard
                        .with_mut_reference::<Context, Context>(&mut ctx)
                        .consume(move |engine, args| {
                            let context = args[0].clone();
                            engine.update_value(CTX, context);

                            engine.call_function_with_args(cloned_func.clone(), Vec::new())
                        })
                    {
                        present_error_inside_engine_context(&mut ctx, guard, e);
                    }
                });
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

        let call: Box<LocalJobCallback> = Box::new(
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
                            engine.update_value(CTX, context);

                            engine.call_function_with_args(cloned_func.clone(), Vec::new())
                        })
                    {
                        present_error_inside_engine_context(&mut ctx, guard, e);
                    }
                });

                patch_callbacks(&mut ctx);
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

        let call: Box<LocalJobCallback> = Box::new(
            move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut compositor_context = compositor::Context {
                    editor,
                    jobs,
                    scroll: None,
                };

                let mut ctx = with_context_guard(&mut compositor_context);

                let cloned_func = rooted.value();

                match future_value {
                    Ok(inner) => {
                        let callback = move |engine: &mut Engine, args: Vec<SteelVal>| {
                            let context = args.into_iter().next().unwrap();
                            engine.update_value(CTX, context);
                            engine.call_function_with_args_from_mut_slice(
                                cloned_func.clone(),
                                &mut [inner],
                            )
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
                        });
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
                let loader: &helix_core::syntax::Loader = &cx.editor.syn_loader.load();
                // If we are between pairs (such as brackets), we want to
                // insert an additional line which is indented one level
                // more and place the cursor there
                let on_auto_pair = doc
                    .auto_pairs(cx.editor, loader, view)
                    .and_then(|pairs| pairs.get(prev))
                    .is_some_and(|pair| pair.open == prev && pair.close == curr);

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

type LocalJobCallback = dyn FnOnce(&mut Editor, &mut Compositor, &mut job::Jobs);

fn create_callback<T: TryInto<SteelVal, Error = SteelErr> + 'static>(
    cx: &mut Context,
    future: impl std::future::Future<Output = Result<T, helix_lsp::Error>> + 'static,
    rooted: steel::RootedSteelVal,
) -> Result<(), anyhow::Error> {
    let callback = async move {
        // Result of the future - this will be whatever we get back
        // from the lsp call
        let res = future.await?;

        let call: Box<LocalJobCallback> = Box::new(
            move |editor: &mut Editor, _compositor: &mut Compositor, jobs: &mut job::Jobs| {
                let mut compositor_context = compositor::Context {
                    editor,
                    jobs,
                    scroll: None,
                };

                let mut ctx = with_context_guard(&mut compositor_context);

                let cloned_func = rooted.value();

                enter_engine(move |guard| match TryInto::<SteelVal>::try_into(res) {
                    Ok(result) => {
                        let res = guard
                            .with_mut_reference::<Context, Context>(&mut ctx)
                            .consume(move |engine, args| {
                                let context = args[0].clone();
                                engine.update_value(CTX, context);

                                engine.call_function_with_args(
                                    cloned_func.clone(),
                                    vec![result.clone()],
                                )
                            });

                        if let Err(e) = res {
                            present_error_inside_engine_context(&mut ctx, guard, e);
                        };
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

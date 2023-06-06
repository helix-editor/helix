use fuzzy_matcher::FuzzyMatcher;
use helix_core::{graphemes, Tendril};
use helix_view::{document::Mode, Document, DocumentId, Editor};
use once_cell::sync::Lazy;
use steel::{
    gc::unsafe_erased_pointers::CustomReference,
    rvals::{FromSteelVal, IntoSteelVal, SteelString},
    steel_vm::{engine::Engine, register_fn::RegisterFn},
    SteelErr, SteelVal,
};

use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    sync::Mutex,
};
use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use steel::{rvals::Custom, steel_vm::builtin::BuiltInModule};

use crate::{
    compositor::{self, Component, Compositor},
    job::{self, Callback},
    keymap::{merge_keys, Keymap},
    ui::{self, menu::Item, overlay::overlaid, Popup, PromptEvent},
};

use self::components::SteelDynamicComponent;

use super::{
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

// External modules that can load via rust dylib. These can then be consumed from
// steel as needed, via the standard FFI for plugin functions.
pub(crate) static EXTERNAL_DYLIBS: Lazy<Arc<RwLock<ExternalContainersAndModules>>> =
    Lazy::new(|| {
        let mut containers = DylibContainers::new();

        // Load the plugins with respect to the extensions directory.
        // containers.load_modules_from_directory(Some(
        //     helix_loader::config_dir()
        //         .join("extensions")
        //         .to_str()
        //         .unwrap()
        //         .to_string(),
        // ));

        println!("Found dylibs: {}", containers.containers.len());

        let modules = containers.create_commands();

        println!("Modules length: {}", modules.len());

        Arc::new(RwLock::new(ExternalContainersAndModules {
            containers,
            modules,
        }))

        // Arc::new(RwLock::new(containers))
    });

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

/// Run the initialization script located at `$helix_config/init.scm`
/// This runs the script in the global environment, and does _not_ load it as a module directly
pub fn run_initialization_script(cx: &mut Context) {
    log::info!("Loading init.scm...");

    let helix_module_path = helix_loader::steel_init_file();

    // These contents need to be registered with the path?
    if let Ok(contents) = std::fs::read_to_string(&helix_module_path) {
        let res = ENGINE.with(|x| {
            x.borrow_mut()
                .run_with_reference::<Context, Context>(cx, "*helix.cx*", &contents)
        });

        match res {
            Ok(_) => {}
            Err(e) => present_error(cx, e),
        }

        log::info!("Finished loading init.scm!")
    } else {
        log::info!("No init.scm found, skipping loading.")
    }

    // Start the worker thread - i.e. message passing to the workers
    // configure_background_thread()
}

// pub static MINOR_MODES: Lazy<Arc<RwLock<HashMap<String,

pub static KEYBINDING_QUEUE: Lazy<SharedKeyBindingsEventQueue> =
    Lazy::new(|| SharedKeyBindingsEventQueue::new());

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

/// In order to send events from the engine back to the configuration, we can created a shared
/// queue that the engine and the config push and pull from. Alternatively, we could use a channel
/// directly, however this was easy enough to set up.
pub struct SharedKeyBindingsEventQueue {
    raw_bindings: Arc<Mutex<VecDeque<String>>>,
}

impl SharedKeyBindingsEventQueue {
    pub fn new() -> Self {
        Self {
            raw_bindings: std::sync::Arc::new(std::sync::Mutex::new(VecDeque::new())),
        }
    }

    pub fn merge(other_as_json: String) {
        KEYBINDING_QUEUE
            .raw_bindings
            .lock()
            .unwrap()
            .push_back(other_as_json);
    }

    pub fn get() -> Option<HashMap<Mode, Keymap>> {
        let mut guard = KEYBINDING_QUEUE.raw_bindings.lock().unwrap();

        if let Some(initial) = guard.pop_front() {
            let mut initial = serde_json::from_str(&initial).unwrap();

            while let Some(remaining_event) = guard.pop_front() {
                let bindings = serde_json::from_str(&remaining_event).unwrap();

                merge_keys(&mut initial, bindings);
            }

            return Some(initial);
        }

        None
    }
}

impl Custom for PromptEvent {}

impl<'a> CustomReference for Context<'a> {}

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

    engine.register_fn(
        "Popup::new",
        |contents: &mut WrappedDynComponent,
         position: helix_core::Position|
         -> WrappedDynComponent {
            let inner = contents.inner.take().unwrap(); // Panic, for now

            WrappedDynComponent {
                inner: Some(Box::new(
                    Popup::<BoxDynComponent>::new("popup", BoxDynComponent::new(inner))
                        .position(Some(position)),
                )),
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

    engine.register_fn("editor-focus", current_focus);
    engine.register_fn("editor->doc-id", get_document_id);
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

    module.register_value(
        "PromptEvent::Validate",
        PromptEvent::Validate.into_steelval().unwrap(),
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

    // // Load the plugins with respect to the extensions directory.
    // EXTERNAL_DYLIBS
    //     .write()
    //     .unwrap()
    //     .load_modules_from_directory(Some(
    //         helix_loader::config_dir()
    //             .join("extensions")
    //             .to_str()
    //             .unwrap()
    //             .to_string(),
    //     ));

    // let commands = EXTERNAL_DYLIBS.read().unwrap().create_commands();

    // TODO: @Matt - these commands need to get loaded into their _own_ module, and registered as such.
    // for dylib in &EXTERNAL_DYLIBS.read().unwrap().modules {
    //     let mut module = BuiltInModule::new(dylib.name.to_string());

    //     println!("{}", dylib.get_name());

    //     // println!("{}", dylib.name);

    //     for command in dylib.commands.iter() {
    //         // TODO: The name needs to be registered for static - but we shouldn't _need_ it to be
    //         // registered for static. We can probably get away with accepting an owned string and pay the price,
    //         // if need be.

    //         let inner = command.fun.clone();

    //         let func = move |cx: &mut Context,
    //                          args: Box<[Box<str>]>,
    //                          event: PromptEvent|
    //               -> anyhow::Result<()> {
    //             // Ensure the lifetime of these variables
    //             let _config = cx.editor.config.clone();
    //             let _theme_loader = cx.editor.theme_loader.clone();
    //             let _syn_loader = cx.editor.syn_loader.clone();

    //             println!("{}", Arc::strong_count(&_config));
    //             println!("{}", Arc::strong_count(&_theme_loader));
    //             println!("{}", Arc::strong_count(&_syn_loader));
    //             // println!("{:p}", _theme_loader);
    //             // println!("{:p}", _syn_loader);

    //             (inner)(cx, &_theme_loader, &_syn_loader, args, &event)
    //         };

    //         module.register_owned_fn(command.name.to_string(), func);
    //     }

    //     engine.register_module(module);
    // }

    engine.register_module(module);

    let mut module = BuiltInModule::new("helix/core/static".to_string());

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
    module.register_fn("run-in-engine!", run_in_engine);
    module.register_fn("get-helix-scm-path", get_helix_scm_path);
    module.register_fn("get-init-scm-path", get_init_scm_path);
    module.register_fn("block-on-shell-command", run_shell_command_text);

    module.register_fn("cx->current-file", current_path);

    engine.register_module(module);

    engine.register_fn("push-component!", push_component);

    let helix_module_path = helix_loader::helix_module_file();

    engine
        .run(&format!(
            r#"(require "{}")"#,
            helix_module_path.to_str().unwrap()
        ))
        .unwrap();

    // __module-mangler/home/matt/Documents/steel/cogs/logging/log.scm

    // TODO: Use the helix.scm file located in the configuration directory instead
    // let mut working_directory = std::env::current_dir().unwrap();

    // working_directory.push("helix.scm");

    // working_directory = working_directory.canonicalize().unwrap();

    let helix_path =
        "__module-mangler".to_string() + helix_module_path.as_os_str().to_str().unwrap();

    // mangler/home/matt/Documents/steel/cogs/logging/log.scmlog/warn!__doc__

    let module_prefix = "mangler".to_string() + helix_module_path.as_os_str().to_str().unwrap();

    let module = engine.extract_value(&helix_path).unwrap();

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
                if let Ok(steel::rvals::SteelVal::StringV(d)) =
                    engine.extract_value(&(module_prefix.to_string() + x.as_str() + "__doc__"))
                {
                    Some((x.to_string(), d.to_string()))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        *EXPORTED_IDENTIFIERS.identifiers.write().unwrap() = exported;
        *EXPORTED_IDENTIFIERS.docs.write().unwrap() = docs;
    } else {
        panic!("Unable to parse exported identifiers from helix module!")
    }

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
        EXPORTED_IDENTIFIERS.get_doc(ident)
    }

    fn get_doc(&self, ident: &str) -> Option<String> {
        self.docs.read().unwrap().get(ident).cloned()
    }
}

fn get_highlighted_text(cx: &mut Context) -> String {
    let (view, doc) = current_ref!(cx.editor);
    let text = doc.text().slice(..);
    doc.selection(view.id).primary().slice(text).to_string()
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

fn document_exists(editor: &mut Editor, doc_id: DocumentId) -> bool {
    editor.documents.get(&doc_id).is_some()
}

fn document_path(doc: &Document) -> Option<String> {
    doc.path().and_then(|x| x.to_str()).map(|x| x.to_string())
}

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

// Pin the value to _this_ thread?

// Overlay the dynamic component, see what happens?
// Probably need to pin the values to this thread - wrap it in a shim which pins the value
// to this thread? - call methods on the thread local value?
fn push_component(cx: &mut Context, component: &mut WrappedDynComponent) {
    // let component = crate::ui::Text::new("Hello world!".to_string());

    log::info!("Pushing dynamic component!");

    // todo!();

    // let callback = async move {
    //     let call: job::Callback = Callback::EditorCompositor(Box::new(
    //         move |_editor: &mut Editor, compositor: &mut Compositor| {
    //             compositor.push(Box::new(component));
    //         },
    //     ));

    //     Ok(call)
    // };

    // cx.jobs.callback(callback);

    // Why does this not work? - UPDATE: This does work when called in a static command context, but
    // in a typed command context, we do not have access to the real compositor. Thus, we need a callback
    // that then requires the values to be moved over threads. We'll need some message passing scheme
    // to call values from the typed command context.
    cx.push_layer(component.inner.take().unwrap());

    // TODO: This _needs_ to go through a callback. Otherwise the new layer is just dropped.
    // Set up some sort of callback queue for dynamic components that we can pull from instead, so that
    // things stay thread local?

    // let root = helix_core::find_workspace().0;
    // let picker = ui::file_picker(root, &cx.editor.config());
    // cx.push_layer(Box::new(overlaid(picker)));
}

// fn push_component_raw(cx: &mut Context, component: Box<dyn Component>) {
//     cx.push_layer(component);
// }

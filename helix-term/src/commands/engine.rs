use fuzzy_matcher::FuzzyMatcher;
use helix_core::{graphemes, Tendril};
use helix_view::{document::Mode, Editor};
use once_cell::sync::Lazy;
use steel::{
    gc::unsafe_erased_pointers::CustomReference,
    rvals::{IntoSteelVal, SteelString},
    steel_vm::register_fn::RegisterFn,
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
    compositor::{self, Compositor},
    job::{self, Callback},
    keymap::{merge_keys, Keymap},
    ui::{self, Popup, PromptEvent},
};

use super::{
    insert::{insert_char, insert_string},
    Context, MappableCommand, TYPABLE_COMMAND_LIST,
};

thread_local! {
    pub static ENGINE: std::rc::Rc<std::cell::RefCell<steel::steel_vm::engine::Engine>> = configure_engine();
}

pub fn initialize_engine() {
    ENGINE.with(|x| x.borrow().globals().first().copied());
}

/// Run the initialization script located at `$helix_config/init.scm`
/// This runs the script in the global environment, and does _not_ load it as a module directly
pub fn run_initialization_script(cx: &mut Context) {
    log::info!("Loading init.scm...");

    let helix_module_path = helix_loader::steel_init_file();

    if let Ok(contents) = std::fs::read_to_string(&helix_module_path) {
        ENGINE.with(|x| {
            x.borrow_mut()
                .run_with_reference::<Context, Context>(cx, "*helix.cx*", &contents)
                .unwrap()
        });

        log::info!("Finished loading init.scm!")
    } else {
        log::info!("No init.scm found, skipping loading.")
    }

    // Start the worker thread - i.e. message passing to the workers
    configure_background_thread()
}

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

fn get_themes(cx: &mut Context) -> Vec<String> {
    ui::completers::theme(cx.editor, "")
        .into_iter()
        .map(|x| x.1.to_string())
        .collect()
}

fn configure_background_thread() {
    std::thread::spawn(move || {
        let mut engine = steel::steel_vm::engine::Engine::new();

        engine.register_fn("set-status-line!", StatusLineMessage::set);

        let helix_module_path = helix_loader::config_dir().join("background.scm");

        if let Ok(contents) = std::fs::read_to_string(&helix_module_path) {
            engine.run(&contents).ok();
        }
    });
}

fn configure_engine() -> std::rc::Rc<std::cell::RefCell<steel::steel_vm::engine::Engine>> {
    let mut engine = steel::steel_vm::engine::Engine::new();

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

    engine.register_module(module);

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

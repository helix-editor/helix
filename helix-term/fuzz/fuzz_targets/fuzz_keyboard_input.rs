#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use crossterm::event::Event;
use helix_term::{application::Application, args::Args, config::Config};
use helix_view::input::{parse_macro, KeyEvent};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzedData {
    events: Vec<KeyEvent>,
    #[arbitrary(with = arbitrary_config)]
    config: Config,
}

fn arbitrary_config(u: &mut Unstructured) -> arbitrary::Result<Config> {
    let mut config = Config::arbitrary(u)?;
    // Language servers have the ability to edit files e.g. format on save. Not to mention,
    // they are external to helix and shouldn't really be included in a fuzzer that's
    // intended to be deterministic.
    // TODO: Explore if LSP could be faked using fuzzed data.
    config.editor.lsp.enable = false;
    Ok(config)
}

/// Generates language configs that merge in overrides, like a user language
/// config. The argument string must be a raw TOML document.
pub fn test_syntax_conf(overrides: Option<String>) -> helix_core::syntax::Configuration {
    let mut lang = helix_loader::config::default_lang_config();

    if let Some(overrides) = overrides {
        let override_toml = toml::from_str(&overrides).unwrap();
        lang = helix_loader::merge_toml_values(lang, override_toml, 3);
    }

    lang.try_into().unwrap()
}

async fn fuzz_input(data: FuzzedData) -> anyhow::Result<()> {
    let mut app = Application::new(Args::default(), data.config, test_syntax_conf(None))?;

    for event in data.events.iter() {
        let key = Event::Key((*event).into());
        app.handle_terminal_events(Ok(key)).await;
    }

    for key_event in parse_macro("<esc>:q!<ret>")?.into_iter() {
        let key = Event::Key(key_event.into());
        app.handle_terminal_events(Ok(key)).await;
    }

    _ = app.close().await;
    Ok(())
}

fuzz_target!(|data: FuzzedData| {
    use tokio::runtime::Runtime;
    use tokio::task;

    let rt = Runtime::new().unwrap();
    let local = task::LocalSet::new();
    _ = local.block_on(&rt, async {
        _ = fuzz_input(data);
    });
});

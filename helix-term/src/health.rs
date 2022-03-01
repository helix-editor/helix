use crossterm::style::Stylize;
use helix_core::{
    config::{default_syntax_loader, user_syntax_loader},
    syntax::load_runtime_file,
};

/// Display general diagnostics.
pub fn general() {
    let config_file = helix_core::config_file();
    let lang_file = helix_core::lang_config_file();
    let log_file = helix_core::log_file();
    let rt_dir = helix_core::runtime_dir();

    if config_file.exists() {
        println!("Config file: {}", config_file.display());
    } else {
        println!("Config file: default")
    }
    if lang_file.exists() {
        println!("Language file: {}", lang_file.display());
    } else {
        println!("Language file: default")
    }
    println!("Log file: {}", log_file.display());
    println!("Runtime directory: {}", rt_dir.display());

    if let Ok(path) = std::fs::read_link(&rt_dir) {
        let msg = format!("Runtime directory is symlinked to {}", path.display());
        println!("{}", msg.yellow());
    }
    if !rt_dir.exists() {
        println!("{}", "Runtime directory does not exist.".red());
    }
    if rt_dir.read_dir().ok().map(|it| it.count()) == Some(0) {
        println!("{}", "Runtime directory is empty.".red());
    }
}

/// Display diagnostics pertaining to a particular language (LSP,
/// highlight queries, etc).
pub fn language(lang_str: String) {
    let syn_loader_conf = user_syntax_loader().unwrap_or_else(|err| {
        eprintln!("{}: {}", "Error parsing user language config".red(), err);
        eprintln!("{}", "Using default language config".yellow());
        default_syntax_loader()
    });

    let lang = match syn_loader_conf
        .language
        .iter()
        .find(|l| l.language_id == lang_str)
    {
        Some(l) => l,
        None => {
            let msg = format!("Language '{lang_str}' not found");
            println!("{}", msg.red());
            let suggestions: Vec<&str> = syn_loader_conf
                .language
                .iter()
                .filter(|l| l.language_id.starts_with(lang_str.chars().next().unwrap()))
                .map(|l| l.language_id.as_str())
                .collect();
            if !suggestions.is_empty() {
                let suggestions = suggestions.join(", ");
                println!("Did you mean one of these: {} ?", suggestions.yellow());
            }
            return;
        }
    };

    probe_protocol(
        "language server",
        lang.language_server
            .as_ref()
            .map(|lsp| lsp.command.to_string()),
    );

    probe_protocol(
        "debug adapter",
        lang.debugger.as_ref().map(|dap| dap.command.to_string()),
    );

    probe_treesitter_feature(&lang_str, "Highlight", "highlights.scm");
    probe_treesitter_feature(&lang_str, "Textobject", "textobjects.scm");
    probe_treesitter_feature(&lang_str, "Indent", "indents.toml");
}

/// Display diagnostics about LSP and DAP.
fn probe_protocol(protocol_name: &str, server_cmd: Option<String>) {
    let cmd_name = match server_cmd {
        Some(ref cmd) => cmd.as_str().green(),
        None => "None".yellow(),
    };
    println!("Configured {}: {}", protocol_name, cmd_name);

    if let Some(cmd) = server_cmd {
        let path = match which::which(&cmd) {
            Ok(path) => path.display().to_string().green(),
            Err(_) => "Not found in $PATH".to_string().red(),
        };
        println!("Binary for {}: {}", protocol_name, path);
    }
}

/// Display diagnostics about a feature that requires tree-sitter
/// query files (highlights, textobjects, etc).
fn probe_treesitter_feature(lang: &str, feature: &str, query_filename: &str) {
    let found = match load_runtime_file(lang, query_filename).is_ok() {
        true => "Found".green(),
        false => "Not found".red(),
    };
    println!("{} queries: {}", feature, found);
}

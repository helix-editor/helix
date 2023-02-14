use crossterm::{
    style::{Color, Print, Stylize},
    tty::IsTty,
};
use helix_core::syntax::{LanguageConfiguration, LanguageConfigurations};
use helix_loader::grammar;
use helix_loader::ts_probe::TsFeature;
use helix_view::clipboard;
use std::io::Write;

pub fn print_health(health_arg: Option<String>) -> std::io::Result<()> {
    match health_arg.as_deref() {
        None => {
            display_paths()?;
            display_clipboard()?;
            writeln!(std::io::stdout().lock())?;
            display_all_languages()?;
        }
        Some("paths") => display_paths()?,
        Some("clipboard") => display_clipboard()?,
        Some("languages") => display_all_languages()?,
        Some(lang) => display_language(lang.to_string())?,
    }
    Ok(())
}

fn display_paths() -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();

    writeln!(
        stdout,
        "Default config merged with user preferences supplied in:"
    )?;
    writeln!(stdout, "Config: {}", helix_loader::config_file().display())?;
    writeln!(
        stdout,
        "Language config: {}",
        helix_loader::user_lang_config_file().display()
    )?;
    writeln!(stdout, "Log file: {}", helix_loader::log_file().display())?;

    let rt_dirs = helix_loader::get_runtime_dirs();
    writeln!(stdout, "Runtime directories by order of priority:",)?;

    for rt_dir in rt_dirs {
        write!(stdout, "- {};", rt_dir.display())?;

        if let Ok(path) = std::fs::read_link(&rt_dir) {
            let msg = format!(" (symlinked to {})", path.display());
            write!(stdout, "{}", msg.yellow())?;
        }
        if rt_dir.read_dir().ok().map(|it| it.count()) == Some(0) {
            write!(stdout, "{}", " is empty.".yellow())?;
        }
        if !rt_dir.exists() {
            write!(stdout, "{}", " does not exist.".red())?;
        }
        writeln!(stdout)?;
    }

    Ok(())
}

fn display_clipboard() -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    let clipboard = clipboard::get_clipboard_provider();
    match clipboard.name().as_ref() {
        "none" => {
            writeln!(
                stdout,
                "{}",
                "No system clipboard provider installed, refer to:".red()
            )?;
            let link = "https://github.com/helix-editor/helix/wiki/Troubleshooting#copypaste-fromto-system-clipboard-not-working";
            writeln!(stdout, "{}", link.red().underlined())?;
        }
        name => writeln!(stdout, "System clipboard provider: {}", name)?,
    }
    Ok(())
}

fn load_merged_language_configurations() -> std::io::Result<Vec<LanguageConfiguration>> {
    LanguageConfigurations::merged()
        .or_else(|err| {
            let mut stderr = std::io::stderr().lock();
            writeln!(
                stderr,
                "{}: {}",
                "Error parsing user language config".red(),
                err
            )?;
            writeln!(stderr, "{}", "Using default language config".yellow())?;
            Ok(LanguageConfigurations::default())
        })
        .map(|lang_configs| lang_configs.language)
}

fn display_language(lang_str: String) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();

    let language_configurations = load_merged_language_configurations()?;
    let lang = match language_configurations
        .iter()
        .find(|l| l.language_id == lang_str)
    {
        Some(found_language) => found_language,
        None => {
            writeln!(
                stdout,
                "{}",
                format!("Language '{lang_str}' not found").red()
            )?;
            let suggestions: Vec<&str> = language_configurations
                .iter()
                .filter(|l| l.language_id.starts_with(lang_str.chars().next().unwrap()))
                .map(|l| l.language_id.as_str())
                .collect();
            if !suggestions.is_empty() {
                let suggestions = suggestions.join(", ");
                writeln!(
                    stdout,
                    "Did you mean one of these: {} ?",
                    suggestions.yellow()
                )?;
            }
            return Ok(());
        }
    };

    let probe_protocol = |protocol_name: &str, server_cmd: Option<String>| -> std::io::Result<()> {
        let mut stdout = std::io::stdout().lock();
        match server_cmd {
            Some(server_cmd) => {
                writeln!(
                    stdout,
                    "Configured {protocol_name}: {}",
                    server_cmd.clone().green()
                )?;
                let result = match which::which(&server_cmd) {
                    Ok(path) => path.display().to_string().green(),
                    Err(_) => "Not found in $PATH".to_string().red(),
                };
                writeln!(stdout, "Binary for {server_cmd}: {result}")?
            }
            None => writeln!(stdout, "Configured {protocol_name}: {}", "None".yellow())?,
        };
        Ok(())
    };

    probe_protocol(
        "language server",
        lang.language_server
            .as_ref()
            .map(|lsp| lsp.command.to_string()),
    )?;
    probe_protocol(
        "debug adapter",
        lang.debugger.as_ref().map(|dap| dap.command.to_string()),
    )?;
    if lang.formatter.is_some() {
        probe_protocol(
            "external formatter",
            lang.formatter
                .as_ref()
                .map(|fmtcfg| fmtcfg.command.to_string()),
        )?;
    }

    for feature in TsFeature::all() {
        let supported =
            match grammar::load_runtime_file(&lang.language_id, feature.runtime_filename()).is_ok()
            {
                true => "✓".green(),
                false => "✗".red(),
            };
        writeln!(stdout, "{} queries: {supported}", feature.short_title())?;
    }
    Ok(())
}

fn display_all_languages() -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();

    let mut column_headers = vec!["Language", "LSP", "DAP"];
    for treesitter_feature in TsFeature::all() {
        column_headers.push(treesitter_feature.short_title())
    }

    let column_width =
        crossterm::terminal::size().map(|(c, _)| c).unwrap_or(80) as usize / column_headers.len();
    let print_column = |item: &str, color: Color| {
        let mut data = format!(
            "{:column_width$}",
            item.get(..column_width - 2)
                .map(|s| format!("{}…", s))
                .unwrap_or_else(|| item.to_string())
        );

        if std::io::stdout().is_tty() {
            data = data.stylize().with(color).to_string();
        }
        // https://github.com/crossterm-rs/crossterm/issues/589
        let _ = crossterm::execute!(std::io::stdout(), Print(data));
    };

    for header in column_headers {
        print_column(header, Color::White);
    }
    writeln!(stdout)?;

    let check_binary = |cmd: Option<String>| match cmd {
        Some(cmd) => match which::which(&cmd) {
            Ok(_) => print_column(&format!("✓ {}", cmd), Color::Green),
            Err(_) => print_column(&format!("✗ {}", cmd), Color::Red),
        },
        None => print_column("None", Color::Yellow),
    };

    let mut language_configurations = load_merged_language_configurations()?;
    language_configurations.sort_unstable_by_key(|l| l.language_id.clone());
    for lang in &language_configurations {
        print_column(&lang.language_id, Color::Reset);

        let lsp = lang
            .language_server
            .as_ref()
            .map(|lsp| lsp.command.to_string());
        check_binary(lsp);
        let dap = lang.debugger.as_ref().map(|dap| dap.command.to_string());
        check_binary(dap);

        for ts_feat in TsFeature::all() {
            match grammar::load_runtime_file(&lang.language_id, ts_feat.runtime_filename()).is_ok()
            {
                true => print_column("✓", Color::Green),
                false => print_column("✗", Color::Red),
            }
        }
        writeln!(stdout)?;
    }
    Ok(())
}

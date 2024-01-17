use crossterm::{
    style::{Color, Print, Stylize},
    tty::IsTty,
};
use helix_core::config::{default_syntax_loader, user_syntax_loader};
use helix_loader::grammar::load_runtime_file;
use helix_view::clipboard::get_clipboard_provider;
use std::io::Write;

#[derive(Copy, Clone)]
pub enum TsFeature {
    Highlight,
    TextObject,
    AutoIndent,
}

impl TsFeature {
    pub fn all() -> &'static [Self] {
        &[Self::Highlight, Self::TextObject, Self::AutoIndent]
    }

    pub fn runtime_filename(&self) -> &'static str {
        match *self {
            Self::Highlight => "highlights.scm",
            Self::TextObject => "textobjects.scm",
            Self::AutoIndent => "indents.scm",
        }
    }

    pub fn long_title(&self) -> &'static str {
        match *self {
            Self::Highlight => "Syntax Highlighting",
            Self::TextObject => "Treesitter Textobjects",
            Self::AutoIndent => "Auto Indent",
        }
    }

    pub fn short_title(&self) -> &'static str {
        match *self {
            Self::Highlight => "Highlight",
            Self::TextObject => "Textobject",
            Self::AutoIndent => "Indent",
        }
    }
}

/// Display general diagnostics.
pub fn general() -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let config_file = helix_loader::config_file();
    let lang_file = helix_loader::lang_config_file();
    let log_file = helix_loader::log_file();
    let rt_dirs = helix_loader::runtime_dirs();
    let clipboard_provider = get_clipboard_provider();

    if config_file.exists() {
        writeln!(stdout, "Config file: {}", config_file.display())?;
    } else {
        writeln!(stdout, "Config file: default")?;
    }
    if lang_file.exists() {
        writeln!(stdout, "Language file: {}", lang_file.display())?;
    } else {
        writeln!(stdout, "Language file: default")?;
    }
    writeln!(stdout, "Log file: {}", log_file.display())?;
    writeln!(
        stdout,
        "Runtime directories: {}",
        rt_dirs
            .iter()
            .map(|d| d.to_string_lossy())
            .collect::<Vec<_>>()
            .join(";")
    )?;
    for rt_dir in rt_dirs.iter() {
        if let Ok(path) = std::fs::read_link(rt_dir) {
            let msg = format!(
                "Runtime directory {} is symlinked to: {}",
                rt_dir.display(),
                path.display()
            );
            writeln!(stdout, "{}", msg.yellow())?;
        }
        if !rt_dir.exists() {
            let msg = format!("Runtime directory does not exist: {}", rt_dir.display());
            writeln!(stdout, "{}", msg.yellow())?;
        } else if rt_dir.read_dir().ok().map(|it| it.count()) == Some(0) {
            let msg = format!("Runtime directory is empty: {}", rt_dir.display());
            writeln!(stdout, "{}", msg.yellow())?;
        }
    }
    writeln!(stdout, "Clipboard provider: {}", clipboard_provider.name())?;

    Ok(())
}

pub fn clipboard() -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let board = get_clipboard_provider();
    match board.name().as_ref() {
        "none" => {
            writeln!(
                stdout,
                "{}",
                "System clipboard provider: Not installed".red()
            )?;
            writeln!(
                stdout,
                "    {}",
                "For troubleshooting system clipboard issues, refer".red()
            )?;
            writeln!(stdout, "    {}",
                "https://github.com/helix-editor/helix/wiki/Troubleshooting#copypaste-fromto-system-clipboard-not-working"
            .red().underlined())?;
        }
        name => writeln!(stdout, "System clipboard provider: {}", name)?,
    }

    Ok(())
}

pub fn languages_all() -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let mut syn_loader_conf = match user_syntax_loader() {
        Ok(conf) => conf,
        Err(err) => {
            let stderr = std::io::stderr();
            let mut stderr = stderr.lock();

            writeln!(
                stderr,
                "{}: {}",
                "Error parsing user language config".red(),
                err
            )?;
            writeln!(stderr, "{}", "Using default language config".yellow())?;
            default_syntax_loader()
        }
    };

    let mut headings = vec!["Language", "LSP", "DAP", "Formatter"];

    for feat in TsFeature::all() {
        headings.push(feat.short_title())
    }

    let terminal_cols = crossterm::terminal::size().map(|(c, _)| c).unwrap_or(80);
    let column_width = terminal_cols as usize / headings.len();
    let is_terminal = std::io::stdout().is_tty();

    let column = |item: &str, color: Color| {
        let mut data = format!(
            "{:width$}",
            item.get(..column_width - 2)
                .map(|s| format!("{}…", s))
                .unwrap_or_else(|| item.to_string()),
            width = column_width,
        );
        if is_terminal {
            data = data.stylize().with(color).to_string();
        }

        // We can't directly use println!() because of
        // https://github.com/crossterm-rs/crossterm/issues/589
        let _ = crossterm::execute!(std::io::stdout(), Print(data));
    };

    for heading in headings {
        column(heading, Color::White);
    }
    writeln!(stdout)?;

    syn_loader_conf
        .language
        .sort_unstable_by_key(|l| l.language_id.clone());

    let check_binary = |cmd: Option<&str>| match cmd {
        Some(cmd) => match which::which(cmd) {
            Ok(_) => column(&format!("✓ {}", cmd), Color::Green),
            Err(_) => column(&format!("✘ {}", cmd), Color::Red),
        },
        None => column("None", Color::Yellow),
    };

    for lang in &syn_loader_conf.language {
        column(&lang.language_id, Color::Reset);

        let mut cmds = lang.language_servers.iter().filter_map(|ls| {
            syn_loader_conf
                .language_server
                .get(&ls.name)
                .map(|config| config.command.as_str())
        });
        check_binary(cmds.next());

        let dap = lang.debugger.as_ref().map(|dap| dap.command.as_str());
        check_binary(dap);

        let formatter = lang
            .formatter
            .as_ref()
            .map(|formatter| formatter.command.as_str());
        check_binary(formatter);

        for ts_feat in TsFeature::all() {
            match load_runtime_file(&lang.language_id, ts_feat.runtime_filename()).is_ok() {
                true => column("✓", Color::Green),
                false => column("✘", Color::Red),
            }
        }

        writeln!(stdout)?;

        for cmd in cmds {
            column("", Color::Reset);
            check_binary(Some(cmd));
            writeln!(stdout)?;
        }
    }

    Ok(())
}

/// Display diagnostics pertaining to a particular language (LSP,
/// highlight queries, etc).
pub fn language(lang_str: String) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let syn_loader_conf = match user_syntax_loader() {
        Ok(conf) => conf,
        Err(err) => {
            let stderr = std::io::stderr();
            let mut stderr = stderr.lock();

            writeln!(
                stderr,
                "{}: {}",
                "Error parsing user language config".red(),
                err
            )?;
            writeln!(stderr, "{}", "Using default language config".yellow())?;
            default_syntax_loader()
        }
    };

    let lang = match syn_loader_conf
        .language
        .iter()
        .find(|l| l.language_id == lang_str)
    {
        Some(l) => l,
        None => {
            let msg = format!("Language '{}' not found", lang_str);
            writeln!(stdout, "{}", msg.red())?;
            let suggestions: Vec<&str> = syn_loader_conf
                .language
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

    probe_protocols(
        "language server",
        lang.language_servers
            .iter()
            .filter_map(|ls| syn_loader_conf.language_server.get(&ls.name))
            .map(|config| config.command.as_str()),
    )?;

    probe_protocol(
        "debug adapter",
        lang.debugger.as_ref().map(|dap| dap.command.to_string()),
    )?;

    probe_protocol(
        "formatter",
        lang.formatter
            .as_ref()
            .map(|formatter| formatter.command.to_string()),
    )?;

    for ts_feat in TsFeature::all() {
        probe_treesitter_feature(&lang_str, *ts_feat)?
    }

    Ok(())
}

/// Display diagnostics about multiple LSPs and DAPs.
fn probe_protocols<'a, I: Iterator<Item = &'a str> + 'a>(
    protocol_name: &str,
    server_cmds: I,
) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let mut server_cmds = server_cmds.peekable();

    write!(stdout, "Configured {}s:", protocol_name)?;
    if server_cmds.peek().is_none() {
        writeln!(stdout, "{}", " None".yellow())?;
        return Ok(());
    }
    writeln!(stdout)?;

    for cmd in server_cmds {
        let (path, icon) = match which::which(cmd) {
            Ok(path) => (path.display().to_string().green(), "✓".green()),
            Err(_) => (format!("'{}' not found in $PATH", cmd).red(), "✘".red()),
        };
        writeln!(stdout, "  {} {}: {}", icon, cmd, path)?;
    }

    Ok(())
}

/// Display diagnostics about LSP and DAP.
fn probe_protocol(protocol_name: &str, server_cmd: Option<String>) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let cmd_name = match server_cmd {
        Some(ref cmd) => cmd.as_str().green(),
        None => "None".yellow(),
    };
    writeln!(stdout, "Configured {}: {}", protocol_name, cmd_name)?;

    if let Some(cmd) = server_cmd {
        let path = match which::which(&cmd) {
            Ok(path) => path.display().to_string().green(),
            Err(_) => format!("'{}' not found in $PATH", cmd).red(),
        };
        writeln!(stdout, "Binary for {}: {}", protocol_name, path)?;
    }

    Ok(())
}

/// Display diagnostics about a feature that requires tree-sitter
/// query files (highlights, textobjects, etc).
fn probe_treesitter_feature(lang: &str, feature: TsFeature) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let found = match load_runtime_file(lang, feature.runtime_filename()).is_ok() {
        true => "✓".green(),
        false => "✘".red(),
    };
    writeln!(stdout, "{} queries: {}", feature.short_title(), found)?;

    Ok(())
}

pub fn print_health(health_arg: Option<String>) -> std::io::Result<()> {
    match health_arg.as_deref() {
        Some("languages") => languages_all()?,
        Some("clipboard") => clipboard()?,
        None | Some("all") => {
            general()?;
            clipboard()?;
            writeln!(std::io::stdout().lock())?;
            languages_all()?;
        }
        Some(lang) => language(lang.to_string())?,
    }
    Ok(())
}

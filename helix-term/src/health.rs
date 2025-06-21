use crate::config::{Config, ConfigLoadError};
use crossterm::{
    style::{Color, StyledContent, Stylize},
    tty::IsTty,
};
use helix_core::config::{default_lang_config, user_lang_config};
use helix_loader::grammar::load_runtime_file;
use std::io::Write;

#[derive(Copy, Clone)]
pub enum TsFeature {
    Highlight,
    TextObject,
    AutoIndent,
    RainbowBracket,
}

impl TsFeature {
    pub fn all() -> &'static [Self] {
        &[
            Self::Highlight,
            Self::TextObject,
            Self::AutoIndent,
            Self::RainbowBracket,
        ]
    }

    pub fn runtime_filename(&self) -> &'static str {
        match *self {
            Self::Highlight => "highlights.scm",
            Self::TextObject => "textobjects.scm",
            Self::AutoIndent => "indents.scm",
            Self::RainbowBracket => "rainbows.scm",
        }
    }

    pub fn long_title(&self) -> &'static str {
        match *self {
            Self::Highlight => "Syntax Highlighting",
            Self::TextObject => "Treesitter Textobjects",
            Self::AutoIndent => "Auto Indent",
            Self::RainbowBracket => "Rainbow Brackets",
        }
    }

    pub fn short_title(&self) -> &'static str {
        match *self {
            Self::Highlight => "Highlight",
            Self::TextObject => "Textobject",
            Self::AutoIndent => "Indent",
            Self::RainbowBracket => "Rainbow",
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

    Ok(())
}

pub fn clipboard() -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let config = match Config::load_default() {
        Ok(config) => config,
        Err(ConfigLoadError::Error(err)) if err.kind() == std::io::ErrorKind::NotFound => {
            Config::default()
        }
        Err(err) => {
            writeln!(stdout, "{}", "Configuration file malformed".red())?;
            writeln!(stdout, "{}", err)?;
            return Ok(());
        }
    };

    match config.editor.clipboard_provider.name().as_ref() {
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

    let mut syn_loader_conf = match user_lang_config() {
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
            default_lang_config()
        }
    };

    let mut headings = vec!["Language", "Language servers", "Debug adapter", "Formatter"];

    for feat in TsFeature::all() {
        headings.push(feat.short_title())
    }

    let terminal_cols = crossterm::terminal::size().map(|(c, _)| c).unwrap_or(80);
    let column_width = terminal_cols as usize / headings.len();
    let is_terminal = std::io::stdout().is_tty();

    let fit = |s: &str| -> StyledContent<String> {
        format!(
            "{:column_width$}",
            s.get(..column_width - 2)
                .map(|s| format!("{}…", s))
                .unwrap_or_else(|| s.to_string())
        )
        .stylize()
    };
    let color = |s: StyledContent<String>, c: Color| if is_terminal { s.with(c) } else { s };
    let bold = |s: StyledContent<String>| if is_terminal { s.bold() } else { s };

    for heading in headings {
        write!(stdout, "{}", bold(fit(heading)))?;
    }
    writeln!(stdout)?;

    syn_loader_conf
        .language
        .sort_unstable_by_key(|l| l.language_id.clone());

    let check_binary_with_name = |cmd: Option<(&str, &str)>| match cmd {
        Some((name, cmd)) => match helix_stdx::env::which(cmd) {
            Ok(_) => color(fit(&format!("✓ {}", name)), Color::Green),
            Err(_) => color(fit(&format!("✘ {}", name)), Color::Red),
        },
        None => color(fit("None"), Color::Yellow),
    };

    let check_binary = |cmd: Option<&str>| check_binary_with_name(cmd.map(|cmd| (cmd, cmd)));

    for lang in &syn_loader_conf.language {
        write!(stdout, "{}", fit(&lang.language_id))?;

        let mut cmds = lang.language_servers.iter().filter_map(|ls| {
            syn_loader_conf
                .language_server
                .get(&ls.name)
                .map(|config| (ls.name.as_str(), config.command.as_str()))
        });
        write!(stdout, "{}", check_binary_with_name(cmds.next()))?;

        let dap = lang.debugger.as_ref().map(|dap| dap.command.as_str());
        write!(stdout, "{}", check_binary(dap))?;

        let formatter = lang
            .formatter
            .as_ref()
            .map(|formatter| formatter.command.as_str());
        write!(stdout, "{}", check_binary(formatter))?;

        for ts_feat in TsFeature::all() {
            match load_runtime_file(&lang.language_id, ts_feat.runtime_filename()).is_ok() {
                true => write!(stdout, "{}", color(fit("✓"), Color::Green))?,
                false => write!(stdout, "{}", color(fit("✘"), Color::Red))?,
            }
        }

        writeln!(stdout)?;

        for cmd in cmds {
            write!(stdout, "{}", fit(""))?;
            writeln!(stdout, "{}", check_binary_with_name(Some(cmd)))?;
        }
    }

    Ok(())
}

/// Display diagnostics pertaining to a particular language (LSP,
/// highlight queries, etc).
pub fn language(lang_str: String) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    let syn_loader_conf = match user_lang_config() {
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
            default_lang_config()
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
        lang.language_servers.iter().filter_map(|ls| {
            syn_loader_conf
                .language_server
                .get(&ls.name)
                .map(|config| (ls.name.as_str(), config.command.as_str()))
        }),
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

    probe_parser(lang.grammar.as_ref().unwrap_or(&lang.language_id))?;

    for ts_feat in TsFeature::all() {
        probe_treesitter_feature(&lang_str, *ts_feat)?
    }

    Ok(())
}

fn probe_parser(grammar_name: &str) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    write!(stdout, "Tree-sitter parser: ")?;

    match helix_loader::grammar::get_language(grammar_name) {
        Ok(_) => writeln!(stdout, "{}", "✓".green()),
        Err(_) => writeln!(stdout, "{}", "None".yellow()),
    }
}

/// Display diagnostics about multiple LSPs and DAPs.
fn probe_protocols<'a, I: Iterator<Item = (&'a str, &'a str)> + 'a>(
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

    for (name, cmd) in server_cmds {
        let (diag, icon) = match helix_stdx::env::which(cmd) {
            Ok(path) => (path.display().to_string().green(), "✓".green()),
            Err(_) => (format!("'{}' not found in $PATH", cmd).red(), "✘".red()),
        };
        writeln!(stdout, "  {} {}: {}", icon, name, diag)?;
    }

    Ok(())
}

/// Display diagnostics about LSP and DAP.
fn probe_protocol(protocol_name: &str, server_cmd: Option<String>) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();

    write!(stdout, "Configured {}:", protocol_name)?;
    let Some(cmd) = server_cmd else {
        writeln!(stdout, "{}", " None".yellow())?;
        return Ok(());
    };
    writeln!(stdout)?;

    let (diag, icon) = match helix_stdx::env::which(&cmd) {
        Ok(path) => (path.display().to_string().green(), "✓".green()),
        Err(_) => (format!("'{}' not found in $PATH", cmd).red(), "✘".red()),
    };
    writeln!(stdout, "  {} {}", icon, diag)?;

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

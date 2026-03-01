use crate::{backend::Backend, buffer::Cell, terminal::Config};
use crossterm::{
    cursor::{Hide, MoveTo, SetCursorStyle, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute, queue,
    style::{
        Attribute as CAttribute, Color as CColor, Colors, Print, SetAttribute, SetBackgroundColor,
        SetColors, SetForegroundColor,
    },
    terminal::{self, Clear, ClearType},
    Command,
};
use helix_view::graphics::{Color, CursorKind, Modifier, Rect, UnderlineStyle};
use once_cell::sync::OnceCell;
use std::{
    fmt,
    io::{self, Write},
};
use termini::TermInfo;

fn term_program() -> Option<String> {
    // Some terminals don't set $TERM_PROGRAM
    match std::env::var("TERM_PROGRAM") {
        Err(_) => std::env::var("TERM").ok(),
        Ok(term_program) => Some(term_program),
    }
}
fn vte_version() -> Option<usize> {
    std::env::var("VTE_VERSION").ok()?.parse().ok()
}
fn reset_cursor_approach(terminfo: TermInfo) -> String {
    let mut reset_str = String::new();

    if let Some(termini::Value::Utf8String(se_str)) = terminfo.extended_cap("Se") {
        reset_str.push_str(se_str);
    };

    reset_str.push_str(
        terminfo
            .utf8_string_cap(termini::StringCapability::CursorNormal)
            .unwrap_or(""),
    );

    reset_str.push_str("\x1B[0 q");

    reset_str
}

/// Describes terminal capabilities like extended underline, truecolor, etc.
#[derive(Clone, Debug)]
struct Capabilities {
    /// Support for undercurled, underdashed, etc.
    has_extended_underlines: bool,
    /// Support for resetting the cursor style back to normal.
    reset_cursor_command: String,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            has_extended_underlines: false,
            reset_cursor_command: "\x1B[0 q".to_string(),
        }
    }
}

impl Capabilities {
    /// Detect capabilities from the terminfo database located based
    /// on the $TERM environment variable. If detection fails, returns
    /// a default value where no capability is supported, or just undercurl
    /// if config.undercurl is set.
    pub fn from_env_or_default(config: &Config) -> Self {
        match termini::TermInfo::from_env() {
            Err(_) => Capabilities {
                has_extended_underlines: config.force_enable_extended_underlines,
                ..Capabilities::default()
            },
            Ok(t) => Capabilities {
                // Smulx, VTE: https://unix.stackexchange.com/a/696253/246284
                // Su (used by kitty): https://sw.kovidgoyal.net/kitty/underlines
                // WezTerm supports underlines but a lot of distros don't properly install its terminfo
                has_extended_underlines: config.force_enable_extended_underlines
                    || t.extended_cap("Smulx").is_some()
                    || t.extended_cap("Su").is_some()
                    || vte_version() >= Some(5102)
                    || matches!(term_program().as_deref(), Some("WezTerm")),
                reset_cursor_command: reset_cursor_approach(t),
            },
        }
    }
}

/// Terminal backend supporting a wide variety of terminals
pub struct CrosstermBackend<W: Write> {
    buffer: W,
    config: Config,
    capabilities: Capabilities,
    supports_keyboard_enhancement_protocol: OnceCell<bool>,
    mouse_capture_enabled: bool,
    supports_bracketed_paste: bool,
}

impl<W> CrosstermBackend<W>
where
    W: Write,
{
    pub fn new(buffer: W, config: Config) -> CrosstermBackend<W> {
        // helix is not usable without colors, but crossterm will disable
        // them by default if NO_COLOR is set in the environment. Override
        // this behaviour.
        crossterm::style::force_color_output(true);
        CrosstermBackend {
            buffer,
            capabilities: Capabilities::from_env_or_default(&config),
            config,
            supports_keyboard_enhancement_protocol: OnceCell::new(),
            mouse_capture_enabled: false,
            supports_bracketed_paste: true,
        }
    }

    #[inline]
    fn supports_keyboard_enhancement_protocol(&self) -> bool {
        *self.supports_keyboard_enhancement_protocol
            .get_or_init(|| {
                use std::time::Instant;

                let now = Instant::now();
                let supported = matches!(terminal::supports_keyboard_enhancement(), Ok(true));
                log::debug!(
                    "The keyboard enhancement protocol is {}supported in this terminal (checked in {:?})",
                    if supported { "" } else { "not " },
                    Instant::now().duration_since(now)
                );
                supported
            })
    }
}

impl<W> Write for CrosstermBackend<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buffer.flush()
    }
}

impl<W> Backend for CrosstermBackend<W>
where
    W: Write,
{
    fn claim(&mut self) -> io::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(
            self.buffer,
            terminal::EnterAlternateScreen,
            EnableFocusChange
        )?;
        match execute!(self.buffer, EnableBracketedPaste,) {
            Err(err) if err.kind() == io::ErrorKind::Unsupported => {
                log::warn!("Bracketed paste is not supported on this terminal.");
                self.supports_bracketed_paste = false;
            }
            Err(err) => return Err(err),
            Ok(_) => (),
        };
        execute!(self.buffer, terminal::Clear(terminal::ClearType::All))?;
        if self.config.enable_mouse_capture {
            execute!(self.buffer, EnableMouseCapture)?;
            self.mouse_capture_enabled = true;
        }
        if self.supports_keyboard_enhancement_protocol() {
            execute!(
                self.buffer,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                )
            )?;
        }
        Ok(())
    }

    fn reconfigure(&mut self, config: Config) -> io::Result<()> {
        if self.mouse_capture_enabled != config.enable_mouse_capture {
            if config.enable_mouse_capture {
                execute!(self.buffer, EnableMouseCapture)?;
            } else {
                execute!(self.buffer, DisableMouseCapture)?;
            }
            self.mouse_capture_enabled = config.enable_mouse_capture;
        }
        self.config = config;

        Ok(())
    }

    fn restore(&mut self) -> io::Result<()> {
        // reset cursor shape
        self.buffer
            .write_all(self.capabilities.reset_cursor_command.as_bytes())?;
        if self.config.enable_mouse_capture {
            execute!(self.buffer, DisableMouseCapture)?;
        }
        if self.supports_keyboard_enhancement_protocol() {
            execute!(self.buffer, PopKeyboardEnhancementFlags)?;
        }
        if self.supports_bracketed_paste {
            execute!(self.buffer, DisableBracketedPaste,)?;
        }
        execute!(
            self.buffer,
            DisableFocusChange,
            terminal::LeaveAlternateScreen
        )?;
        terminal::disable_raw_mode()
    }

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        let mut fg = Color::Reset;
        let mut bg = Color::Reset;
        let mut underline_color = Color::Reset;
        let mut underline_style = UnderlineStyle::Reset;
        let mut modifier = Modifier::empty();
        let mut last_pos: Option<(u16, u16)> = None;
        for (x, y, cell) in content {
            // Move the cursor if the previous location was not (x - 1, y)
            if !matches!(last_pos, Some(p) if x == p.0 + 1 && y == p.1) {
                queue!(self.buffer, MoveTo(x, y))?;
            }
            last_pos = Some((x, y));
            if cell.modifier != modifier {
                let diff = ModifierDiff {
                    from: modifier,
                    to: cell.modifier,
                };
                diff.queue(&mut self.buffer)?;
                modifier = cell.modifier;
            }
            if cell.fg != fg || cell.bg != bg {
                queue!(
                    self.buffer,
                    SetColors(Colors::new(cell.fg.into(), cell.bg.into()))
                )?;
                fg = cell.fg;
                bg = cell.bg;
            }

            let mut new_underline_style = cell.underline_style;
            if self.capabilities.has_extended_underlines {
                if cell.underline_color != underline_color {
                    let color = CColor::from(cell.underline_color);
                    queue!(self.buffer, SetUnderlineColor(color))?;
                    underline_color = cell.underline_color;
                }
            } else {
                match new_underline_style {
                    UnderlineStyle::Reset | UnderlineStyle::Line => (),
                    _ => new_underline_style = UnderlineStyle::Line,
                }
            }

            if new_underline_style != underline_style {
                let attr = CAttribute::from(new_underline_style);
                queue!(self.buffer, SetAttribute(attr))?;
                underline_style = new_underline_style;
            }

            queue!(self.buffer, Print(&cell.symbol))?;
        }

        queue!(
            self.buffer,
            SetUnderlineColor(CColor::Reset),
            SetForegroundColor(CColor::Reset),
            SetBackgroundColor(CColor::Reset),
            SetAttribute(CAttribute::Reset)
        )
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        execute!(self.buffer, Hide)
    }

    fn show_cursor(&mut self, kind: CursorKind) -> io::Result<()> {
        let shape = match kind {
            CursorKind::Block => SetCursorStyle::SteadyBlock,
            CursorKind::Bar => SetCursorStyle::SteadyBar,
            CursorKind::Underline => SetCursorStyle::SteadyUnderScore,
            CursorKind::Hidden => unreachable!(),
        };
        execute!(self.buffer, Show, shape)
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        execute!(self.buffer, MoveTo(x, y))
    }

    fn clear(&mut self) -> io::Result<()> {
        execute!(self.buffer, Clear(ClearType::All))
    }

    fn size(&self) -> io::Result<Rect> {
        let (width, height) =
            terminal::size().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        Ok(Rect::new(0, 0, width, height))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buffer.flush()
    }

    fn supports_true_color(&self) -> bool {
        false
    }

    fn get_theme_mode(&self) -> Option<helix_view::theme::Mode> {
        None
    }
}

#[derive(Debug)]
struct ModifierDiff {
    pub from: Modifier,
    pub to: Modifier,
}

impl ModifierDiff {
    fn queue<W>(&self, mut w: W) -> io::Result<()>
    where
        W: io::Write,
    {
        //use crossterm::Attribute;
        let removed = self.from - self.to;
        if removed.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CAttribute::NoReverse))?;
        }
        if removed.contains(Modifier::BOLD) {
            queue!(w, SetAttribute(CAttribute::NormalIntensity))?;
            if self.to.contains(Modifier::DIM) {
                queue!(w, SetAttribute(CAttribute::Dim))?;
            }
        }
        if removed.contains(Modifier::ITALIC) {
            queue!(w, SetAttribute(CAttribute::NoItalic))?;
        }
        if removed.contains(Modifier::DIM) {
            queue!(w, SetAttribute(CAttribute::NormalIntensity))?;
        }
        if removed.contains(Modifier::CROSSED_OUT) {
            queue!(w, SetAttribute(CAttribute::NotCrossedOut))?;
        }
        if removed.contains(Modifier::SLOW_BLINK) || removed.contains(Modifier::RAPID_BLINK) {
            queue!(w, SetAttribute(CAttribute::NoBlink))?;
        }
        if removed.contains(Modifier::HIDDEN) {
            queue!(w, SetAttribute(CAttribute::NoHidden))?;
        }

        let added = self.to - self.from;
        if added.contains(Modifier::REVERSED) {
            queue!(w, SetAttribute(CAttribute::Reverse))?;
        }
        if added.contains(Modifier::BOLD) {
            queue!(w, SetAttribute(CAttribute::Bold))?;
        }
        if added.contains(Modifier::ITALIC) {
            queue!(w, SetAttribute(CAttribute::Italic))?;
        }
        if added.contains(Modifier::DIM) {
            queue!(w, SetAttribute(CAttribute::Dim))?;
        }
        if added.contains(Modifier::CROSSED_OUT) {
            queue!(w, SetAttribute(CAttribute::CrossedOut))?;
        }
        if added.contains(Modifier::SLOW_BLINK) {
            queue!(w, SetAttribute(CAttribute::SlowBlink))?;
        }
        if added.contains(Modifier::RAPID_BLINK) {
            queue!(w, SetAttribute(CAttribute::RapidBlink))?;
        }
        if added.contains(Modifier::HIDDEN) {
            queue!(w, SetAttribute(CAttribute::Hidden))?;
        }

        Ok(())
    }
}

/// Crossterm uses semicolon as a separator for colors
/// this is actually not spec compliant (although commonly supported)
/// However the correct approach is to use colons as a separator.
/// This usually doesn't make a difference for emulators that do support colored underlines.
/// However terminals that do not support colored underlines will ignore underlines colors with colons
/// while escape sequences with semicolons are always processed which leads to weird visual artifacts.
/// See [this nvim issue](https://github.com/neovim/neovim/issues/9270) for details
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetUnderlineColor(pub CColor);

impl Command for SetUnderlineColor {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        let color = self.0;

        if color == CColor::Reset {
            write!(f, "\x1b[59m")?;
            return Ok(());
        }
        f.write_str("\x1b[58:")?;

        let res = match color {
            CColor::Black => f.write_str("5:0"),
            CColor::DarkGrey => f.write_str("5:8"),
            CColor::Red => f.write_str("5:9"),
            CColor::DarkRed => f.write_str("5:1"),
            CColor::Green => f.write_str("5:10"),
            CColor::DarkGreen => f.write_str("5:2"),
            CColor::Yellow => f.write_str("5:11"),
            CColor::DarkYellow => f.write_str("5:3"),
            CColor::Blue => f.write_str("5:12"),
            CColor::DarkBlue => f.write_str("5:4"),
            CColor::Magenta => f.write_str("5:13"),
            CColor::DarkMagenta => f.write_str("5:5"),
            CColor::Cyan => f.write_str("5:14"),
            CColor::DarkCyan => f.write_str("5:6"),
            CColor::White => f.write_str("5:15"),
            CColor::Grey => f.write_str("5:7"),
            CColor::Rgb { r, g, b } => write!(f, "2::{}:{}:{}", r, g, b),
            CColor::AnsiValue(val) => write!(f, "5:{}", val),
            _ => Ok(()),
        };
        res?;
        write!(f, "m")?;
        Ok(())
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "SetUnderlineColor not supported by winapi.",
        ))
    }
}

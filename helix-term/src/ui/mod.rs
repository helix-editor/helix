mod editor;
mod prompt;

pub use editor::EditorView;
pub use prompt::Prompt;

pub use tui::layout::Rect;
pub use tui::style::{Color, Modifier, Style};

// TODO: temp
#[inline(always)]
pub fn text_color() -> Style {
    Style::default().fg(Color::Rgb(219, 191, 239)) // lilac
}

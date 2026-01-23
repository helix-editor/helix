//! Trust prompt dialog for workspace trust decisions.

use crate::compositor::{Component, Compositor, Context, Event, EventResult};
use helix_view::graphics::Rect;
use helix_view::input::KeyEvent;
use helix_view::keyboard::{KeyCode, KeyModifiers};
use std::path::PathBuf;
use tui::buffer::Buffer as Surface;
use tui::text::Text;
use tui::widgets::{Block, Borders, Paragraph, Widget};

/// Decision made by the user in the trust prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustDecision {
    /// User chose to trust the workspace (y/Y).
    Trust,
    /// User chose not to trust the workspace (n/N).
    Untrust,
    /// User cancelled without making a decision (Esc).
    Cancel,
}

/// A prompt dialog asking the user whether to trust a workspace.
pub struct TrustPrompt {
    workspace_path: PathBuf,
    callback: Option<Box<dyn FnOnce(&mut Compositor, &mut helix_view::Editor, TrustDecision) + Send>>,
}

impl TrustPrompt {
    pub fn new(
        workspace_path: PathBuf,
        callback: impl FnOnce(&mut Compositor, &mut helix_view::Editor, TrustDecision) + Send + 'static,
    ) -> Self {
        Self {
            workspace_path,
            callback: Some(Box::new(callback)),
        }
    }

    fn close_with_decision(&mut self, decision: TrustDecision) -> EventResult {
        let callback = self.callback.take();
        EventResult::Consumed(Some(Box::new(move |compositor: &mut Compositor, cx: &mut Context| {
            // Remove the prompt from the compositor
            compositor.remove(TrustPrompt::ID);
            // Call the callback with the decision
            if let Some(cb) = callback {
                cb(compositor, cx.editor, decision);
            }
        })))
    }

    const ID: &'static str = "trust-prompt";
}

impl Component for TrustPrompt {
    fn render(&mut self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        // Calculate dialog size and position (centered)
        let width = 60.min(area.width.saturating_sub(4));
        let height = 12.min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        let dialog_area = Rect::new(x, y, width, height);

        // Build the dialog content
        let path_display = self.workspace_path.display().to_string();
        let path_str = if path_display.len() > (width as usize - 4) {
            format!(
                "...{}",
                &path_display[path_display.len().saturating_sub(width as usize - 7)..]
            )
        } else {
            path_display
        };

        let text = Text::from(format!(
            "Do you trust the authors of this workspace?\n\n\
             {}\n\n\
             Trusting enables:\n\
              - Language servers (LSP)\n\
              - Shell commands\n\
              - Workspace configuration\n\n\
             [y] Trust   [n] Don't Trust   [Esc] Cancel",
            path_str
        ));

        let theme = &cx.editor.theme;
        let style = theme
            .try_get("ui.popup")
            .unwrap_or_else(|| theme.get("ui.text"));

        // Clear the dialog area with the background style
        for row in dialog_area.top()..dialog_area.bottom() {
            for col in dialog_area.left()..dialog_area.right() {
                if let Some(cell) = surface.get_mut(col, row) {
                    cell.set_style(style);
                    cell.set_char(' ');
                }
            }
        }

        let block = Block::default()
            .title(" Workspace Trust ")
            .borders(Borders::ALL)
            .border_style(style);

        let paragraph = Paragraph::new(&text).block(block).style(style);

        paragraph.render(dialog_area, surface);
    }

    fn handle_event(&mut self, event: &Event, _cx: &mut Context) -> EventResult {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('y' | 'Y'),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            }) => self.close_with_decision(TrustDecision::Trust),

            Event::Key(KeyEvent {
                code: KeyCode::Char('n' | 'N'),
                modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            }) => self.close_with_decision(TrustDecision::Untrust),

            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                ..
            }) => self.close_with_decision(TrustDecision::Cancel),

            // Consume all other events to prevent them from reaching the editor
            Event::Key(_) => EventResult::Consumed(None),
            _ => EventResult::Ignored(None),
        }
    }

    fn id(&self) -> Option<&'static str> {
        Some(Self::ID)
    }
}

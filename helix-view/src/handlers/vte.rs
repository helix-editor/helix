use crate::{terminal::TerminalEvent, Editor};

impl Editor {
    pub async fn handle_virtual_terminal_events(&mut self, _event: TerminalEvent) -> bool {
        true
    }
}

use helix_core::file_watcher::FileSystemDidChange;
use helix_event::{events, register_event};
use helix_view::document::Mode;
use helix_view::events::{
    ConfigDidChange, DiagnosticsDidChange, DocumentDidChange, DocumentDidClose, DocumentDidOpen,
    DocumentFocusLost, LanguageServerExited, LanguageServerInitialized, SelectionDidChange,
};

use crate::commands;
use crate::keymap::MappableCommand;

events! {
    OnModeSwitch<'a, 'cx> { old_mode: Mode, new_mode: Mode, cx: &'a mut commands::Context<'cx> }
    PostInsertChar<'a, 'cx> { c: char, cx: &'a mut commands::Context<'cx> }
    PostCommand<'a, 'cx> { command: & 'a MappableCommand, cx: &'a mut commands::Context<'cx> }
}

pub fn register() {
    register_event::<OnModeSwitch>();
    register_event::<PostInsertChar>();
    register_event::<PostCommand>();
    register_event::<DocumentDidOpen>();
    register_event::<DocumentDidChange>();
    register_event::<DocumentDidClose>();
    register_event::<DocumentFocusLost>();
    register_event::<SelectionDidChange>();
    register_event::<DiagnosticsDidChange>();
    register_event::<LanguageServerInitialized>();
    register_event::<LanguageServerExited>();
    register_event::<ConfigDidChange>();
    register_event::<FileSystemDidChange>();
}

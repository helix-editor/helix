//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


use helix_event::{events, register_event};
use helix_view::document::Mode;
use helix_view::events::{DiagnosticsDidChange, DocumentDidChange, SelectionDidChange};

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
    register_event::<DocumentDidChange>();
    register_event::<SelectionDidChange>();
    register_event::<DiagnosticsDidChange>();
}

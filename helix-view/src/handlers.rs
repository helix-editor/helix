//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


use helix_event::send_blocking;
use tokio::sync::mpsc::Sender;

use crate::handlers::lsp::SignatureHelpInvoked;
use crate::{DocumentId, Editor, ViewId};

pub mod dap;
pub mod diagnostics;
pub mod lsp;

#[derive(Debug)]
pub enum AutoSaveEvent {
    DocumentChanged { save_after: u64 },
    LeftInsertMode,
}

pub struct Handlers {
    // only public because most of the actual implementation is in helix-term right now :/
    pub completions: Sender<lsp::CompletionEvent>,
    pub signature_hints: Sender<lsp::SignatureHelpEvent>,
    pub auto_save: Sender<AutoSaveEvent>,
}

impl Handlers {
    /// Manually trigger completion (c-x)
    pub fn trigger_completions(&self, trigger_pos: usize, doc: DocumentId, view: ViewId) {
        send_blocking(
            &self.completions,
            lsp::CompletionEvent::ManualTrigger {
                cursor: trigger_pos,
                doc,
                view,
            },
        );
    }

    pub fn trigger_signature_help(&self, invocation: SignatureHelpInvoked, editor: &Editor) {
        let event = match invocation {
            SignatureHelpInvoked::Automatic => {
                if !editor.config().lsp.auto_signature_help {
                    return;
                }
                lsp::SignatureHelpEvent::Trigger
            }
            SignatureHelpInvoked::Manual => lsp::SignatureHelpEvent::Invoked,
        };
        send_blocking(&self.signature_hints, event)
    }
}

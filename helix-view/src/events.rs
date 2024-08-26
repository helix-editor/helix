//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


use helix_core::Rope;
use helix_event::events;

use crate::{Document, DocumentId, Editor, ViewId};

events! {
    DocumentDidChange<'a> { doc: &'a mut Document, view: ViewId, old_text: &'a Rope  }
    SelectionDidChange<'a> { doc: &'a mut Document, view: ViewId }
    DiagnosticsDidChange<'a> { editor: &'a mut Editor, doc: DocumentId }
}

use std::path::PathBuf;

use helix_core::unicode::width::UnicodeWidthStr;

use crate::{Document, DocumentId, document::SCRATCH_BUFFER_NAME};

#[derive(Default)]
pub struct BufferLineTabs {
    pub tabs: Vec<Tab>,
    pub scroll: u16,
    pub total_width: u32,
}

pub struct Tab {
    pub document_id: DocumentId,
    pub document_name: String, // I feel like this could just be a reference
    pub modified: bool,
    pub start: u32,
    pub end: u32,
}

impl BufferLineTabs {
    pub fn add_tab(&mut self, document: &Document) {
        let scratch = PathBuf::from(SCRATCH_BUFFER_NAME);

        let file_name = document
            .path()
            .unwrap_or(&scratch)
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        self.tabs.push(Tab {
            document_id: document.id(),
            document_name: file_name.to_string(),
            modified: document.is_modified(),
            start: 0,
            end: 0,
        });
        self.recalculate_tabs(self.tabs.len() - 1);
    }

    pub fn remove_tab(&mut self, document_id: &DocumentId) {
        if let Some(index) = self.tabs.iter().position(|t| t.document_id == *document_id) {
            self.tabs.remove(index);
            self.recalculate_tabs(index);
        }
    }

    pub fn set_modified(&mut self, document_id: DocumentId, modified: bool) {
        let Some(index) = self
            .tabs
            .iter()
            .position(|tab| tab.document_id == document_id)
        else {
            return;
        };
        if self.tabs[index].modified == modified {
            return;
        }
        self.tabs[index].modified = modified;
        self.recalculate_tabs(index);
    }

    pub fn recalculate_tabs(&mut self, starting_index: usize) {
        let mut cursor = starting_index
            .checked_sub(1)
            .and_then(|i| self.tabs.get(i))
            .map_or(0u32, |t| t.end);

        for tab in &mut self.tabs[starting_index..] {
            tab.start = cursor;
            cursor += tab.label().width() as u32;
            tab.end = cursor;
        }

        self.total_width = cursor;
    }
}

impl Tab {
    pub fn label(&self) -> String {
        format!(
            " {}{} ",
            self.document_name,
            if self.modified { "[+]" } else { "" }
        )
    }
}

use std::collections::BTreeMap;

use helix_core::unicode::width::UnicodeWidthStr;

use crate::{Document, DocumentId, document::SCRATCH_BUFFER_NAME};

#[derive(Default)]
pub struct BufferLineTabs {
    pub tabs: Vec<Tab>,
    pub scroll: u16,
}

pub struct Tab {
    pub document_id: DocumentId,
    pub document_name: String,
    pub modified: bool,
    pub start: u32,
    pub end: u32,
}

fn tab_name(document: &Document) -> String {
    document
        .path()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or(SCRATCH_BUFFER_NAME)
        .to_string()
}

impl BufferLineTabs {
    pub fn add_tab(&mut self, document: &Document) {
        self.tabs.push(Tab {
            document_id: document.id(),
            document_name: tab_name(document),
            modified: document.is_modified(),
            start: 0,
            end: 0,
        });
        self.recalculate_tabs(self.tabs.len() - 1);
    }

    pub fn rename_tab(&mut self, document: &Document) {
        let Some(index) = self.index_of(document.id()) else {
            return;
        };
        let name = tab_name(document);
        if self.tabs[index].document_name == name {
            return;
        }
        self.tabs[index].document_name = name;
        self.recalculate_tabs(index);
    }

    pub fn remove_tab(&mut self, document_id: DocumentId) {
        if let Some(index) = self.tabs.iter().position(|t| t.document_id == document_id) {
            self.tabs.remove(index);
            self.recalculate_tabs(index);
        }
    }

    pub fn sync_modified(&mut self, documents: &BTreeMap<DocumentId, Document>) {
        let mut first_changed = None;

        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let Some(doc) = documents.get(&tab.document_id) else {
                continue;
            };

            let modified = doc.is_modified();

            if tab.modified != modified {
                tab.modified = modified;
                first_changed.get_or_insert(index);
            }
        }

        if let Some(index) = first_changed {
            self.recalculate_tabs(index);
        }
    }

    pub fn total_width(&self) -> u32 {
        self.tabs.last().map_or(0, |t| t.end)
    }

    fn index_of(&self, document_id: DocumentId) -> Option<usize> {
        self.tabs.iter().position(|t| t.document_id == document_id)
    }

    fn recalculate_tabs(&mut self, starting_index: usize) {
        let mut cursor = starting_index
            .checked_sub(1)
            .and_then(|i| self.tabs.get(i))
            .map_or(0u32, |t| t.end);

        for tab in &mut self.tabs[starting_index..] {
            tab.start = cursor;
            cursor += tab.label().width() as u32;
            tab.end = cursor;
        }
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

use std::path::Path;

use super::TestCaseSpec;
use crate::test::helpers::TestApplication;
use helix_core::path::get_normalized_path;
use helix_view::doc;

pub struct ValidationContext<'a> {
    pub spec: &'a TestCaseSpec,
    pub app: &'a TestApplication,
}

impl<'a> ValidationContext<'a> {
    pub fn assert_eq_text_current(&self) {
        assert_eq!(&self.spec.expected.text, doc!(self.app.editor).text());
    }

    pub fn assert_eq_selection(&self) {
        let mut selections: Vec<_> = doc!(self.app.editor)
            .selections()
            .values()
            .cloned()
            .collect();
        assert_eq!(1, selections.len());

        let sel = selections.pop().unwrap();
        assert_eq!(self.spec.expected.selection, sel);
    }

    pub fn assert_view_count(&self, expected_count: usize) {
        assert_eq!(expected_count, self.app.editor.tree.views().count());
    }

    pub fn assert_document_count(&self, expected_count: usize) {
        assert_eq!(expected_count, self.app.editor.documents.len());
    }

    pub fn assert_eq_document_path<P: AsRef<Path>>(&self, path: P) {
        assert_eq!(
            &get_normalized_path(path.as_ref()),
            doc!(self.app.editor).path().unwrap()
        );
    }

    pub fn newest_doc_is_modified(&self) -> bool {
        self.app
            .editor
            .documents
            .values()
            .last()
            .unwrap()
            .is_modified()
    }

    pub fn assert_app_is_err(&self) {
        assert!(self.app.editor.is_err());
    }

    pub fn assert_app_is_ok(&self) {
        assert!(
            !self.app.editor.is_err(),
            "error: {:?}",
            self.app.editor.get_status()
        );
    }
}

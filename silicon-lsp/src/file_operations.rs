use std::path::Path;

use globset::{GlobBuilder, GlobSet};

use crate::lsp;

#[derive(Default, Debug)]
pub(crate) struct FileOperationFilter {
    dir_globs: GlobSet,
    file_globs: GlobSet,
}

impl FileOperationFilter {
    fn new(capability: Option<&lsp::FileOperationRegistrationOptions>) -> FileOperationFilter {
        let Some(cap) = capability else {
            return FileOperationFilter::default();
        };
        let mut dir_globs = GlobSet::builder();
        let mut file_globs = GlobSet::builder();
        for filter in &cap.filters {
            // TODO: support other url schemes
            let is_non_file_schema = filter
                .scheme
                .as_ref()
                .is_some_and(|schema| schema != "file");
            if is_non_file_schema {
                continue;
            }
            let ignore_case = filter
                .pattern
                .options
                .as_ref()
                .and_then(|opts| opts.ignore_case)
                .unwrap_or(false);
            let mut glob_builder = GlobBuilder::new(&filter.pattern.glob);
            glob_builder.case_insensitive(!ignore_case);
            let glob = match glob_builder.build() {
                Ok(glob) => glob,
                Err(err) => {
                    log::error!("invalid glob send by LS: {err}");
                    continue;
                }
            };
            match filter.pattern.matches {
                Some(lsp::FileOperationPatternKind::File) => {
                    file_globs.add(glob);
                }
                Some(lsp::FileOperationPatternKind::Folder) => {
                    dir_globs.add(glob);
                }
                None => {
                    file_globs.add(glob.clone());
                    dir_globs.add(glob);
                }
            };
        }
        let file_globs = file_globs.build().unwrap_or_else(|err| {
            log::error!("invalid globs send by LS: {err}");
            GlobSet::empty()
        });
        let dir_globs = dir_globs.build().unwrap_or_else(|err| {
            log::error!("invalid globs send by LS: {err}");
            GlobSet::empty()
        });
        FileOperationFilter {
            dir_globs,
            file_globs,
        }
    }

    pub(crate) fn has_interest(&self, path: &Path, is_dir: bool) -> bool {
        if is_dir {
            self.dir_globs.is_match(path)
        } else {
            self.file_globs.is_match(path)
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct FileOperationsInterest {
    // TODO: support other notifications
    // did_create: FileOperationFilter,
    // will_create: FileOperationFilter,
    pub did_rename: FileOperationFilter,
    pub will_rename: FileOperationFilter,
    // did_delete: FileOperationFilter,
    // will_delete: FileOperationFilter,
}

impl FileOperationsInterest {
    pub fn new(capabilities: &lsp::ServerCapabilities) -> FileOperationsInterest {
        let capabilities = capabilities
            .workspace
            .as_ref()
            .and_then(|capabilities| capabilities.file_operations.as_ref());
        let Some(capabilities) = capabilities else {
            return FileOperationsInterest::default();
        };
        FileOperationsInterest {
            did_rename: FileOperationFilter::new(capabilities.did_rename.as_ref()),
            will_rename: FileOperationFilter::new(capabilities.will_rename.as_ref()),
        }
    }
}

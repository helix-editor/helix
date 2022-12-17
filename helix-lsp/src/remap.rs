use std::path::Path;

use lsp_types::Url;

pub trait LSPRemap {
    fn remap(&self, from: &str, to: &str) -> Self;
}

impl LSPRemap for Url {
    fn remap(&self, from: &str, to: &str) -> Self {
        let path = self.to_file_path().ok();

        path.and_then(|p| {
            let replaced = p
                .strip_prefix(from)
                .map_or(p.clone(), |stripped| Path::new(to).join(stripped));
            Self::from_file_path(replaced).ok()
        })
        .unwrap_or(self.clone())
    }
}

use std::{path::Path, sync::Arc, time::Duration};

use helix_event::AsyncHook;
use tokio::time::Instant;

use crate::{
    job,
    ui::{menu::Item, overlay::Overlay},
};

use super::{CachedPreview, DynamicPicker, Picker};

pub(super) struct PreviewHighlightHandler<T: Item> {
    trigger: Option<Arc<Path>>,
    phantom_data: std::marker::PhantomData<T>,
}

impl<T: Item> Default for PreviewHighlightHandler<T> {
    fn default() -> Self {
        Self {
            trigger: None,
            phantom_data: Default::default(),
        }
    }
}

impl<T: Item> AsyncHook for PreviewHighlightHandler<T> {
    type Event = Arc<Path>;

    fn handle_event(
        &mut self,
        path: Self::Event,
        timeout: Option<tokio::time::Instant>,
    ) -> Option<tokio::time::Instant> {
        if self
            .trigger
            .as_ref()
            .is_some_and(|trigger| trigger == &path)
        {
            // If the path hasn't changed, don't reset the debounce
            timeout
        } else {
            self.trigger = Some(path);
            Some(Instant::now() + Duration::from_millis(150))
        }
    }

    fn finish_debounce(&mut self) {
        let Some(path) = self.trigger.take() else {
            return;
        };

        job::dispatch_blocking(move |editor, compositor| {
            let picker = match compositor.find::<Overlay<Picker<T>>>() {
                Some(Overlay { content, .. }) => content,
                None => match compositor.find::<Overlay<DynamicPicker<T>>>() {
                    Some(Overlay { content, .. }) => &mut content.file_picker,
                    None => return,
                },
            };

            let Some(CachedPreview::Document(ref mut doc)) = picker.preview_cache.get_mut(&path)
            else {
                return;
            };

            if doc.language_config().is_some() {
                return;
            }

            let Some(language_config) = doc.detect_language_config(&editor.syn_loader.load())
            else {
                return;
            };
            doc.language = Some(language_config.clone());
            let text = doc.text().clone();
            let loader = editor.syn_loader.clone();

            tokio::task::spawn_blocking(move || {
                let Some(syntax) = language_config
                    .highlight_config(&loader.load().scopes())
                    .and_then(|highlight_config| {
                        helix_core::Syntax::new(text.slice(..), highlight_config, loader)
                    })
                else {
                    log::info!("highlighting picker item failed");
                    return;
                };

                job::dispatch_blocking(move |editor, compositor| {
                    let picker = match compositor.find::<Overlay<Picker<T>>>() {
                        Some(Overlay { content, .. }) => Some(content),
                        None => compositor
                            .find::<Overlay<DynamicPicker<T>>>()
                            .map(|overlay| &mut overlay.content.file_picker),
                    };
                    let Some(picker) = picker else {
                        log::info!("picker closed before syntax highlighting finished");
                        return;
                    };
                    let Some(CachedPreview::Document(ref mut doc)) =
                        picker.preview_cache.get_mut(&path)
                    else {
                        return;
                    };
                    let diagnostics = helix_view::Editor::doc_diagnostics(
                        &editor.language_servers,
                        &editor.diagnostics,
                        doc,
                    );
                    doc.replace_diagnostics(diagnostics, &[], None);
                    doc.syntax = Some(syntax);
                });
            });
        });
    }
}

use std::{
    sync::{atomic, Arc},
    time::Duration,
};

use helix_event::AsyncHook;
use tokio::time::Instant;

use crate::{job, ui::overlay::Overlay};

use super::{DynQueryCallback, Picker};

pub(super) struct DynamicQueryChange {
    pub query: Arc<str>,
    pub is_paste: bool,
}

pub(super) struct DynamicQueryHandler<T: 'static + Send + Sync, D: 'static + Send + Sync> {
    callback: Arc<DynQueryCallback<T, D>>,
    // Duration used as a debounce.
    // Defaults to 100ms if not provided via `Picker::with_dynamic_query`. Callers may want to set
    // this higher if the dynamic query is expensive - for example global search.
    debounce: Duration,
    last_query: Arc<str>,
    query: Option<Arc<str>>,
}

impl<T: 'static + Send + Sync, D: 'static + Send + Sync> DynamicQueryHandler<T, D> {
    pub(super) fn new(callback: DynQueryCallback<T, D>, duration_ms: Option<u64>) -> Self {
        Self {
            callback: Arc::new(callback),
            debounce: Duration::from_millis(duration_ms.unwrap_or(100)),
            last_query: "".into(),
            query: None,
        }
    }
}

impl<T: 'static + Send + Sync, D: 'static + Send + Sync> AsyncHook for DynamicQueryHandler<T, D> {
    type Event = DynamicQueryChange;

    fn handle_event(&mut self, change: Self::Event, _timeout: Option<Instant>) -> Option<Instant> {
        let DynamicQueryChange { query, is_paste } = change;
        if query == self.last_query {
            // If the search query reverts to the last one we requested, no need to
            // make a new request.
            self.query = None;
            None
        } else {
            self.query = Some(query);
            if is_paste {
                self.finish_debounce();
                None
            } else {
                Some(Instant::now() + self.debounce)
            }
        }
    }

    fn finish_debounce(&mut self) {
        let Some(query) = self.query.take() else {
            return;
        };
        self.last_query = query.clone();
        let callback = self.callback.clone();

        job::dispatch_blocking(move |editor, compositor| {
            let Some(Overlay {
                content: picker, ..
            }) = compositor.find::<Overlay<Picker<T, D>>>()
            else {
                return;
            };
            // Increment the version number to cancel any ongoing requests.
            picker.version.fetch_add(1, atomic::Ordering::Relaxed);
            picker.matcher.restart(false);
            let injector = picker.injector();
            let get_options = (callback)(&query, editor, picker.editor_data.clone(), &injector);
            tokio::spawn(async move {
                if let Err(err) = get_options.await {
                    log::info!("Dynamic request failed: {err}");
                }
                // NOTE: the Drop implementation of Injector will request a redraw when the
                // injector falls out of scope here, clearing the "running" indicator.
            });
        })
    }
}

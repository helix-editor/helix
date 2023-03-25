use crate::compositor::{Component, Compositor};
use helix_view::Editor;
use std::pin::Pin;
use std::task::Poll;
use tokio::sync::watch;

use futures_util::future::{Future, FutureExt};
use futures_util::stream::{FuturesUnordered, StreamExt};

pub type EditorCompositorCallback = Box<dyn FnOnce(&mut Editor, &mut Compositor)>;
pub type EditorCallback = Box<dyn FnOnce(&mut Editor)>;

pub enum Callback {
    EditorCompositor(EditorCompositorCallback),
    Editor(EditorCallback),
}

pub type JobFuture = Pin<Box<dyn Future<Output = anyhow::Result<Option<Callback>>>>>;

pub struct Job {
    pub future: JobFuture,
    /// Do we need to wait for this job to finish before exiting?
    pub wait: bool,
}

pub fn cancelation() -> (CancelSender, CancelReciver) {
    let (sender, reciver) = watch::channel(());
    (CancelSender(sender), CancelReciver(reciver))
}

#[derive(Debug)]
pub struct CancelSender(watch::Sender<()>);

/// A cancel flag that can be awaited in an async context
/// and cheaply checked with a non-blocking check for use in
/// a synchronous call
// using a watch intead of a Notify so we can implement is_cancelled
#[derive(Clone, Debug)]
pub struct CancelReciver(watch::Receiver<()>);

impl CancelReciver {
    pub fn is_cancelled(&self) -> bool {
        !matches!(self.0.has_changed(), Ok(false))
    }

    pub async fn canceled(mut self) {
        let _ = self.0.changed().await;
    }
}

/// A Blocking Job is a job that would normally be executed synchronously
/// and block the UI thread but is performed asynchrounsly instead so that:
/// * The UI doesn't freeze (when resizing the window for example)
/// * We don't perform blocking tasks on a normal tokio thread
/// * The user can cancel an unresponsive task with C-c
pub struct BlockingJob {
    future: JobFuture,
    // When a BlockingJob is dropped all watchers are notified by the Drop
    // implementation of watch::channel.
    cancel: CancelSender,
    pub msg: &'static str,
}

impl BlockingJob {
    pub fn new<F: Future<Output = anyhow::Result<Callback>> + 'static>(
        f: F,
        cancel: CancelSender,
        msg: &'static str,
    ) -> BlockingJob {
        BlockingJob {
            future: Box::pin(f.map(|r| r.map(Some))),
            cancel,
            msg,
        }
    }

    pub fn push_layer<F: Future<Output = anyhow::Result<Box<dyn Component>>> + 'static>(
        layer: F,
        cancel: CancelSender,
        msg: &'static str,
    ) -> BlockingJob {
        BlockingJob::new(
            async {
                let layer = layer.await?;
                let callback =
                    Callback::EditorCompositor(Box::new(move |_, compositor: &mut Compositor| {
                        compositor.push(layer)
                    }));
                Ok(callback)
            },
            cancel,
            msg,
        )
    }

    pub fn non_blocking(self) -> Job {
        Job {
            future: Box::pin(self.future.map(move |res| {
                drop(self.cancel);
                res
            })),
            wait: false,
        }
    }

    pub fn cancel(&self) {
        let _ = self.cancel.0.send(());
    }
}

impl Future for BlockingJob {
    type Output = anyhow::Result<Option<Callback>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.future.poll_unpin(cx)
    }
}

#[derive(Default)]
pub struct Jobs {
    pub futures: FuturesUnordered<JobFuture>,
    /// These are the ones that need to complete before we exit.
    pub wait_futures: FuturesUnordered<JobFuture>,
    pub blocking_job: Option<BlockingJob>,
}

impl Job {
    pub fn new<F: Future<Output = anyhow::Result<()>> + 'static>(f: F) -> Self {
        Self {
            future: Box::pin(f.map(|r| r.map(|()| None))),
            wait: false,
        }
    }

    pub fn with_callback<F: Future<Output = anyhow::Result<Callback>> + 'static>(f: F) -> Self {
        Self {
            future: Box::pin(f.map(|r| r.map(Some))),
            wait: false,
        }
    }

    pub fn wait_before_exiting(mut self) -> Self {
        self.wait = true;
        self
    }
}

impl Jobs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn<F: Future<Output = anyhow::Result<()>> + 'static>(&mut self, f: F) {
        self.add(Job::new(f));
    }

    pub fn callback<F: Future<Output = anyhow::Result<Callback>> + 'static>(&mut self, f: F) {
        self.add(Job::with_callback(f));
    }

    pub fn handle_callback(
        &self,
        editor: &mut Editor,
        compositor: &mut Compositor,
        call: anyhow::Result<Option<Callback>>,
    ) {
        match call {
            Ok(None) => {}
            Ok(Some(call)) => match call {
                Callback::EditorCompositor(call) => call(editor, compositor),
                Callback::Editor(call) => call(editor),
            },
            Err(e) => {
                editor.set_error(format!("Async job failed: {}", e));
            }
        }
    }

    pub async fn next_job(&mut self) -> Option<anyhow::Result<Option<Callback>>> {
        tokio::select! {
            event = self.futures.next() => {  event }
            event = self.wait_futures.next() => { event }
        }
    }

    pub fn add(&self, j: Job) {
        if j.wait {
            self.wait_futures.push(j.future);
        } else {
            self.futures.push(j.future);
        }
    }

    /// Blocks until all the jobs that need to be waited on are done.
    pub async fn finish(
        &mut self,
        editor: &mut Editor,
        mut compositor: Option<&mut Compositor>,
    ) -> anyhow::Result<()> {
        log::debug!("waiting on jobs...");
        let mut wait_futures = std::mem::take(&mut self.wait_futures);

        while let (Some(job), tail) = wait_futures.into_future().await {
            match job {
                Ok(callback) => {
                    wait_futures = tail;

                    if let Some(callback) = callback {
                        // clippy doesn't realize this is an error without the derefs
                        #[allow(clippy::needless_option_as_deref)]
                        match callback {
                            Callback::EditorCompositor(call) if compositor.is_some() => {
                                call(editor, compositor.as_deref_mut().unwrap())
                            }
                            Callback::Editor(call) => call(editor),

                            // skip callbacks for which we don't have the necessary references
                            _ => (),
                        }
                    }
                }
                Err(e) => {
                    self.wait_futures = tail;
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

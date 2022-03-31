use helix_view::Editor;

use crate::compositor::Compositor;

use futures_util::future::{BoxFuture, Future, FutureExt};
use futures_util::stream::{FuturesUnordered, StreamExt};

pub type Callback = Box<dyn FnOnce(&mut Editor, &mut Compositor) + Send>;
pub type JobFuture = BoxFuture<'static, anyhow::Result<Option<Callback>>>;

pub struct Job {
    pub future: BoxFuture<'static, anyhow::Result<Option<Callback>>>,
    /// Do we need to wait for this job to finish before exiting?
    pub wait: bool,
}

#[derive(Default)]
pub struct Jobs {
    pub futures: FuturesUnordered<JobFuture>,
    /// These are the ones that need to complete before we exit.
    pub wait_futures: FuturesUnordered<JobFuture>,
}

impl Job {
    pub fn new<F: Future<Output = anyhow::Result<()>> + Send + 'static>(f: F) -> Self {
        Self {
            future: f.map(|r| r.map(|()| None)).boxed(),
            wait: false,
        }
    }

    pub fn with_callback<F: Future<Output = anyhow::Result<Callback>> + Send + 'static>(
        f: F,
    ) -> Self {
        Self {
            future: f.map(|r| r.map(Some)).boxed(),
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

    pub fn spawn<F: Future<Output = anyhow::Result<()>> + Send + 'static>(&mut self, f: F) {
        self.add(Job::new(f));
    }

    pub fn callback<F: Future<Output = anyhow::Result<Callback>> + Send + 'static>(
        &mut self,
        f: F,
    ) {
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
            Ok(Some(call)) => {
                call(editor, compositor);
            }
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
    pub async fn finish(&mut self) -> anyhow::Result<()> {
        log::debug!("waiting on jobs...");
        let mut wait_futures = std::mem::take(&mut self.wait_futures);
        while let (Some(job), tail) = wait_futures.into_future().await {
            match job {
                Ok(_) => {
                    wait_futures = tail;
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

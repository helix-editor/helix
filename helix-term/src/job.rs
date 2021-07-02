use helix_view::Editor;

use crate::compositor::Compositor;

use futures_util::future::{self, BoxFuture, Future, FutureExt};
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
    futures: FuturesUnordered<JobFuture>,
    /// These are the ones that need to complete before we exit.
    wait_futures: FuturesUnordered<JobFuture>,
}

impl Job {
    pub fn new<F: Future<Output = anyhow::Result<()>> + Send + 'static>(f: F) -> Job {
        Job {
            future: f.map(|r| r.map(|()| None)).boxed(),
            wait: false,
        }
    }

    pub fn with_callback<F: Future<Output = anyhow::Result<Callback>> + Send + 'static>(
        f: F,
    ) -> Job {
        Job {
            future: f.map(|r| r.map(Some)).boxed(),
            wait: false,
        }
    }

    pub fn wait_before_exiting(mut self) -> Job {
        self.wait = true;
        self
    }
}

impl Jobs {
    pub fn new() -> Jobs {
        Jobs::default()
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
        &mut self,
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

    pub fn next_job(
        &mut self,
    ) -> impl Future<Output = Option<anyhow::Result<Option<Callback>>>> + '_ {
        future::select(self.futures.next(), self.wait_futures.next())
            .map(|either| either.factor_first().0)
    }

    pub fn add(&mut self, j: Job) {
        if j.wait {
            self.wait_futures.push(j.future);
        } else {
            self.futures.push(j.future);
        }
    }

    /// Blocks until all the jobs that need to be waited on are done.
    pub fn finish(&mut self) {
        let wait_futures = std::mem::take(&mut self.wait_futures);
        helix_lsp::block_on(wait_futures.for_each(|_| future::ready(())));
    }
}

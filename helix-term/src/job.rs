use helix_view::Editor;

use crate::compositor::Compositor;

use futures_util::future::{self, BoxFuture, FusedFuture, Future, FutureExt};
use futures_util::stream::{FusedStream, FuturesUnordered, Stream, StreamExt};

use std::pin::Pin;
use std::task::{self, Poll};

pub type Callback = Box<dyn FnOnce(&mut Editor, &mut Compositor) + Send>;
pub type JobFuture = BoxFuture<'static, anyhow::Result<Option<Callback>>>;

/// A wrapper around two streams, yielding from either one.
///
/// It would be nice to achieve the same effect by combining adapters from `futures_util`, but:
/// - `stream::Select` takes ownership of the streams and doesn't seem to work right if we modify
///    the streams afterwards.
/// - `stream::Next` combined with `future::Select` doesn't do what we want when one of the streams
///    is empty: it will return straight away but we want to wait for the other stream.
/// - `stream::SelectNextSome` panics if it's polled after completing, and that seems to be hard to
///    avoid when using it in conjuction with other futures adapters.
///
/// This implementation has the same `FusedFuture` conventions as `SelectNextSome`:
/// if the streams are empty it will always return `Pending`, but it will still return `true`
/// from `is_terminated`. Apparently this behavior is useful for `tokio::select!`.
struct SelectNext<'a, St1, St2> {
    st1: &'a mut St1,
    st2: &'a mut St2,
    // Should we poll st1 or st2? We toggle this at every poll to avoid starvation.
    first: bool,
}

impl<'a, St1, St2> Unpin for SelectNext<'a, St1, St2> {}

impl<'a, T, St1: Stream<Item = T> + Unpin, St2: Stream<Item = T> + Unpin> Future
    for SelectNext<'a, St1, St2>
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<T> {
        self.first = !self.first;
        if self.first {
            if let Poll::Ready(Some(x)) = self.st1.poll_next_unpin(cx) {
                Poll::Ready(x)
            } else if let Poll::Ready(Some(x)) = self.st2.poll_next_unpin(cx) {
                Poll::Ready(x)
            } else {
                Poll::Pending
            }
        } else {
            if let Poll::Ready(Some(x)) = self.st2.poll_next_unpin(cx) {
                Poll::Ready(x)
            } else if let Poll::Ready(Some(x)) = self.st1.poll_next_unpin(cx) {
                Poll::Ready(x)
            } else {
                Poll::Pending
            }
        }
    }
}

impl<'a, T, St1: FusedStream<Item = T> + Unpin, St2: FusedStream<Item = T> + Unpin> FusedFuture
    for SelectNext<'a, St1, St2>
{
    fn is_terminated(&self) -> bool {
        self.st1.is_terminated() && self.st2.is_terminated()
    }
}

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

    pub fn next_job(&mut self) -> impl Future<Output = anyhow::Result<Option<Callback>>> + '_ {
        SelectNext {
            st1: &mut self.futures,
            st2: &mut self.wait_futures,
            first: true,
        }
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

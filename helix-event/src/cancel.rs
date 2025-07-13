use std::borrow::Borrow;
use std::future::Future;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use tokio::sync::Notify;

pub async fn cancelable_future<T>(
    future: impl Future<Output = T>,
    cancel: impl Borrow<TaskHandle>,
) -> Option<T> {
    tokio::select! {
        biased;
        _ = cancel.borrow().canceled() => {
            None
        }
        res = future => {
            Some(res)
        }
    }
}

#[derive(Default, Debug)]
struct Shared {
    state: AtomicU64,
    // `Notify` has some features that we don't really need here because it
    // supports waking single tasks (`notify_one`) and does its own (more
    // complicated) state tracking, we could reimplement the waiter linked list
    // with modest effort and reduce memory consumption by one word/8 bytes and
    // reduce code complexity/number of atomic operations.
    //
    // I don't think that's worth the complexity (unsafe code).
    //
    // if we only cared about async code then we could also only use a notify
    // (without the generation count), this would be equivalent (or maybe more
    // correct if we want to allow cloning the TX) but it would be extremly slow
    // to frequently check for cancelation from sync code
    notify: Notify,
}

impl Shared {
    fn generation(&self) -> u32 {
        self.state.load(Relaxed) as u32
    }

    fn num_running(&self) -> u32 {
        (self.state.load(Relaxed) >> 32) as u32
    }

    /// Increments the generation count and sets `num_running`
    /// to the provided value, this operation is not with
    /// regard to the generation counter (doesn't use `fetch_add`)
    /// so the calling code must ensure it cannot execute concurrently
    /// to maintain correctness (but not safety)
    fn inc_generation(&self, num_running: u32) -> (u32, u32) {
        let state = self.state.load(Relaxed);
        let generation = state as u32;
        let prev_running = (state >> 32) as u32;
        // no need to create a new generation if the refcount is zero (fastpath)
        if prev_running == 0 && num_running == 0 {
            return (generation, 0);
        }
        let new_generation = generation.saturating_add(1);
        self.state.store(
            new_generation as u64 | ((num_running as u64) << 32),
            Relaxed,
        );
        self.notify.notify_waiters();
        (new_generation, prev_running)
    }

    fn inc_running(&self, generation: u32) {
        let mut state = self.state.load(Relaxed);
        loop {
            let current_generation = state as u32;
            if current_generation != generation {
                break;
            }
            let off = 1 << 32;
            let res = self.state.compare_exchange_weak(
                state,
                state.saturating_add(off),
                Relaxed,
                Relaxed,
            );
            match res {
                Ok(_) => break,
                Err(new_state) => state = new_state,
            }
        }
    }

    fn dec_running(&self, generation: u32) {
        let mut state = self.state.load(Relaxed);
        loop {
            let current_generation = state as u32;
            if current_generation != generation {
                break;
            }
            let num_running = (state >> 32) as u32;
            // running can't be zero here, that would mean we miscounted somewhere
            assert_ne!(num_running, 0);
            let off = 1 << 32;
            let res = self
                .state
                .compare_exchange_weak(state, state - off, Relaxed, Relaxed);
            match res {
                Ok(_) => break,
                Err(new_state) => state = new_state,
            }
        }
    }
}

// This intentionally doesn't implement `Clone` and requires a mutable reference
// for cancelation to avoid races (in inc_generation).

/// A task controller allows managing a single subtask enabling the controller
/// to cancel the subtask and to check whether it is still running.
///
/// For efficiency reasons the controller can be reused/restarted,
/// in that case the previous task is automatically canceled.
///
/// If the controller is dropped, the subtasks are automatically canceled.
#[derive(Default, Debug)]
pub struct TaskController {
    shared: Arc<Shared>,
}

impl TaskController {
    pub fn new() -> Self {
        TaskController::default()
    }
    /// Cancels the active task (handle).
    ///
    /// Returns whether any tasks were still running before the cancelation.
    pub fn cancel(&mut self) -> bool {
        self.shared.inc_generation(0).1 != 0
    }

    /// Checks whether there are any task handles
    /// that haven't been dropped (or canceled) yet.
    pub fn is_running(&self) -> bool {
        self.shared.num_running() != 0
    }

    /// Starts a new task and cancels the previous task (handles).
    pub fn restart(&mut self) -> TaskHandle {
        TaskHandle {
            generation: self.shared.inc_generation(1).0,
            shared: self.shared.clone(),
        }
    }
}

impl Drop for TaskController {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// A handle that is used to link a task with a task controller.
///
/// It can be used to cancel async futures very efficiently but can also be checked for
/// cancelation very quickly (single atomic read) in blocking code.
/// The handle can be cheaply cloned (reference counted).
///
/// The TaskController can check whether a task is "running" by inspecting the
/// refcount of the (current) tasks handles. Therefore, if that information
/// is important, ensure that the handle is not dropped until the task fully
/// completes.
pub struct TaskHandle {
    shared: Arc<Shared>,
    generation: u32,
}

impl Clone for TaskHandle {
    fn clone(&self) -> Self {
        self.shared.inc_running(self.generation);
        TaskHandle {
            shared: self.shared.clone(),
            generation: self.generation,
        }
    }
}

impl Drop for TaskHandle {
    fn drop(&mut self) {
        self.shared.dec_running(self.generation);
    }
}

impl TaskHandle {
    /// Waits until [`TaskController::cancel`] is called for the corresponding
    /// [`TaskController`]. Immediately returns if `cancel` was already called since
    pub async fn canceled(&self) {
        let notified = self.shared.notify.notified();
        if !self.is_canceled() {
            notified.await
        }
    }

    pub fn is_canceled(&self) -> bool {
        self.generation != self.shared.generation()
    }
}

#[cfg(test)]
mod tests {
    use std::future::poll_fn;

    use futures_executor::block_on;
    use tokio::task::yield_now;

    use crate::{cancelable_future, TaskController};

    #[test]
    fn immediate_cancel() {
        let mut controller = TaskController::new();
        let handle = controller.restart();
        controller.cancel();
        assert!(handle.is_canceled());
        controller.restart();
        assert!(handle.is_canceled());

        let res = block_on(cancelable_future(
            poll_fn(|_cx| std::task::Poll::Ready(())),
            handle,
        ));
        assert!(res.is_none());
    }

    #[test]
    fn running_count() {
        let mut controller = TaskController::new();
        let handle = controller.restart();
        assert!(controller.is_running());
        assert!(!handle.is_canceled());
        drop(handle);
        assert!(!controller.is_running());
        assert!(!controller.cancel());
        let handle = controller.restart();
        assert!(!handle.is_canceled());
        assert!(controller.is_running());
        let handle2 = handle.clone();
        assert!(!handle.is_canceled());
        assert!(controller.is_running());
        drop(handle2);
        assert!(!handle.is_canceled());
        assert!(controller.is_running());
        assert!(controller.cancel());
        assert!(handle.is_canceled());
        assert!(!controller.is_running());
    }

    #[test]
    fn no_cancel() {
        let mut controller = TaskController::new();
        let handle = controller.restart();
        assert!(!handle.is_canceled());

        let res = block_on(cancelable_future(
            poll_fn(|_cx| std::task::Poll::Ready(())),
            handle,
        ));
        assert!(res.is_some());
    }

    #[test]
    fn delayed_cancel() {
        let mut controller = TaskController::new();
        let handle = controller.restart();

        let mut hit = false;
        let res = block_on(cancelable_future(
            async {
                controller.cancel();
                hit = true;
                yield_now().await;
            },
            handle,
        ));
        assert!(res.is_none());
        assert!(hit);
    }
}

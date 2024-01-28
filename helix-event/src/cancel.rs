use std::future::Future;

pub use oneshot::channel as cancelation;
use tokio::sync::oneshot;

pub type CancelTx = oneshot::Sender<()>;
pub type CancelRx = oneshot::Receiver<()>;

pub async fn cancelable_future<T>(future: impl Future<Output = T>, cancel: CancelRx) -> Option<T> {
    tokio::select! {
        biased;
        _ = cancel => {
            None
        }
        res = future => {
            Some(res)
        }
    }
}

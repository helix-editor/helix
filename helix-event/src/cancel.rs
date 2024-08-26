//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


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

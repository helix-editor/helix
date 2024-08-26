//!
//! This Source Code Form is subject to the terms of the Mozilla Public
//! License, v. 2.0. If a copy of the MPL was not distributed with this
//! file, You can find the complete license text at
//! https://mozilla.org/MPL/2.0/
//!
//! Copyright (c) 2024 Helix Editor Contributors


mod client;
mod transport;
mod types;

pub use client::{Client, ConnectionType};
pub use events::Event;
pub use transport::{Payload, Response, Transport};
pub use types::*;

use thiserror::Error;
#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("request {0} timed out")]
    Timeout(u64),
    #[error("server closed the stream")]
    StreamClosed,
    #[error(transparent)]
    ExecutableNotFound(#[from] helix_stdx::env::ExecutableNotFoundError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
pub type Result<T> = core::result::Result<T, Error>;

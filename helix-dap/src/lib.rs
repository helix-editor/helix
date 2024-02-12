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

mod client;
mod transport;

pub use client::{Breakpoint, Client, OutputEventBody, SourceBreakpoint, StoppedEventBody};
pub use transport::{Event, Payload, Request, Response, Transport};

use thiserror::Error;
#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to parse: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("request timed out")]
    Timeout,
    #[error("server closed the stream")]
    StreamClosed,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
pub type Result<T> = core::result::Result<T, Error>;

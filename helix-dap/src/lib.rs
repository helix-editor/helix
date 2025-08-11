mod client;
pub mod registry;
mod transport;
mod types;

pub use client::Client;
pub use transport::{Payload, Response, Transport};
pub use types::*;

use serde::de::DeserializeOwned;

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
    #[error("Unhandled")]
    Unhandled,
    #[error(transparent)]
    ExecutableNotFound(#[from] helix_stdx::env::ExecutableNotFoundError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Request {
    RunInTerminal(<requests::RunInTerminal as types::Request>::Arguments),
    StartDebugging(<requests::StartDebugging as types::Request>::Arguments),
}

impl Request {
    pub fn parse(command: &str, arguments: Option<serde_json::Value>) -> Result<Self> {
        use crate::types::Request as _;

        let arguments = arguments.unwrap_or_default();
        let request = match command {
            requests::RunInTerminal::COMMAND => Self::RunInTerminal(parse_value(arguments)?),
            requests::StartDebugging::COMMAND => Self::StartDebugging(parse_value(arguments)?),
            _ => return Err(Error::Unhandled),
        };

        Ok(request)
    }
}

#[derive(Debug)]
pub enum Event {
    Initialized(<events::Initialized as events::Event>::Body),
    Stopped(<events::Stopped as events::Event>::Body),
    Continued(<events::Continued as events::Event>::Body),
    Exited(<events::Exited as events::Event>::Body),
    Terminated(<events::Terminated as events::Event>::Body),
    Thread(<events::Thread as events::Event>::Body),
    Output(<events::Output as events::Event>::Body),
    Breakpoint(<events::Breakpoint as events::Event>::Body),
    Module(<events::Module as events::Event>::Body),
    LoadedSource(<events::LoadedSource as events::Event>::Body),
    Process(<events::Process as events::Event>::Body),
    Capabilities(<events::Capabilities as events::Event>::Body),
    // ProgressStart(),
    // ProgressUpdate(),
    // ProgressEnd(),
    // Invalidated(),
    Memory(<events::Memory as events::Event>::Body),
}

impl Event {
    pub fn parse(event: &str, body: Option<serde_json::Value>) -> Result<Self> {
        use crate::events::Event as _;

        let body = body.unwrap_or_default();
        let event = match event {
            events::Initialized::EVENT => Self::Initialized(parse_value(body)?),
            events::Stopped::EVENT => Self::Stopped(parse_value(body)?),
            events::Continued::EVENT => Self::Continued(parse_value(body)?),
            events::Exited::EVENT => Self::Exited(parse_value(body)?),
            events::Terminated::EVENT => Self::Terminated(parse_value(body)?),
            events::Thread::EVENT => Self::Thread(parse_value(body)?),
            events::Output::EVENT => Self::Output(parse_value(body)?),
            events::Breakpoint::EVENT => Self::Breakpoint(parse_value(body)?),
            events::Module::EVENT => Self::Module(parse_value(body)?),
            events::LoadedSource::EVENT => Self::LoadedSource(parse_value(body)?),
            events::Process::EVENT => Self::Process(parse_value(body)?),
            events::Capabilities::EVENT => Self::Capabilities(parse_value(body)?),
            events::Memory::EVENT => Self::Memory(parse_value(body)?),
            _ => return Err(Error::Unhandled),
        };

        Ok(event)
    }
}

fn parse_value<T>(value: serde_json::Value) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).map_err(|err| err.into())
}

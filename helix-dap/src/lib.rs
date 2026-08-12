mod client;
pub mod registry;
mod transport;

pub use client::Client;
pub use helix_dap_types::*;
pub use transport::{Payload, Response, Transport};

use serde::de::DeserializeOwned;
use std::collections::HashMap;

use thiserror::Error;
#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to parse: {0}")]
    Parse(Box<dyn std::error::Error + Send + Sync>),
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

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Parse(Box::new(value))
    }
}

impl From<sonic_rs::Error> for Error {
    fn from(value: sonic_rs::Error) -> Self {
        Self::Parse(Box::new(value))
    }
}

#[derive(Debug)]
pub enum Request {
    RunInTerminal(<requests::RunInTerminal as helix_dap_types::Request>::Arguments),
    StartDebugging(<requests::StartDebugging as helix_dap_types::Request>::Arguments),
}

impl Request {
    pub fn parse(command: &str, arguments: Option<serde_json::Value>) -> Result<Self> {
        use helix_dap_types::Request as _;

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
    ProgressStart(<events::ProgressStart as events::Event>::Body),
    ProgressUpdate(<events::ProgressUpdate as events::Event>::Body),
    ProgressEnd(<events::ProgressEnd as events::Event>::Body),
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
            events::ProgressStart::EVENT => Self::ProgressStart(parse_value(body)?),
            events::ProgressUpdate::EVENT => Self::ProgressUpdate(parse_value(body)?),
            events::ProgressEnd::EVENT => Self::ProgressEnd(parse_value(body)?),
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

#[derive(Debug, Clone)]
pub struct ProgressState {
    title: String,
    message: Option<String>,
    percentage: Option<u8>,
}

impl ProgressState {
    pub fn new(title: String, message: Option<String>, percentage: Option<u8>) -> Self {
        Self {
            title,
            message,
            percentage,
        }
    }

    pub fn update(&mut self, message: Option<String>, percentage: Option<u8>) {
        if let Some(message) = message {
            self.message = Some(message);
        }
        if let Some(percentage) = percentage {
            self.percentage = Some(percentage);
        }
    }

    pub fn status_line(&self) -> String {
        let mut status = format!("Debug: {}", self.title);
        if let Some(message) = self.message.as_deref() {
            status.push_str(" - ");
            status.push_str(message);
        }
        if let Some(percentage) = self.percentage {
            status.push_str(&format!(" ({}%)", percentage));
        }
        status
    }

    pub fn end_status_line(&self, message: Option<&str>) -> String {
        let mut status = format!("Debug: {}", self.title);
        if let Some(message) = message.or(self.message.as_deref()) {
            status.push_str(" - ");
            status.push_str(message);
        } else {
            status.push_str(" finished");
        }
        status
    }
}

pub type ProgressMap = HashMap<String, ProgressState>;

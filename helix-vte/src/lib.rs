pub mod error;

use std::{
    collections::HashMap,
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use bytes::Bytes;
use error::Error;
use futures::{
    select,
    stream::{self, SelectAll},
    FutureExt, Stream, StreamExt,
};
use pty_process::{OwnedWritePty, Pty, Size};
use tokio::{io::AsyncWriteExt, sync::Notify};
use tokio_util::io::ReaderStream;

pub type TerminalId = u32;

static TERMINAL_ID_SEQ: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone)]
pub enum PtyMessage {
    Input(Vec<u8>),
    Resize(u16, u16),
    Close,
}

#[derive(Debug, Clone)]
pub enum PtyEvent {
    Data(Bytes),
    Error(String),
    Terminated(i32),
}

pub struct PtySpawnConfig {
    pub command: String,
    pub arguments: Option<Vec<String>>,
    pub size: Option<(u16, u16)>,
    pub cwd: Option<PathBuf>,
    pub env: Option<HashMap<String, String>>,
}

impl Default for PtySpawnConfig {
    fn default() -> Self {
        Self {
            command: std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()),
            arguments: None,
            size: None,
            cwd: Some(std::env::current_dir().unwrap()),
            env: None,
        }
    }
}

struct TermEntry {
    writer: OwnedWritePty,
    killer: Arc<Notify>,
}

type VteEventStream = Pin<Box<dyn Stream<Item = (TerminalId, PtyEvent)>>>;

pub struct VteRegistry {
    terminals: HashMap<TerminalId, TermEntry>,
    pub incoming: SelectAll<VteEventStream>,
}

impl Default for VteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl VteRegistry {
    pub fn new() -> Self {
        Self {
            terminals: Default::default(),
            incoming: SelectAll::new(),
        }
    }

    pub fn spawn_pty(&mut self, cfg: PtySpawnConfig) -> Result<TerminalId, Error> {
        let pty = Pty::new()?;
        let pts = pty.pts()?;

        pty.resize(match cfg.size {
            Some((r, c)) => Size::new(r, c),
            None => Size::new(24, 80),
        })?;

        let mut child = pty_process::Command::new(cfg.command);
        if let Some(args) = cfg.arguments {
            child.args(args);
        }

        if let Some(env) = cfg.env {
            child.envs(env);
        }

        if let Some(cwd) = cfg.cwd {
            child.current_dir(cwd);
        }

        let (reader, writer) = pty.into_split();
        let reader = ReaderStream::new(reader);

        let term_id = TERMINAL_ID_SEQ.fetch_add(1, Ordering::Relaxed);
        let mut process = child.spawn(&pts)?;
        let killer = Arc::new(Notify::new());
        let notify = killer.clone();

        self.incoming.push(Box::pin(stream::select(
            reader.map(move |dat| match dat {
                Ok(bytes) => (term_id, PtyEvent::Data(bytes)),
                Err(err) => (term_id, PtyEvent::Error(format!("{}", err))),
            }),
            async move {
                loop {
                    select! {
                        res = process.wait().fuse() => {
                            break (term_id, match res {
                                Ok(es) => PtyEvent::Terminated(es.code().unwrap_or(0)),
                                Err(err) => PtyEvent::Error(format!("{}", err)),
                            })
                        }
                        _ = notify.notified().fuse() => ()
                    }

                    if let Err(err) = process.start_kill() {
                        break (term_id, PtyEvent::Error(format!("{}", err)));
                    }
                }
            }
            .into_stream(),
        )));

        self.terminals.insert(term_id, TermEntry { writer, killer });

        Ok(term_id)
    }

    pub fn terminate(&mut self, id: TerminalId) -> Result<(), Error> {
        let entry = self
            .terminals
            .get_mut(&id)
            .ok_or(Error::TerminalNotFound(id))?;

        entry.killer.notify_waiters();

        Ok(())
    }

    pub async fn write<D: AsRef<[u8]>>(&mut self, id: TerminalId, data: D) -> Result<(), Error> {
        let entry = self
            .terminals
            .get_mut(&id)
            .ok_or(Error::TerminalNotFound(id))?;

        entry.writer.write_all(data.as_ref()).await?;

        Ok(())
    }

    pub fn resize(&mut self, id: TerminalId, new_size: (u16, u16)) -> Result<(), Error> {
        let entry = self
            .terminals
            .get_mut(&id)
            .ok_or(Error::TerminalNotFound(id))?;

        entry.writer.resize(Size::new(new_size.0, new_size.1))?;

        Ok(())
    }
}

use std::{
    collections::HashMap,
    future::poll_fn,
    io::{Read, Write},
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    task::Poll,
};

use crate::error::Error;
use bytes::Bytes;
use futures::{
    select,
    stream::{self, SelectAll},
    FutureExt, Stream, StreamExt,
};
use portable_pty::{native_pty_system, MasterPty, PtySize};
use tokio::sync::Notify;
use tokio_stream::wrappers::ReceiverStream;

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
    writer: Box<dyn Write + Send>,
    killer: Arc<Notify>,
    master: Box<dyn MasterPty + Send>,
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
        // Use the native pty implementation for the system
        let pty_system = native_pty_system();

        // Create a new pty
        let pair = pty_system.openpty(match cfg.size {
            Some((r, c)) => PtySize {
                rows: r,
                cols: c,
                pixel_height: 0,
                pixel_width: 0,
            },
            None => PtySize {
                rows: 24,
                cols: 80,
                pixel_height: 0,
                pixel_width: 0,
            },
        })?;

        let mut cmd = portable_pty::CommandBuilder::new(cfg.command);
        if let Some(args) = cfg.arguments {
            cmd.args(args);
        }

        if let Some(env) = cfg.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        if let Some(cwd) = cfg.cwd {
            cmd.cwd(cwd);
        }

        let mut process = pair.slave.spawn_command(cmd)?;

        drop(pair.slave);

        let (reader, writer) = (pair.master.try_clone_reader()?, pair.master.take_writer()?);

        let reader = Self::reader_to_stream(reader);

        let term_id = TERMINAL_ID_SEQ.fetch_add(1, Ordering::Relaxed);
        let killer = Arc::new(Notify::new());
        let notify = killer.clone();

        self.incoming.push(Box::pin(stream::select(
            reader.map(move |dat| match dat {
                Ok(bytes) => (term_id, PtyEvent::Data(bytes.into())),
                Err(err) => (term_id, PtyEvent::Error(format!("{}", err))),
            }),
            async move {
                loop {
                    select! {
                        res = poll_fn(|_cx| {
                            match process.try_wait() {
                                Ok(es) => {match es {
                                    Some(es) => return Poll::Ready(PtyEvent::Terminated(es.exit_code().try_into().unwrap())),
                                    None => return Poll::Pending
                                }},
                                Err(err) => return Poll::Ready(PtyEvent::Error(format!("{}", err))),
                            }
                        }).fuse() => {
                            break (term_id, res)
                        }
                        _ = notify.notified().fuse() => ()
                    }

                    if let Err(err) = process.kill() {
                        break (term_id, PtyEvent::Error(format!("{}", err)));
                    }
                }
            }
            .into_stream(),
        )));

        self.terminals.insert(
            term_id,
            TermEntry {
                writer,
                killer,
                master: pair.master,
            },
        );

        Ok(term_id)
    }

    pub async fn terminate(&mut self, id: TerminalId) -> Result<(), Error> {
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

        entry.writer.write_all(data.as_ref())?;
        Ok(())
    }

    pub fn resize(&mut self, id: TerminalId, new_size: (u16, u16)) -> Result<(), Error> {
        let entry = self
            .terminals
            .get_mut(&id)
            .ok_or(Error::TerminalNotFound(id))?;

        entry.master.resize(PtySize {
            rows: new_size.0,
            cols: new_size.1,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        Ok(())
    }

    pub fn reader_to_stream<R: Read + Send + 'static>(
        mut reader: R,
    ) -> impl Stream<Item = std::io::Result<Vec<u8>>> {
        let (tx, rx) = tokio::sync::mpsc::channel(8);

        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if tx.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.blocking_send(Err(e));
                        break;
                    }
                }
            }
        });

        ReceiverStream::new(rx)
    }
}

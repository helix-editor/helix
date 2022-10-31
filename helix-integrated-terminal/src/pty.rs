use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    io::{self, ErrorKind, Read, Write},
    num::NonZeroUsize,
    path::PathBuf,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

pub type TerminalId = u32;

use crate::error::Error;
use alacritty_terminal::{
    event::OnResize,
    event_loop::Msg,
    tty::{self, EventedPty, EventedReadWrite, Options, Pty, Shell},
};
use anyhow::Result;
use polling::PollMode;
use tokio::sync::mpsc::UnboundedSender;

static TERMINAL_ID_SEQ: AtomicU32 = AtomicU32::new(0);

const READ_BUFFER_SIZE: usize = 0x10_0000;

#[cfg(any(target_os = "linux", target_os = "macos"))]
const PTY_READ_WRITE_TOKEN: usize = 0;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const PTY_CHILD_EVENT_TOKEN: usize = 1;

#[cfg(target_os = "windows")]
const PTY_READ_WRITE_TOKEN: usize = 2;
#[cfg(target_os = "windows")]
const PTY_CHILD_EVENT_TOKEN: usize = 1;

pub struct TerminalSender {
    tx: UnboundedSender<Msg>,
    poller: Arc<polling::Poller>,
}

impl TerminalSender {
    pub fn new(tx: UnboundedSender<Msg>, poller: Arc<polling::Poller>) -> Self {
        Self { tx, poller }
    }

    pub fn send(&self, msg: Msg) {
        if let Err(err) = self.tx.send(msg) {
            log::error!("{:?}", err);
        }
        if let Err(err) = self.poller.notify() {
            log::error!("{:?}", err);
        }
    }
}

pub struct TermConfig {
    pub command: Option<String>,
    pub arguments: Option<Vec<String>>,
    pub size: Option<(u16, u16)>,
    pub cwd: Option<PathBuf>,
    pub env: Option<HashMap<String, String>>,
}

impl Default for TermConfig {
    fn default() -> Self {
        Self {
            command: None,
            arguments: None,
            size: None,
            cwd: Some(std::env::current_dir().unwrap()),
            env: None,
        }
    }
}

struct Terminal {
    pty: Pty,
    rx: tokio::sync::mpsc::UnboundedReceiver<alacritty_terminal::event_loop::Msg>,
    tx: tokio::sync::mpsc::UnboundedSender<alacritty_terminal::event_loop::Msg>,
    pub poller: Arc<polling::Poller>,
    outer_tx: tokio::sync::mpsc::UnboundedSender<(TerminalId, PtyEvent)>,
    term_id: TerminalId,
}

impl Terminal {
    /// Create a new terminal from a `TermConfig`
    fn new(
        cfg: TermConfig,
        outer_tx: tokio::sync::mpsc::UnboundedSender<(TerminalId, PtyEvent)>,
    ) -> Result<Terminal> {
        let poller = polling::Poller::new()?.into();

        let options = Options {
            shell: cfg.command.map(|s| Shell::new(s, Vec::new())),
            working_directory: cfg.cwd,
            drain_on_exit: true,
            env: if let Some(env) = cfg.env {
                env
            } else {
                HashMap::new()
            },
            #[cfg(target_os = "windows")]
            escape_args: true, // TODO: I have no idea whether this should be true or false
        };

        let pty = tty::new(
            &options,
            match cfg.size {
                Some((r, c)) => alacritty_terminal::event::WindowSize {
                    num_lines: r,
                    num_cols: c,
                    cell_width: 0,
                    cell_height: 0,
                },
                None => alacritty_terminal::event::WindowSize {
                    num_lines: 24,
                    num_cols: 80,
                    cell_width: 0,
                    cell_height: 0,
                },
            },
            0,
        )?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let term_id = TERMINAL_ID_SEQ.fetch_add(1, Ordering::Relaxed);

        Ok(Terminal {
            pty,
            rx,
            tx,
            outer_tx,
            poller,
            term_id,
        })
    }

    fn run(&mut self) {
        let mut state = State::default();
        let mut buf = [0u8; READ_BUFFER_SIZE];

        let poll_opts = PollMode::Level;
        let mut interest = polling::Event::readable(0);

        // Register TTY through EventedRW interface.
        unsafe {
            self.pty
                .register(&self.poller, interest, poll_opts)
                .unwrap();
        }

        let mut events = polling::Events::with_capacity(NonZeroUsize::new(1024).unwrap());

        let timeout = Some(Duration::from_secs(6));
        let mut exit_code = None;
        'event_loop: loop {
            events.clear();
            if let Err(err) = self.poller.wait(&mut events, timeout) {
                match err.kind() {
                    ErrorKind::Interrupted => continue,
                    _ => panic!("EventLoop polling error: {err:?}"),
                }
            }

            // Handle channel events, if there are any.
            if !self.drain_recv_channel(&mut state) {
                break;
            }

            for event in events.iter() {
                match event.key {
                    PTY_CHILD_EVENT_TOKEN => {
                        if let Some(tty::ChildEvent::Exited(exited_code)) =
                            self.pty.next_child_event()
                        {
                            if let Err(err) = self.pty_read(&mut buf) {
                                log::error!("{:?}", err);
                            }
                            exit_code = exited_code;
                            break 'event_loop;
                        }
                    }

                    PTY_READ_WRITE_TOKEN => {
                        if event.is_interrupt() {
                            // Don't try to do I/O on a dead PTY.
                            continue;
                        }

                        if event.readable {
                            if let Err(err) = self.pty_read(&mut buf) {
                                // On Linux, a `read` on the master side of a PTY can fail
                                // with `EIO` if the client side hangs up.  In that case,
                                // just loop back round for the inevitable `Exited` event.
                                // This sucks, but checking the process is either racy or
                                // blocking.
                                #[cfg(target_os = "linux")]
                                if err.raw_os_error() == Some(libc::EIO) {
                                    continue;
                                }

                                log::error!("Error reading from PTY in event loop: {}", err);
                                break 'event_loop;
                            }
                        }

                        if event.writable {
                            if let Err(_err) = self.pty_write(&mut state) {
                                // error!(
                                //     "Error writing to PTY in event loop: {}",
                                //     err
                                // );
                                break 'event_loop;
                            }
                        }
                    }
                    _ => (),
                }
            }

            // Register write interest if necessary.
            let needs_write = state.needs_write();
            if needs_write != interest.writable {
                interest.writable = needs_write;

                // Re-register with new interest.
                self.pty
                    .reregister(&self.poller, interest, poll_opts)
                    .unwrap();
            }
        }
        let _ = self
            .outer_tx
            .send((self.term_id, PtyEvent::TerminalStopped(exit_code))); // TODO: Should we be ignoring this?
        if let Err(err) = self.pty.deregister(&self.poller) {
            log::error!("{:?}", err);
        }
    }

    /// Drain the channel.
    ///
    /// Returns `false` when a shutdown message was received.
    fn drain_recv_channel(&mut self, state: &mut State) -> bool {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Input(input) => state.write_list.push_back(input),
                Msg::Shutdown => return false,
                Msg::Resize(size) => self.pty.on_resize(size),
            }
        }

        true
    }

    #[inline]
    fn pty_read(&mut self, buf: &mut [u8]) -> io::Result<()> {
        loop {
            match self.pty.reader().read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = self
                        .outer_tx
                        .send((self.term_id, PtyEvent::UpdateTerminal(buf[..n].to_vec())));
                }
                Err(err) => match err.kind() {
                    ErrorKind::Interrupted | ErrorKind::WouldBlock => {
                        break;
                    }
                    _ => return Err(err),
                },
            }
        }
        Ok(())
    }

    #[inline]
    fn pty_write(&mut self, state: &mut State) -> io::Result<()> {
        state.ensure_next();

        'write_many: while let Some(mut current) = state.take_current() {
            'write_one: loop {
                match self.pty.writer().write(current.remaining_bytes()) {
                    Ok(0) => {
                        state.set_current(Some(current));
                        break 'write_many;
                    }
                    Ok(n) => {
                        current.advance(n);
                        if current.finished() {
                            state.goto_next();
                            break 'write_one;
                        }
                    }
                    Err(err) => {
                        state.set_current(Some(current));
                        match err.kind() {
                            ErrorKind::Interrupted | ErrorKind::WouldBlock => {
                                break 'write_many;
                            }
                            _ => return Err(err),
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum PtyEvent {
    UpdateTerminal(Vec<u8>),
    TerminalStopped(Option<i32>),
}

pub struct TerminalRegistry {
    terminals: HashMap<TerminalId, TerminalSender>,
    pub rx: tokio::sync::mpsc::UnboundedReceiver<(TerminalId, PtyEvent)>,
    tx: tokio::sync::mpsc::UnboundedSender<(TerminalId, PtyEvent)>,
}

impl Default for TerminalRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalRegistry {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            terminals: Default::default(),
            tx,
            rx,
        }
    }

    pub fn new_terminal(&mut self, cfg: TermConfig) -> Result<TerminalId, Error> {
        let tx_outer = self.tx.clone();
        let mut terminal = Terminal::new(cfg, tx_outer)?;
        let terminal_id = terminal.term_id.clone();
        let terminal_tx = terminal.tx.clone();
        let poller = terminal.poller.clone();
        let sender = TerminalSender::new(terminal_tx, poller);
        self.terminals.insert(terminal_id, sender);
        tokio::task::spawn_blocking(move || {
            terminal.run();
        });
        Ok(terminal_id)
    }

    pub fn terminate(&mut self, id: TerminalId) -> Result<(), Error> {
        let entry = self
            .terminals
            .get_mut(&id)
            .ok_or(Error::TerminalNotFound(id))?;

        entry.send(Msg::Shutdown);
        Ok(())
    }

    pub fn write(&mut self, id: TerminalId, data: Cow<'static, [u8]>) -> Result<(), Error> {
        let entry = self
            .terminals
            .get_mut(&id)
            .ok_or(Error::TerminalNotFound(id))?;

        entry.send(Msg::Input(data));
        Ok(())
    }

    pub fn resize(&mut self, id: TerminalId, row: u16, col: u16) -> Result<(), Error> {
        let entry = self
            .terminals
            .get_mut(&id)
            .ok_or(Error::TerminalNotFound(id))?;

        entry.send(Msg::Resize(alacritty_terminal::event::WindowSize {
            num_lines: row,
            num_cols: col,
            cell_width: 0,
            cell_height: 0,
        }));

        Ok(())
    }
}

struct Writing {
    source: Cow<'static, [u8]>,
    written: usize,
}

impl Writing {
    #[inline]
    fn new(c: Cow<'static, [u8]>) -> Writing {
        Writing {
            source: c,
            written: 0,
        }
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        self.written += n;
    }

    #[inline]
    fn remaining_bytes(&self) -> &[u8] {
        &self.source[self.written..]
    }

    #[inline]
    fn finished(&self) -> bool {
        self.written >= self.source.len()
    }
}

#[derive(Default)]
pub struct State {
    write_list: VecDeque<Cow<'static, [u8]>>,
    writing: Option<Writing>,
}

impl State {
    #[inline]
    fn ensure_next(&mut self) {
        if self.writing.is_none() {
            self.goto_next();
        }
    }

    #[inline]
    fn goto_next(&mut self) {
        self.writing = self.write_list.pop_front().map(Writing::new);
    }

    #[inline]
    fn take_current(&mut self) -> Option<Writing> {
        self.writing.take()
    }

    #[inline]
    fn needs_write(&self) -> bool {
        self.writing.is_some() || !self.write_list.is_empty()
    }

    #[inline]
    fn set_current(&mut self, new: Option<Writing>) {
        self.writing = new;
    }
}

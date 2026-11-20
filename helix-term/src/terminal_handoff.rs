//! Lending the terminal to child processes.
//!
//! This backs `:run-shell-command-blocking` and the `%shb{..}` expansion. Programs like file
//! managers or git clients are full-screen TUIs: they can't share the terminal with the editor
//! so, for as long as they run, the editor puts the terminal back in the state it found it in
//! and waits for them to exit.

use std::{
    cell::{RefCell, RefMut},
    io,
    process::{Command, Output},
    rc::Rc,
};

use helix_view::{editor::TerminalHandoff, graphics::CursorKind};
use tui::{backend::Backend, terminal::Terminal};

/// The terminal, shared between the [`Application`](crate::application::Application) and the
/// [`Handoff`] it installs in the `Editor`.
pub struct SharedTerminal<B: Backend>(Rc<RefCell<Terminal<B>>>);

impl<B: Backend> Clone for SharedTerminal<B> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<B: Backend> SharedTerminal<B> {
    pub fn new(terminal: Terminal<B>) -> Self {
        Self(Rc::new(RefCell::new(terminal)))
    }

    /// Borrows the terminal. Don't hold on to the borrow across anything that might run a
    /// command (the handoff fails if the terminal is already borrowed).
    pub fn get(&self) -> RefMut<'_, Terminal<B>> {
        self.0.borrow_mut()
    }

    fn try_get(&self) -> io::Result<RefMut<'_, Terminal<B>>> {
        self.0.try_borrow_mut().map_err(|_| {
            io::Error::other("the terminal is in use, try again from a keybinding or `:`")
        })
    }

    pub fn handoff(&self) -> Handoff<B> {
        Handoff(self.clone())
    }
}

pub struct Handoff<B: Backend>(SharedTerminal<B>);

impl<B: Backend> TerminalHandoff for Handoff<B> {
    fn run(&self, command: &mut Command) -> io::Result<Output> {
        let mut terminal = self.0.try_get()?;

        // Nothing is reading terminal input while we're in here: handlers run synchronously from
        // the event loop, which is the only thing that polls the terminal's event stream.
        terminal.backend_mut().show_cursor(CursorKind::Block).ok();
        terminal.restore()?;
        sys::blank_screen();

        let output = sys::run(command);

        // Reclaim the terminal even if the command couldn't be run. The alternate screen is
        // blank again, so the diffing renderer must be told to repaint everything.
        let claimed = terminal.claim();
        terminal.clear().ok();
        helix_event::request_redraw();

        let output = output?;
        claimed?;
        Ok(output)
    }
}

#[cfg(unix)]
mod sys {
    use std::{
        fs::File,
        io::{self, IsTerminal, Read},
        os::unix::process::{CommandExt, ExitStatusExt},
        process::{Command, ExitStatus, Output},
        thread,
    };

    /// Signals the terminal's line discipline sends to the whole foreground process group in
    /// response to `<C-c>`, `<C-\>` and `<C-z>` while the terminal is in cooked mode.
    const TERMINAL_SIGNALS: [libc::c_int; 3] = [libc::SIGINT, libc::SIGQUIT, libc::SIGTSTP];

    /// Ignores [`TERMINAL_SIGNALS`] in this process until dropped.
    ///
    /// The child shares our process group, so `<C-c>` in `sleep 10` would otherwise also reach
    /// the editor, which quits on SIGINT and suspends itself on SIGTSTP. This is the same dance
    /// `system(3)` does.
    struct IgnoreTerminalSignals(Vec<(libc::c_int, libc::sigaction)>);

    impl IgnoreTerminalSignals {
        fn new() -> Self {
            let mut previous = Vec::new();
            for signal in TERMINAL_SIGNALS {
                // SAFETY: `sigaction` is plain old data for which all-zeroes is a valid value
                // and the pointers passed to `libc::sigaction` are valid for the call.
                unsafe {
                    let mut ignore: libc::sigaction = std::mem::zeroed();
                    ignore.sa_sigaction = libc::SIG_IGN;
                    libc::sigemptyset(&mut ignore.sa_mask);
                    let mut old: libc::sigaction = std::mem::zeroed();
                    if libc::sigaction(signal, &ignore, &mut old) == 0 {
                        previous.push((signal, old));
                    }
                }
            }
            Self(previous)
        }
    }

    impl Drop for IgnoreTerminalSignals {
        fn drop(&mut self) {
            for (signal, old) in &self.0 {
                // SAFETY: restores the action returned by `sigaction` above.
                unsafe { libc::sigaction(*signal, old, std::ptr::null_mut()) };
            }
        }
    }

    /// Switches to a cleared alternate screen. Best effort.
    pub fn blank_screen() {
        use std::io::Write;

        if let Ok(mut tty) = File::options().write(true).open("/dev/tty") {
            let _ = tty.write_all(b"\x1b[?1049h\x1b[2J\x1b[H");
        }
    }

    pub fn run(command: &mut Command) -> io::Result<Output> {
        // When the editor was fed a file on stdin (`cat foo | hx`) its stdin isn't the terminal.
        if !io::stdin().is_terminal() {
            if let Ok(tty) = File::open("/dev/tty") {
                command.stdin(tty);
            }
        }

        // SAFETY: only async-signal-safe functions are called between `fork` and `exec`.
        unsafe {
            command.pre_exec(|| {
                // Ignored signals stay ignored across `exec`: give the child the defaults back.
                for signal in TERMINAL_SIGNALS {
                    libc::signal(signal, libc::SIG_DFL);
                }
                Ok(())
            });
        }

        let _ignore = IgnoreTerminalSignals::new();
        let mut child = command.spawn()?;

        // Read stdout on the side so a chatty child can't fill the pipe and deadlock with `wait`.
        let stdout = child.stdout.take().map(|mut stdout| {
            thread::spawn(move || {
                let mut buf = Vec::new();
                stdout.read_to_end(&mut buf).map(|_| buf)
            })
        });

        let status = wait(child.id() as libc::pid_t)?;
        let stdout = match stdout {
            Some(reader) => reader
                .join()
                .map_err(|_| io::Error::other("stdout reader panicked"))??,
            None => Vec::new(),
        };

        Ok(Output {
            status,
            stdout,
            stderr: Vec::new(),
        })
    }

    /// Waits for `pid` to exit, resuming it if it stops.
    ///
    /// A child that suspends itself (`<C-z>` in a TUI which supports it) would otherwise hang the
    /// editor: there's no shell around to bring it back to the foreground.
    fn wait(pid: libc::pid_t) -> io::Result<ExitStatus> {
        loop {
            let mut raw = 0;
            // SAFETY: `raw` is valid for writes for the duration of the call.
            let res = unsafe { libc::waitpid(pid, &mut raw, libc::WUNTRACED) };
            if res == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }
            if libc::WIFSTOPPED(raw) {
                // SAFETY: `pid` is our un-reaped child.
                unsafe { libc::kill(pid, libc::SIGCONT) };
                continue;
            }
            return Ok(ExitStatus::from_raw(raw));
        }
    }
}

#[cfg(not(unix))]
mod sys {
    use std::{
        io,
        process::{Command, Output},
    };

    pub fn blank_screen() {}

    pub fn run(command: &mut Command) -> io::Result<Output> {
        command.spawn()?.wait_with_output()
    }
}

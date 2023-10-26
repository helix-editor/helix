use std::cell::Cell;
use std::io::{Error as IoError, ErrorKind, Result as IoResult, Write};
use std::ops::Deref;

use super::Terminal;

// License for code below:

/*
Copyright (c) 2020 Rahul Butani

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
*/

/// Wrapper for the [xterm.js terminal](Terminal) for use with [crossterm].
///
/// [crossterm]: docs.rs/crossterm
pub struct XtermJsCrosstermBackend<'a> {
    /// The xterm.js terminal that this struct instance wraps.
    pub terminal: &'a Terminal,
    /// Internal buffer for data to write to the terminal.
    ///
    /// This lets us make one big call to [`Terminal::write`] with a batch of
    /// commands rather than many small calls.
    buffer: Cell<Vec<u8>>,
}

impl<'a> Deref for XtermJsCrosstermBackend<'a> {
    type Target = Terminal;

    fn deref(&self) -> &Terminal {
        //! This will flush the internal buffer before providing the reference
        //! to make sure that the order of operations is preserved.
        self.flush_immutable().unwrap();
        self.terminal
    }
}

impl<'a> Drop for XtermJsCrosstermBackend<'a> {
    fn drop(&mut self) {
        self.flush().unwrap()
    }
}

impl<'a> XtermJsCrosstermBackend<'a> {
    /// Constructor for the wrapper type.
    #[must_use]
    pub fn new(terminal: &'a Terminal) -> Self {
        Self::new_with_capacity(terminal, 0)
    }

    /// Like [`new`](XtermJsCrosstermBackend::new) except it also takes an
    /// estimate for the size of the internal buffer.
    ///
    /// This is useful if you have a good guess about how many bytes the
    /// commands you're going to send will need.
    #[must_use]
    pub fn new_with_capacity(terminal: &'a Terminal, capacity: usize) -> Self {
        Self {
            terminal,
            buffer: Cell::new(Vec::with_capacity(capacity)),
        }
    }

    /// Writes a `String` directly to the underlying terminal, bypassing the
    /// buffer.
    ///
    /// This is useful for situations in which the commands being sent are
    /// already buffered and the extra copy is undesired. Note that this will
    /// flush the buffer first to preserve the order of commands.
    ///
    /// # Errors
    ///
    /// This should never actually error. For consistency with the [`Write`]
    /// calls it results an [`io::Result`]
    ///
    /// [`Write`]: std::io::Write
    /// [`io::Result`]: std::io::Result
    pub fn write_immediately(&mut self, commands: String) -> IoResult<()> {
        self.flush()?;
        self.terminal.write(&commands);

        Ok(())
    }

    /// A version of [`flush`](Write::flush) that takes an immutable reference
    /// instead of a mutable one.
    ///
    /// This exists because we want to flush the buffer in the [`Deref`] impl.
    #[inline]
    fn flush_immutable(&self) -> IoResult<()> {
        // Can't call `self.buffer.flush()` here but since that's just a Vec,
        // it's probably fine.

        let s = String::from_utf8(self.buffer.replace(Vec::new()))
            .map_err(|e| IoError::new(ErrorKind::Other, e))?;
        self.terminal.write(&s);

        Ok(())
    }
}

impl<'a> Write for XtermJsCrosstermBackend<'a> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.buffer.get_mut().write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.buffer.get_mut().flush()?;

        self.flush_immutable()
    }
}

impl<'a> From<&'a Terminal> for XtermJsCrosstermBackend<'a> {
    fn from(terminal: &'a Terminal) -> Self {
        Self::new(terminal)
    }
}

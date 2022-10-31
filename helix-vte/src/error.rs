#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Pty Error: {0}")]
    PtyError(#[from] pty_process::Error),

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Termina Not Found: {0}")]
    TerminalNotFound(u32),
}

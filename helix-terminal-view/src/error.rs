#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Pty Error: {0}")]
    PtyError(#[from] anyhow::Error),

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Terminal Not Found: {0}")]
    TerminalNotFound(u32),
}

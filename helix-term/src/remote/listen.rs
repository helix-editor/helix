use std::fs::{self, Permissions};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::{Path, PathBuf};

use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub enum BindError {
    /// Another process is already accepting on this path.
    InUse,
    Io(io::Error),
}

impl From<io::Error> for BindError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Create the parent directory, replace a stale sock file, bind, and chmod 0600.
///
/// If `path` exists and `connect()` succeeds, another Helix owns it: return
/// [`BindError::InUse`] and do not unlink. `chmod 0700` applies only to a
/// directory this call created, never to a pre-existing `XDG_RUNTIME_DIR`.
pub fn bind_socket(path: &Path) -> Result<UnixListener, BindError> {
    if let Some(parent) = path.parent() {
        let existed = parent.exists();
        fs::create_dir_all(parent)?;
        if !existed {
            fs::set_permissions(parent, Permissions::from_mode(0o700))?;
        }
    }

    if path.exists() {
        match StdUnixStream::connect(path) {
            Ok(_) => return Err(BindError::InUse),
            Err(_) => fs::remove_file(path)?,
        }
    }

    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Bind `path` and spawn the accept loop. Returns `None` if bind failed or the
/// path is already in use (does not steal a live sock).
pub fn spawn(path: PathBuf, tx: mpsc::Sender<String>) -> Option<(JoinHandle<()>, PathBuf)> {
    let listener = match bind_socket(&path) {
        Ok(listener) => listener,
        Err(BindError::InUse) => {
            log::error!(
                "socket {} is already in use; not binding",
                path.display()
            );
            return None;
        }
        Err(BindError::Io(e)) => {
            log::error!("Failed to bind listener to socket: {e}");
            return None;
        }
    };
    let handle = tokio::spawn(accept_loop(listener, tx));
    Some((handle, path))
}

async fn accept_loop(listener: UnixListener, tx: mpsc::Sender<String>) {
    loop {
        match listener.accept().await {
            Ok((mut socket, _)) => {
                let mut buf = vec![0; 1024];
                match socket.read(&mut buf).await {
                    Ok(n) if n > 0 => {
                        let msg = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = tx.send(msg).await;
                    }
                    Ok(_) => {}
                    Err(e) => log::error!("Socket read error: {e}"),
                }
            }
            Err(e) => log::error!("Socket accept error: {e}"),
        }
    }
}

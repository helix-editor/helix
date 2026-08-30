use std::fs::{self, Permissions};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::{Path, PathBuf};

use tokio::io::{AsyncBufRead, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::protocol::{ClientMessage, ClientOp, MAX_LINE, PROTOCOL_V};

#[derive(Debug)]
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
pub fn spawn(
    path: PathBuf,
    tx: mpsc::UnboundedSender<String>,
) -> Option<(JoinHandle<()>, PathBuf)> {
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

async fn accept_loop(listener: UnixListener, tx: mpsc::UnboundedSender<String>) {
    loop {
        match listener.accept().await {
            Ok((socket, _)) => {
                let tx = tx.clone();
                tokio::spawn(connection_task(socket, tx));
            }
            Err(e) => log::error!("Socket accept error: {e}"),
        }
    }
}

async fn connection_task(socket: UnixStream, tx: mpsc::UnboundedSender<String>) {
    let (read, _write) = socket.into_split();
    let mut reader = BufReader::new(read);
    loop {
        match read_line_capped(&mut reader, MAX_LINE).await {
            Ok(None) => break,
            Ok(Some(line)) => handle_inbound_line(&line, &tx),
            Err(ReadLineError::Oversize) => {
                log::error!("remote connection closed: line exceeded {MAX_LINE} bytes");
                break;
            }
            Err(ReadLineError::Io(e)) => {
                if e.kind() != io::ErrorKind::InvalidData {
                    log::error!("Socket read error: {e}");
                } else {
                    log::error!("remote connection closed: invalid UTF-8");
                }
                break;
            }
        }
    }
}

fn handle_inbound_line(line: &str, tx: &mpsc::UnboundedSender<String>) {
    if line.is_empty() {
        return;
    }
    if line.starts_with('{') {
        let msg: ClientMessage = match serde_json::from_str(line) {
            Ok(msg) => msg,
            Err(err) => {
                log::error!("invalid remote JSON: {err}");
                return;
            }
        };
        if msg.v != PROTOCOL_V {
            log::error!("unsupported remote protocol version {}", msg.v);
            return;
        }
        match msg.op {
            ClientOp::Command { cmd } => {
                let _ = tx.send(cmd);
            }
        }
        return;
    }
    let _ = tx.send(line.to_string());
}

pub(super) enum ReadLineError {
    Oversize,
    Io(io::Error),
}

impl From<io::Error> for ReadLineError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Read one newline-delimited line, capped at `cap` bytes. A final line without
/// a trailing newline is returned on EOF. `Ok(None)` is clean EOF.
pub(super) async fn read_line_capped<R>(
    reader: &mut R,
    cap: usize,
) -> Result<Option<String>, ReadLineError>
where
    R: AsyncBufRead + Unpin,
{
    let mut buf = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            if buf.is_empty() {
                return Ok(None);
            }
            break;
        }
        if let Some(i) = available.iter().position(|&b| b == b'\n') {
            if buf.len() + i + 1 > cap {
                return Err(ReadLineError::Oversize);
            }
            buf.extend_from_slice(&available[..=i]);
            reader.consume(i + 1);
            break;
        }
        if buf.len() + available.len() > cap {
            return Err(ReadLineError::Oversize);
        }
        let n = available.len();
        buf.extend_from_slice(available);
        reader.consume(n);
    }

    if buf.last() == Some(&b'\n') {
        buf.pop();
    }
    if buf.last() == Some(&b'\r') {
        buf.pop();
    }

    String::from_utf8(buf)
        .map(Some)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn stale_sock_is_replaced() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("helix.sock");
        std::fs::write(&path, b"").unwrap();
        bind_socket(&path).expect("stale path should bind");
        assert!(path.exists());
    }

    #[tokio::test]
    async fn live_sock_is_not_stolen() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("helix.sock");
        let _first = bind_socket(&path).unwrap();
        match bind_socket(&path) {
            Err(BindError::InUse) => {}
            other => panic!("expected InUse, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn sock_is_0600() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("helix.sock");
        let _listener = bind_socket(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[tokio::test]
    async fn created_dir_is_0700() {
        let dir = TempDir::new().unwrap();
        let parent = dir.path().join("helix");
        let path = parent.join("helix.sock");
        let _listener = bind_socket(&path).unwrap();
        let mode = std::fs::metadata(&parent).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
    }

    #[tokio::test]
    async fn preexisting_dir_mode_unchanged() {
        let dir = TempDir::new().unwrap();
        let parent = dir.path().join("helix");
        std::fs::create_dir(&parent).unwrap();
        std::fs::set_permissions(&parent, Permissions::from_mode(0o755)).unwrap();
        let path = parent.join("helix.sock");
        let _listener = bind_socket(&path).unwrap();
        let mode = std::fs::metadata(&parent).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);
    }

    async fn spawn_conn(server: UnixStream) -> mpsc::UnboundedReceiver<String> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        tokio::spawn(super::connection_task(server, cmd_tx));
        cmd_rx
    }

    #[tokio::test]
    async fn framing_splits_lines() {
        let (client, server) = UnixStream::pair().unwrap();
        let mut cmd_rx = spawn_conn(server).await;
        let mut client = client;
        client.write_all(b":open a\n:open b\n").await.unwrap();
        client.shutdown().await.unwrap();
        assert_eq!(cmd_rx.recv().await.unwrap(), ":open a");
        assert_eq!(cmd_rx.recv().await.unwrap(), ":open b");
    }

    #[tokio::test]
    async fn eof_without_newline() {
        let (client, server) = UnixStream::pair().unwrap();
        let mut cmd_rx = spawn_conn(server).await;
        let mut client = client;
        client.write_all(b":open a").await.unwrap();
        client.shutdown().await.unwrap();
        assert_eq!(cmd_rx.recv().await.unwrap(), ":open a");
    }

    #[tokio::test]
    async fn json_command_unwraps_cmd() {
        let (client, server) = UnixStream::pair().unwrap();
        let mut cmd_rx = spawn_conn(server).await;
        let mut client = client;
        client
            .write_all(br#"{"v":1,"op":"command","cmd":":open /abs/foo.rs:12"}"#)
            .await
            .unwrap();
        client.write_all(b"\n").await.unwrap();
        client.shutdown().await.unwrap();
        assert_eq!(cmd_rx.recv().await.unwrap(), ":open /abs/foo.rs:12");
    }

    #[tokio::test]
    async fn oversize_closes_only_that_connection() {
        let (c1, s1) = UnixStream::pair().unwrap();
        let (c2, s2) = UnixStream::pair().unwrap();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        tokio::spawn(super::connection_task(s1, cmd_tx.clone()));
        tokio::spawn(super::connection_task(s2, cmd_tx));

        let mut oversized = vec![b'x'; MAX_LINE + 1];
        oversized.push(b'\n');
        let mut c1 = c1;
        c1.write_all(&oversized).await.unwrap();
        c1.shutdown().await.unwrap();

        let mut c2 = c2;
        c2.write_all(b":ok\n").await.unwrap();
        c2.shutdown().await.unwrap();
        assert_eq!(cmd_rx.recv().await.unwrap(), ":ok");
        assert!(cmd_rx.try_recv().is_err());
    }
}

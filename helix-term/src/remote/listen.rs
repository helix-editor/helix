use std::fs::{self, Permissions};
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::protocol::{ClientMessage, ClientOp, MAX_LINE, PROTOCOL_V};

/// Serializes `umask` around bind: umask is process-global.
static BIND_UMASK: Mutex<()> = Mutex::new(());

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
/// Bind first. On `EADDRINUSE`, `connect`: success means a live peer ([`BindError::InUse`],
/// do not unlink); failure means stale, so unlink and bind once, then retry once more
/// if that bind still fails. Bind uses umask 0o077 so the sock is never
/// world-connectable, then fchmod 0600 on the listener fd. `chmod 0700` applies
/// only to a directory this call created, never to a pre-existing
/// `XDG_RUNTIME_DIR`. A pre-existing parent must be owned by the current user
/// or root (so `$TMPDIR/helix-$UID` cannot be an attacker-owned nest, while
/// `--socket` under `/tmp` still works).
pub fn bind_socket(path: &Path) -> Result<UnixListener, BindError> {
    ensure_parent(path)?;
    match bind_private(path) {
        Ok(listener) => Ok(listener),
        Err(err) if is_addr_in_use(&err) => replace_stale_or_in_use(path),
        Err(err) => Err(err.into()),
    }
}

fn is_addr_in_use(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::AddrInUse | io::ErrorKind::AlreadyExists
    )
}

/// Live peer → [`BindError::InUse`]. Stale path → unlink and bind, then one retry.
fn replace_stale_or_in_use(path: &Path) -> Result<UnixListener, BindError> {
    if StdUnixStream::connect(path).is_ok() {
        return Err(BindError::InUse);
    }
    let _ = fs::remove_file(path);
    match bind_private(path) {
        Ok(listener) => Ok(listener),
        Err(err) if is_addr_in_use(&err) => {
            if StdUnixStream::connect(path).is_ok() {
                return Err(BindError::InUse);
            }
            let _ = fs::remove_file(path);
            bind_private(path).map_err(BindError::from)
        }
        Err(err) => Err(err.into()),
    }
}

fn ensure_parent(path: &Path) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    let existed = parent.exists();
    fs::create_dir_all(parent)?;
    let owner = fs::metadata(parent)?.uid();
    if owner != current_uid() && owner != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "socket parent {} is not owned by the current user",
                parent.display()
            ),
        ));
    }
    if !existed {
        fs::set_permissions(parent, Permissions::from_mode(0o700))?;
    }
    Ok(())
}

struct UmaskGuard {
    previous: libc::mode_t,
}

impl UmaskGuard {
    fn set(mask: libc::mode_t) -> Self {
        // SAFETY: umask has no preconditions; the caller holds BIND_UMASK so
        // concurrent binds in this process cannot clobber each other.
        let previous = unsafe { libc::umask(mask) };
        Self { previous }
    }
}

impl Drop for UmaskGuard {
    fn drop(&mut self) {
        unsafe { libc::umask(self.previous) };
    }
}

/// Bind with umask 0o077 so the sock is never world-connectable, then fchmod 0600.
fn bind_private(path: &Path) -> io::Result<UnixListener> {
    let _lock = BIND_UMASK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let listener = {
        let _umask = UmaskGuard::set(0o077);
        UnixListener::bind(path)?
    };
    // fchmod the bound inode (no path TOCTOU). Path chmod keeps the mode at
    // 0600 if the filesystem reports the umask-created 0700 to `stat`.
    fchmod_socket(&listener, 0o600)?;
    fs::set_permissions(path, Permissions::from_mode(0o600))?;
    Ok(listener)
}

fn fchmod_socket(listener: &UnixListener, mode: libc::mode_t) -> io::Result<()> {
    let rc = unsafe { libc::fchmod(listener.as_raw_fd(), mode) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn current_uid() -> u32 {
    // SAFETY: getuid has no preconditions and cannot fail.
    unsafe { libc::getuid() }
}

fn peer_uid_allowed(peer: u32, self_uid: u32) -> bool {
    peer == self_uid
}

fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        stream.peer_cred().map(|cred| cred.uid())
    }
    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        local_peer_uid(stream)
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    {
        let _ = stream;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "peer credentials unavailable on this OS",
        ))
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn local_peer_uid(stream: &UnixStream) -> io::Result<u32> {
    let mut cred: libc::xucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::xucred>() as libc::socklen_t;
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERCRED,
            &mut cred as *mut _ as *mut libc::c_void,
            &mut len,
        )
    };
    if rc == 0 {
        Ok(cred.cr_uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

const PEER_CRED_REQUIRED: bool = cfg!(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
));

/// Drop connections whose peer uid is not the editor's. On platforms without
/// a peer-cred API, allow the connection (mode 0600 is the remaining gate).
fn accept_if_same_uid(stream: UnixStream) -> Option<UnixStream> {
    match peer_uid(&stream) {
        Ok(uid) if peer_uid_allowed(uid, current_uid()) => Some(stream),
        Ok(uid) => {
            log::error!("dropping remote connection: peer uid {uid} != current user");
            None
        }
        Err(err) if PEER_CRED_REQUIRED => {
            log::error!("dropping remote connection: could not read peer credentials ({err})");
            None
        }
        Err(err) => {
            log::debug!("peer uid check skipped: {err}");
            Some(stream)
        }
    }
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
            log::error!("socket {} is already in use; not binding", path.display());
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
                if let Some(socket) = accept_if_same_uid(socket) {
                    let tx = tx.clone();
                    tokio::spawn(connection_task(socket, tx));
                }
            }
            Err(e) => log::error!("Socket accept error: {e}"),
        }
    }
}

pub(crate) async fn connection_task(socket: UnixStream, tx: mpsc::UnboundedSender<String>) {
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
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

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
    async fn concurrent_two_starters_one_wins() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("helix.sock");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let handle = tokio::runtime::Handle::current();
        let spawn = |path: PathBuf, barrier: std::sync::Arc<std::sync::Barrier>| {
            let handle = handle.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let _enter = handle.enter();
                bind_socket(&path)
            })
        };
        let a = spawn(path.clone(), barrier.clone());
        let b = spawn(path, barrier);
        let r1 = a.join().expect("thread 1");
        let r2 = b.join().expect("thread 2");
        let winners = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
        assert_eq!(winners, 1, "r1={r1:?} r2={r2:?}");
        let loser = if r1.is_ok() { r2.as_ref() } else { r1.as_ref() };
        assert!(
            matches!(loser, Err(BindError::InUse) | Err(BindError::Io(_))),
            "loser={loser:?}"
        );
    }

    #[tokio::test]
    async fn sock_is_0600() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("helix.sock");
        let _listener = bind_socket(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn peer_uid_matches_self() {
        assert!(peer_uid_allowed(1000, 1000));
        assert!(!peer_uid_allowed(1001, 1000));
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

    #[tokio::test]
    async fn preexisting_parent_must_be_self_or_root() {
        let dir = TempDir::new().unwrap();
        let parent = dir.path().join("helix");
        std::fs::create_dir(&parent).unwrap();
        let owner = std::fs::metadata(&parent).unwrap().uid();
        assert_eq!(owner, current_uid());
        let path = parent.join("helix.sock");
        bind_socket(&path).expect("self-owned parent must bind");
    }

    #[tokio::test]
    async fn tmp_parent_refuses_foreign_owner() {
        let tmp = std::env::temp_dir();
        let owner = std::fs::metadata(&tmp).unwrap().uid();
        let path = tmp.join(format!("helix-bind-uid-test-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let result = bind_socket(&path);
        if owner == 0 || owner == current_uid() {
            let _listener = result.expect("tmp parent owned by self or root should bind");
            let _ = std::fs::remove_file(&path);
        } else {
            let _ = std::fs::remove_file(&path);
            match result {
                Err(BindError::Io(e)) => {
                    assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
                }
                other => panic!("expected PermissionDenied, got {other:?}"),
            }
        }
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

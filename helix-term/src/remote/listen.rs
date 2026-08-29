use std::path::PathBuf;

use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tokio::sync::mpsc;

/// Bind `path` and forward each accepted connection's first read to `tx`.
///
/// Connections are dropped after a single 1024-byte read. Framing and
/// keep-alive are a later change.
pub async fn listen(path: PathBuf, tx: mpsc::Sender<String>) {
    use std::fs::{create_dir, set_permissions, Permissions};
    use std::os::unix::fs::PermissionsExt;

    if let Some(parent_folder) = path.parent() {
        if !parent_folder.exists() {
            if let Err(e) = create_dir(parent_folder) {
                log::error!("Failed to create socket directory: {e}");
                return;
            }
        }
    }

    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            log::error!("Failed to bind listener to socket: {e}");
            return;
        }
    };

    if let Err(e) = set_permissions(&path, Permissions::from_mode(0o600)) {
        log::error!("Failed to set permissions for file: {e}");
    }

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

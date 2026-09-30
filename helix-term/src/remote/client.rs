use std::path::Path;
use std::time::Duration;

use anyhow::anyhow;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

async fn connect(path: &Path) -> anyhow::Result<UnixStream> {
    UnixStream::connect(path)
        .await
        .map_err(|err| anyhow!("helix: could not connect to {} ({err})", path.display()))
}

pub fn normalize_remote_command(cmd: &str) -> String {
    let cmd = cmd.trim();
    if cmd.starts_with('{') || cmd.starts_with(':') {
        cmd.to_string()
    } else {
        format!(":{cmd}")
    }
}

/// Connect, write one command-mode or JSON line, and exit. Does not wait for
/// Helix to finish executing the command.
pub async fn client_remote(path: &Path, cmd: &str) -> anyhow::Result<()> {
    let mut stream = connect(path).await?;
    let line = normalize_remote_command(cmd);
    stream.write_all(line.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.shutdown().await?;
    let mut buf = [0u8; 1024];
    let _ = tokio::time::timeout(Duration::from_millis(100), stream.read(&mut buf)).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_typable_commands() {
        assert_eq!(normalize_remote_command("open foo.rs"), ":open foo.rs");
        assert_eq!(normalize_remote_command(":open foo.rs"), ":open foo.rs");
    }

    #[test]
    fn leaves_json_commands_intact() {
        let json = r#"{"v":1,"op":"command","cmd":":open foo.rs"}"#;
        assert_eq!(normalize_remote_command(json), json);
    }
}

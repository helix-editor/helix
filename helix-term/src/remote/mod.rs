use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

mod client;
mod listen;
mod path;
mod protocol;

pub use client::{client_remote, normalize_remote_command};
pub use listen::{bind_socket, spawn, BindError};
pub use path::{resolve, resolve_from, PathSources};
pub use protocol::{ClientMessage, ClientOp, MAX_LINE, PROTOCOL_V};

/// Whether a TUI Helix process should bind a remote socket.
///
/// True when `--socket` was passed (path optional) or `HELIX_SOCKET_PATH` is
/// set and non-empty. Config `editor.socket-path` never enables listening.
pub fn should_listen(cli_socket: bool) -> bool {
    should_listen_from(
        cli_socket,
        env::var_os("HELIX_SOCKET_PATH").as_deref(),
    )
}

pub fn should_listen_from(cli_socket: bool, env_socket: Option<&OsStr>) -> bool {
    cli_socket || env_socket.is_some_and(|s| !s.is_empty())
}

/// Path used by `--remote`. Loads `editor.socket-path` from config only when
/// CLI and `HELIX_SOCKET_PATH` are both unset.
pub fn resolve_connect_path(cli: Option<&Path>) -> PathBuf {
    let env_set = env::var_os("HELIX_SOCKET_PATH").is_some_and(|s| !s.is_empty());
    let config_path = if cli.is_none() && !env_set {
        crate::config::Config::load_default()
            .ok()
            .and_then(|c| c.editor.socket_path)
    } else {
        None
    };
    resolve(cli, config_path.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_listen_false_by_default() {
        assert!(!should_listen_from(false, None));
        assert!(!should_listen_from(false, Some(OsStr::new(""))));
    }

    #[test]
    fn should_listen_when_cli_socket_set() {
        assert!(should_listen_from(true, None));
    }

    #[test]
    fn should_listen_when_env_set() {
        assert!(should_listen_from(false, Some(OsStr::new("/tmp/hx.sock"))));
    }
}

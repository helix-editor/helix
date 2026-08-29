use std::env;
use std::ffi::OsStr;

mod listen;
mod path;

pub use listen::{bind_socket, spawn, BindError};
pub use path::{resolve, resolve_from, PathSources};

/// Whether a TUI Helix process should bind a remote socket.
///
/// True when `--socket` was passed or `HELIX_SOCKET_PATH` is set and non-empty.
/// Config `editor.socket-path` never enables listening on its own.
pub fn should_listen(cli_socket: bool) -> bool {
    should_listen_from(
        cli_socket,
        env::var_os("HELIX_SOCKET_PATH").as_deref(),
    )
}

pub fn should_listen_from(cli_socket: bool, env_socket: Option<&OsStr>) -> bool {
    cli_socket || env_socket.is_some_and(|s| !s.is_empty())
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

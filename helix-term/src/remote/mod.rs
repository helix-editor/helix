use std::env;

mod listen;
mod path;

pub use listen::{bind_socket, spawn, BindError};
pub use path::{resolve, resolve_from, PathSources};

/// Whether a TUI Helix process should bind a remote socket.
///
/// True when `--socket` was passed or `HELIX_SOCKET_PATH` is set and non-empty.
/// Config `editor.socket-path` never enables listening on its own.
pub fn should_listen(cli_socket: bool) -> bool {
    cli_socket || env::var_os("HELIX_SOCKET_PATH").is_some_and(|s| !s.is_empty())
}

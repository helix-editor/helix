use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Inputs for socket path discovery. Tests pass these explicitly so they do
/// not mutate process environment.
pub struct PathSources<'a> {
    pub cli: Option<&'a Path>,
    pub env_socket: Option<&'a OsStr>,
    pub config_path: Option<&'a Path>,
    pub xdg_runtime_dir: Option<&'a OsStr>,
    pub tmpdir: Option<&'a OsStr>,
    pub uid: u32,
}

/// Resolve the remote socket path from the process environment and the given
/// CLI / config overrides.
///
/// Order (first present wins): `--socket`, `HELIX_SOCKET_PATH`, config
/// `editor.socket-path`, then `$XDG_RUNTIME_DIR/helix/helix.sock` or
/// `$TMPDIR/helix-$UID/helix.sock`.
pub fn resolve(cli: Option<&Path>, config_path: Option<&Path>) -> PathBuf {
    let env_socket = env::var_os("HELIX_SOCKET_PATH");
    let xdg = env::var_os("XDG_RUNTIME_DIR");
    let tmpdir = env::var_os("TMPDIR");
    resolve_from(PathSources {
        cli,
        env_socket: env_socket.as_deref(),
        config_path,
        xdg_runtime_dir: xdg.as_deref(),
        tmpdir: tmpdir.as_deref(),
        uid: uid(),
    })
}

pub fn resolve_from(sources: PathSources<'_>) -> PathBuf {
    if let Some(cli) = sources.cli {
        return cli.to_path_buf();
    }
    if let Some(env_socket) = nonempty(sources.env_socket) {
        return PathBuf::from(env_socket);
    }
    if let Some(config_path) = sources.config_path {
        return config_path.to_path_buf();
    }
    default_path(
        nonempty(sources.xdg_runtime_dir),
        nonempty(sources.tmpdir),
        sources.uid,
    )
}

fn default_path(xdg_runtime_dir: Option<&OsStr>, tmpdir: Option<&OsStr>, uid: u32) -> PathBuf {
    if let Some(xdg) = xdg_runtime_dir {
        return PathBuf::from(xdg).join("helix").join("helix.sock");
    }
    let tmp = tmpdir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    tmp.join(format!("helix-{uid}")).join("helix.sock")
}

fn nonempty(value: Option<&OsStr>) -> Option<&OsStr> {
    value.filter(|s| !s.is_empty())
}

fn uid() -> u32 {
    // SAFETY: getuid has no preconditions and cannot fail.
    unsafe { libc::getuid() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources<'a>(
        cli: Option<&'a Path>,
        env_socket: Option<&'a str>,
        config_path: Option<&'a Path>,
        xdg: Option<&'a str>,
        tmpdir: Option<&'a str>,
    ) -> PathSources<'a> {
        PathSources {
            cli,
            env_socket: env_socket.map(OsStr::new),
            config_path,
            xdg_runtime_dir: xdg.map(OsStr::new),
            tmpdir: tmpdir.map(OsStr::new),
            uid: 1000,
        }
    }

    #[test]
    fn cli_wins_over_env_and_config() {
        let path = resolve_from(sources(
            Some(Path::new("/cli.sock")),
            Some("/env.sock"),
            Some(Path::new("/cfg.sock")),
            Some("/run/user/1000"),
            Some("/tmp"),
        ));
        assert_eq!(path, PathBuf::from("/cli.sock"));
    }

    #[test]
    fn env_wins_over_config() {
        let path = resolve_from(sources(
            None,
            Some("/env.sock"),
            Some(Path::new("/cfg.sock")),
            Some("/run/user/1000"),
            Some("/tmp"),
        ));
        assert_eq!(path, PathBuf::from("/env.sock"));
    }

    #[test]
    fn config_wins_over_default() {
        let path = resolve_from(sources(
            None,
            None,
            Some(Path::new("/cfg.sock")),
            Some("/run/user/1000"),
            Some("/tmp"),
        ));
        assert_eq!(path, PathBuf::from("/cfg.sock"));
    }

    #[test]
    fn xdg_default() {
        let path = resolve_from(sources(
            None,
            None,
            None,
            Some("/run/user/1000"),
            Some("/var/tmp"),
        ));
        assert_eq!(
            path,
            PathBuf::from("/run/user/1000/helix/helix.sock")
        );
    }

    #[test]
    fn tmpdir_uid_default_when_xdg_unset() {
        let path = resolve_from(sources(None, None, None, None, Some("/var/tmp")));
        assert_eq!(path, PathBuf::from("/var/tmp/helix-1000/helix.sock"));
    }

    #[test]
    fn tmp_when_no_tmpdir() {
        let path = resolve_from(sources(None, None, None, None, None));
        assert_eq!(path, PathBuf::from("/tmp/helix-1000/helix.sock"));
    }

    #[test]
    fn empty_env_socket_treated_as_unset() {
        let path = resolve_from(sources(
            None,
            Some(""),
            Some(Path::new("/cfg.sock")),
            None,
            None,
        ));
        assert_eq!(path, PathBuf::from("/cfg.sock"));
    }

    #[test]
    fn empty_xdg_falls_through_to_tmpdir() {
        let path = resolve_from(sources(None, None, None, Some(""), Some("/var/tmp")));
        assert_eq!(path, PathBuf::from("/var/tmp/helix-1000/helix.sock"));
    }
}

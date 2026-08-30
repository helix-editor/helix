use helix_core::command_line::{split, ExpansionKind, TokenKind, Tokenizer};

use crate::commands::typed::TYPABLE_COMMAND_MAP;

/// Canonical typable-command names forbidden on the socket.
///
/// Aliases resolve through [`TYPABLE_COMMAND_MAP`] to the same `name`, so they
/// do not need their own entries. `%sh{...}` is not a command name and is
/// rejected separately by tokenizing the line for shell expansions.
const DENYLIST: &[&str] = &[
    "run-shell-command",
    "insert-output",
    "append-output",
    "pipe",
    "pipe-to",
    "write",
    "write!",
    "write-buffer-close",
    "write-buffer-close!",
    "write-quit",
    "write-quit!",
    "write-all",
    "write-all!",
    "write-quit-all",
    "write-quit-all!",
    "update",
    "exit",
    "exit!",
    "move",
    "move!",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboundDeny {
    Command(&'static str),
    ShellExpansion,
}

impl InboundDeny {
    pub fn message(self) -> String {
        match self {
            Self::Command(name) => format!("Running command {name} is forbidden from socket"),
            Self::ShellExpansion => "shell expansion is forbidden from socket".into(),
        }
    }
}

/// Whether a socket-originated command line must be rejected before execution.
///
/// `line` may include a leading `:`. Empty input is allowed (no-op).
pub fn deny_inbound_command(line: &str) -> Option<InboundDeny> {
    let line = line.trim_end_matches('\r').trim();
    if line.is_empty() {
        return None;
    }
    let line = line.strip_prefix(':').unwrap_or(line);

    if contains_shell_expansion(line) {
        return Some(InboundDeny::ShellExpansion);
    }

    let (name, _, _) = split(line);
    if let Some(cmd) = TYPABLE_COMMAND_MAP.get(name) {
        if DENYLIST.contains(&cmd.name) {
            return Some(InboundDeny::Command(cmd.name));
        }
    }
    None
}

fn contains_shell_expansion(input: &str) -> bool {
    for token in Tokenizer::new(input, false).flatten() {
        match token.kind {
            TokenKind::Expansion(ExpansionKind::Shell) => return true,
            TokenKind::Expand if contains_shell_expansion(&token.content) => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn denied_command(line: &str) -> &'static str {
        match deny_inbound_command(line) {
            Some(InboundDeny::Command(name)) => name,
            other => panic!("expected command deny for {line:?}, got {other:?}"),
        }
    }

    fn denied_shell(line: &str) {
        assert_eq!(
            deny_inbound_command(line),
            Some(InboundDeny::ShellExpansion),
            "{line:?}"
        );
    }

    #[test]
    fn allows_open() {
        assert_eq!(deny_inbound_command("open /abs/file.rs"), None);
        assert_eq!(deny_inbound_command(":open /abs/file.rs"), None);
    }

    #[test]
    fn allows_single_quoted_sh_literal() {
        assert_eq!(deny_inbound_command("open '%sh{echo /tmp/x}'"), None);
    }

    #[test]
    fn allows_empty() {
        assert_eq!(deny_inbound_command(""), None);
        assert_eq!(deny_inbound_command("  "), None);
    }

    #[test]
    fn denylist_shell_commands_and_aliases() {
        assert_eq!(
            denied_command("run-shell-command echo hi"),
            "run-shell-command"
        );
        assert_eq!(denied_command("sh echo hi"), "run-shell-command");
        assert_eq!(denied_command("! echo hi"), "run-shell-command");
        assert_eq!(denied_command("insert-output echo hi"), "insert-output");
        assert_eq!(denied_command("append-output echo hi"), "append-output");
        assert_eq!(denied_command("pipe cat"), "pipe");
        assert_eq!(denied_command("| cat"), "pipe");
        assert_eq!(denied_command("pipe-to cat"), "pipe-to");
    }

    #[test]
    fn denylist_writes_and_aliases() {
        assert_eq!(denied_command("write"), "write");
        assert_eq!(denied_command("w"), "write");
        assert_eq!(denied_command("write!"), "write!");
        assert_eq!(denied_command("update"), "update");
        assert_eq!(denied_command("u"), "update");
        assert_eq!(denied_command("exit"), "exit");
        assert_eq!(denied_command("x"), "exit");
        assert_eq!(denied_command("exit!"), "exit!");
        assert_eq!(denied_command("move /tmp/x"), "move");
        assert_eq!(denied_command("mv /tmp/x"), "move");
        assert_eq!(denied_command("move! /tmp/x"), "move!");
        assert_eq!(denied_command("mv! /tmp/x"), "move!");
        assert_eq!(denied_command("write-quit-all"), "write-quit-all");
        assert_eq!(denied_command("xa"), "write-quit-all");
    }

    #[test]
    fn denylist_shell_expansion_bypass() {
        denied_shell("open %sh{echo /etc/passwd}");
        denied_shell(":open %sh{id}");
        denied_shell(r#"open "foo %sh{echo x}""#);
        denied_shell("echo %sh{uname}");
    }
}

use serde::Deserialize;

pub const MAX_LINE: usize = 64 * 1024;
pub const PROTOCOL_V: u32 = 1;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ClientMessage {
    pub v: u32,
    #[serde(flatten)]
    pub op: ClientOp,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum ClientOp {
    Command { cmd: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command() {
        let msg: ClientMessage =
            serde_json::from_str(r#"{"v":1,"op":"command","cmd":":open \"/abs/foo.rs:12\""}"#)
                .unwrap();
        assert_eq!(msg.v, 1);
        assert_eq!(
            msg.op,
            ClientOp::Command {
                cmd: r#":open "/abs/foo.rs:12""#.into()
            }
        );
    }
}

use noncity_net::RawPacket;
use noncity_proto::io::read_string;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyCommand {
    Server(Option<String>),
    Glist,
    Noncity,
}

pub fn extract_proxy_command(packet: &RawPacket) -> Option<ProxyCommand> {
    let mut slice = packet.payload.as_ref();
    let text = read_string(&mut slice, 256).ok()?;

    let cmd_str = text.strip_prefix('/').unwrap_or(&text).trim();
    let mut parts = cmd_str.split_whitespace();
    let root = parts.next()?;

    match root {
        "server" => {
            let target = parts.next().map(|s| s.to_string());
            Some(ProxyCommand::Server(target))
        }
        "glist" => Some(ProxyCommand::Glist),
        "noncity" => Some(ProxyCommand::Noncity),
        _ => None,
    }
}

pub fn make_chat_json(message: &str) -> String {
    let escaped = serde_json::to_string(message).unwrap_or_else(|_| format!("\"{}\"", message));
    format!(r#"{{"text":{}}}"#, escaped)
}

use noncity_net::RawPacket;
use noncity_proto::io::read_string;
use noncity_proto::packet::play::{CommandSuggestionMatch, CommandSuggestionsResponse};
use noncity_proto::varint::VarInt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyCommand {
    Server(Option<String>),
    Glist,
    Noncity,
    PluginCustom(String),
}

pub fn is_chat_command_packet(packet_id: i32, protocol_version: i32) -> bool {
    if protocol_version >= 766 {
        packet_id == 0x04 || packet_id == 0x05
    } else if protocol_version >= 761 {
        packet_id == 0x03 || packet_id == 0x04
    } else if protocol_version >= 759 {
        packet_id == 0x04 || packet_id == 0x05
    } else {
        packet_id == 0x03 || packet_id == 0x02 || packet_id == 0x01
    }
}

pub fn is_tab_complete_packet(packet_id: i32, protocol_version: i32) -> bool {
    if protocol_version >= 766 {
        packet_id == 0x09
    } else if protocol_version >= 761 {
        packet_id == 0x08
    } else if protocol_version >= 759 {
        packet_id == 0x08
    } else if protocol_version >= 477 {
        packet_id == 0x06
    } else if protocol_version >= 393 {
        packet_id == 0x05
    } else {
        packet_id == 0x01 || packet_id == 0x14
    }
}

pub fn extract_proxy_command(packet: &RawPacket, protocol_version: i32) -> Option<ProxyCommand> {
    if !is_chat_command_packet(packet.id, protocol_version) {
        return None;
    }

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
        _ => Some(ProxyCommand::PluginCustom(cmd_str.to_string())),
    }
}

pub fn handle_tab_complete(
    packet: &RawPacket,
    server_names: &[String],
    protocol_version: i32,
) -> Option<CommandSuggestionsResponse> {
    if !is_tab_complete_packet(packet.id, protocol_version) {
        return None;
    }

    let mut slice = packet.payload.as_ref();
    let transaction_id = VarInt::decode(&mut slice).ok()?;
    let text = read_string(&mut slice, 256).ok()?;

    if !text.starts_with("/server") {
        return None;
    }

    let parts: Vec<&str> = text.split_whitespace().collect();
    let query = if text.ends_with(' ') {
        ""
    } else {
        parts.get(1).copied().unwrap_or("")
    };

    let start_pos = text.rfind(' ').map(|p| p + 1).unwrap_or(text.len());
    let mut matches = Vec::new();

    for name in server_names {
        if name.starts_with(query) {
            matches.push(CommandSuggestionMatch {
                value: name.clone(),
                tooltip: None,
            });
        }
    }

    Some(CommandSuggestionsResponse {
        transaction_id,
        start: VarInt(start_pos as i32),
        length: VarInt(query.len() as i32),
        matches,
    })
}

pub fn make_chat_json(message: &str) -> String {
    let escaped = serde_json::to_string(message).unwrap_or_else(|_| format!("\"{}\"", message));
    format!(r#"{{"text":{}}}"#, escaped)
}

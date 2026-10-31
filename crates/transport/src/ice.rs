//! Interactive Connectivity Establishment (ICE), STUN, and TURN relay support.
//!
//! Standards implemented:
//! - RFC 8445 / RFC 5245: ICE candidate format, foundation, and priority calculations.
//! - RFC 8489 / RFC 5389: STUN Binding Request and XOR-MAPPED-ADDRESS resolution.
//! - RFC 8656 / RFC 5766: TURN Allocate Request and XOR-RELAYED-ADDRESS resolution for relay fallback.

use crate::TransportError;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;

/// Magic Cookie constant for STUN/TURN (RFC 8489).
pub const STUN_MAGIC_COOKIE: u32 = 0x2112_A442;

/// STUN message types.
pub const STUN_BINDING_REQUEST: u16 = 0x0001;
pub const STUN_BINDING_RESPONSE: u16 = 0x0101;
pub const TURN_ALLOCATE_REQUEST: u16 = 0x0003;
pub const TURN_ALLOCATE_RESPONSE: u16 = 0x0103;

/// STUN/TURN Attribute types.
pub const ATTR_MAPPED_ADDRESS: u16 = 0x0001;
pub const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;
pub const ATTR_XOR_RELAYED_ADDRESS: u16 = 0x0016;
pub const ATTR_LIFETIME: u16 = 0x000D;
pub const ATTR_REQUESTED_TRANSPORT: u16 = 0x0019;

/// ICE candidate transport protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportProtocol {
    Udp,
    Tcp,
}

impl std::fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Udp => write!(f, "UDP"),
            Self::Tcp => write!(f, "TCP"),
        }
    }
}

/// ICE candidate type (RFC 8445 Section 5.1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CandidateType {
    /// Local network interface address.
    Host,
    /// Public server reflexive address discovered via STUN.
    Srflx,
    /// Peer reflexive address learned during connectivity checks.
    Prflx,
    /// Relayed address allocated on a TURN server.
    Relay,
}

impl CandidateType {
    /// Recommended type preference values from RFC 8445 Section 5.1.2.2.
    pub fn type_preference(&self) -> u32 {
        match self {
            Self::Host => 126,
            Self::Prflx => 110,
            Self::Srflx => 100,
            Self::Relay => 0,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Srflx => "srflx",
            Self::Prflx => "prflx",
            Self::Relay => "relay",
        }
    }
}

/// A parsed ICE candidate (RFC 5245 / RFC 8445).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IceCandidate {
    pub foundation: String,
    pub component: u32,
    pub protocol: TransportProtocol,
    pub priority: u32,
    pub ip: IpAddr,
    pub port: u16,
    pub candidate_type: CandidateType,
    pub rel_addr: Option<IpAddr>,
    pub rel_port: Option<u16>,
}

impl IceCandidate {
    /// Create a new ICE candidate with computed RFC 8445 priority.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        foundation: &str,
        component: u32,
        protocol: TransportProtocol,
        ip: IpAddr,
        port: u16,
        candidate_type: CandidateType,
        rel_addr: Option<IpAddr>,
        rel_port: Option<u16>,
    ) -> Self {
        let type_pref = candidate_type.type_preference();
        let local_pref = 65535; // Default single interface priority
        let priority = (type_pref << 24) | (local_pref << 8) | (256 - component);

        Self {
            foundation: foundation.to_string(),
            component,
            protocol,
            priority,
            ip,
            port,
            candidate_type,
            rel_addr,
            rel_port,
        }
    }

    /// Format candidate according to RFC 5245 SDP candidate attribute line:
    /// `candidate:foundation component protocol priority ip port typ type [raddr ... rport ...]`
    pub fn to_sdp_attribute(&self) -> String {
        let mut s = format!(
            "candidate:{} {} {} {} {} {} typ {}",
            self.foundation,
            self.component,
            self.protocol,
            self.priority,
            self.ip,
            self.port,
            self.candidate_type.as_str()
        );

        if let (Some(raddr), Some(rport)) = (self.rel_addr, self.rel_port) {
            s.push_str(&format!(" raddr {} rport {}", raddr, rport));
        }

        s
    }

    /// Parse an RFC 5245 candidate string.
    pub fn parse_sdp_attribute(s: &str) -> Result<Self, TransportError> {
        let trimmed = s.trim().strip_prefix("candidate:").unwrap_or(s.trim());
        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        if parts.len() < 8 {
            return Err(TransportError::ReceiveFailed(format!(
                "invalid candidate string: insufficient tokens: '{}'",
                s
            )));
        }

        let foundation = parts[0].to_string();
        let component: u32 = parts[1]
            .parse()
            .map_err(|e| TransportError::ReceiveFailed(format!("invalid component: {}", e)))?;

        let protocol = match parts[2].to_uppercase().as_str() {
            "UDP" => TransportProtocol::Udp,
            "TCP" => TransportProtocol::Tcp,
            other => {
                return Err(TransportError::ReceiveFailed(format!(
                    "unsupported candidate protocol: {}",
                    other
                )))
            }
        };

        let priority: u32 = parts[3]
            .parse()
            .map_err(|e| TransportError::ReceiveFailed(format!("invalid priority: {}", e)))?;
        let ip: IpAddr = parts[4]
            .parse()
            .map_err(|e| TransportError::ReceiveFailed(format!("invalid IP: {}", e)))?;
        let port: u16 = parts[5]
            .parse()
            .map_err(|e| TransportError::ReceiveFailed(format!("invalid port: {}", e)))?;

        if parts[6] != "typ" {
            return Err(TransportError::ReceiveFailed(format!(
                "expected 'typ' keyword in candidate, found '{}'",
                parts[6]
            )));
        }

        let candidate_type = match parts[7] {
            "host" => CandidateType::Host,
            "srflx" => CandidateType::Srflx,
            "prflx" => CandidateType::Prflx,
            "relay" => CandidateType::Relay,
            other => {
                return Err(TransportError::ReceiveFailed(format!(
                    "unknown candidate type: {}",
                    other
                )))
            }
        };

        let mut rel_addr = None;
        let mut rel_port = None;

        let mut i = 8;
        while i + 1 < parts.len() {
            match parts[i] {
                "raddr" => {
                    rel_addr = parts[i + 1].parse().ok();
                    i += 2;
                }
                "rport" => {
                    rel_port = parts[i + 1].parse().ok();
                    i += 2;
                }
                _ => i += 1,
            }
        }

        Ok(Self {
            foundation,
            component,
            protocol,
            priority,
            ip,
            port,
            candidate_type,
            rel_addr,
            rel_port,
        })
    }
}

/// Result of an active TURN allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnAllocation {
    pub relayed_addr: SocketAddr,
    pub lifetime_secs: u32,
}

/// Build a STUN Binding Request packet (RFC 8489 Section 5).
/// Returns (packet_bytes, transaction_id).
pub fn build_stun_binding_request(transaction_id: [u8; 12]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(20);
    // 1. Message Type: 0x0001 (Binding Request)
    packet.extend_from_slice(&STUN_BINDING_REQUEST.to_be_bytes());
    // 2. Message Length: 0 (no attributes in basic binding request)
    packet.extend_from_slice(&0u16.to_be_bytes());
    // 3. Magic Cookie: 0x2112A442
    packet.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
    // 4. Transaction ID: 12 bytes
    packet.extend_from_slice(&transaction_id);
    packet
}

/// Parse a STUN Binding Response and extract XOR-MAPPED-ADDRESS or MAPPED-ADDRESS.
pub fn parse_stun_binding_response(
    data: &[u8],
    expected_tx_id: &[u8; 12],
) -> Result<SocketAddr, TransportError> {
    if data.len() < 20 {
        return Err(TransportError::ReceiveFailed(
            "STUN packet too short (< 20 bytes)".into(),
        ));
    }

    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != STUN_BINDING_RESPONSE {
        return Err(TransportError::ReceiveFailed(format!(
            "Unexpected STUN response message type: 0x{:04x}",
            msg_type
        )));
    }

    let magic = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if magic != STUN_MAGIC_COOKIE {
        return Err(TransportError::ReceiveFailed(
            "Invalid STUN magic cookie".into(),
        ));
    }

    if &data[8..20] != expected_tx_id {
        return Err(TransportError::ReceiveFailed(
            "STUN transaction ID mismatch".into(),
        ));
    }

    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    let payload = &data[20..std::cmp::min(data.len(), 20 + msg_len)];

    parse_xor_mapped_address(payload, expected_tx_id)
}

/// Build a TURN Allocate Request packet (RFC 8656 Section 6.1).
pub fn build_turn_allocate_request(transaction_id: [u8; 12]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(28);
    // Header
    packet.extend_from_slice(&TURN_ALLOCATE_REQUEST.to_be_bytes());
    // Attribute: REQUESTED-TRANSPORT (type 0x0019, length 4, UDP = 17 << 24)
    let attr_len = 8u16;
    packet.extend_from_slice(&attr_len.to_be_bytes());
    packet.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
    packet.extend_from_slice(&transaction_id);

    // REQUESTED-TRANSPORT attribute (type = 0x0019, len = 4)
    packet.extend_from_slice(&ATTR_REQUESTED_TRANSPORT.to_be_bytes());
    packet.extend_from_slice(&4u16.to_be_bytes());
    packet.push(17); // Protocol 17 = UDP
    packet.push(0);
    packet.push(0);
    packet.push(0);

    packet
}

/// Parse a TURN Allocate Response packet and extract XOR-RELAYED-ADDRESS and LIFETIME.
pub fn parse_turn_allocate_response(
    data: &[u8],
    expected_tx_id: &[u8; 12],
) -> Result<TurnAllocation, TransportError> {
    if data.len() < 20 {
        return Err(TransportError::ReceiveFailed(
            "TURN packet too short (< 20 bytes)".into(),
        ));
    }

    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != TURN_ALLOCATE_RESPONSE {
        return Err(TransportError::ReceiveFailed(format!(
            "Unexpected TURN response type: 0x{:04x}",
            msg_type
        )));
    }

    let magic = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if magic != STUN_MAGIC_COOKIE {
        return Err(TransportError::ReceiveFailed(
            "Invalid TURN magic cookie".into(),
        ));
    }

    if &data[8..20] != expected_tx_id {
        return Err(TransportError::ReceiveFailed(
            "TURN transaction ID mismatch".into(),
        ));
    }

    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    let payload = &data[20..std::cmp::min(data.len(), 20 + msg_len)];

    let mut relayed_addr = None;
    let mut lifetime_secs = 600; // default 10 min

    let mut offset = 0;
    while offset + 4 <= payload.len() {
        let attr_type = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        let attr_len = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]) as usize;
        offset += 4;

        if offset + attr_len > payload.len() {
            break;
        }

        let attr_val = &payload[offset..offset + attr_len];

        match attr_type {
            ATTR_XOR_RELAYED_ADDRESS => {
                if let Ok(addr) = decode_xor_address(attr_val, expected_tx_id) {
                    relayed_addr = Some(addr);
                }
            }
            ATTR_LIFETIME if attr_val.len() >= 4 => {
                lifetime_secs =
                    u32::from_be_bytes([attr_val[0], attr_val[1], attr_val[2], attr_val[3]]);
            }
            _ => {}
        }

        // Attributes are padded to 4-byte boundaries (RFC 5389 Section 15)
        let padding = (4 - (attr_len % 4)) % 4;
        offset += attr_len + padding;
    }

    let addr = relayed_addr.ok_or_else(|| {
        TransportError::ReceiveFailed(
            "Missing XOR-RELAYED-ADDRESS attribute in TURN response".into(),
        )
    })?;

    Ok(TurnAllocation {
        relayed_addr: addr,
        lifetime_secs,
    })
}

fn parse_xor_mapped_address(
    payload: &[u8],
    tx_id: &[u8; 12],
) -> Result<SocketAddr, TransportError> {
    let mut offset = 0;
    while offset + 4 <= payload.len() {
        let attr_type = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
        let attr_len = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]) as usize;
        offset += 4;

        if offset + attr_len > payload.len() {
            break;
        }

        let attr_val = &payload[offset..offset + attr_len];

        if attr_type == ATTR_XOR_MAPPED_ADDRESS {
            return decode_xor_address(attr_val, tx_id);
        }

        // Padding to 4 bytes
        let padding = (4 - (attr_len % 4)) % 4;
        offset += attr_len + padding;
    }

    Err(TransportError::ReceiveFailed(
        "Missing XOR-MAPPED-ADDRESS attribute in STUN response".into(),
    ))
}

fn decode_xor_address(val: &[u8], tx_id: &[u8; 12]) -> Result<SocketAddr, TransportError> {
    if val.len() < 8 {
        return Err(TransportError::ReceiveFailed(
            "XOR-Address attribute too short".into(),
        ));
    }

    let family = val[1];
    let x_port = u16::from_be_bytes([val[2], val[3]]);
    let port = x_port ^ ((STUN_MAGIC_COOKIE >> 16) as u16);

    match family {
        0x01 => {
            // IPv4 (4 bytes)
            let x_ip = u32::from_be_bytes([val[4], val[5], val[6], val[7]]);
            let ip_u32 = x_ip ^ STUN_MAGIC_COOKIE;
            let ip = Ipv4Addr::from(ip_u32);
            Ok(SocketAddr::new(IpAddr::V4(ip), port))
        }
        0x02 => {
            // IPv6 (16 bytes)
            if val.len() < 20 {
                return Err(TransportError::ReceiveFailed(
                    "IPv6 XOR-Address attribute too short".into(),
                ));
            }
            let mut xor_key = [0u8; 16];
            xor_key[..4].copy_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
            xor_key[4..16].copy_from_slice(tx_id);

            let mut ip_bytes = [0u8; 16];
            for i in 0..16 {
                ip_bytes[i] = val[4 + i] ^ xor_key[i];
            }
            let ip = std::net::Ipv6Addr::from(ip_bytes);
            Ok(SocketAddr::new(IpAddr::V6(ip), port))
        }
        other => Err(TransportError::ReceiveFailed(format!(
            "Unsupported address family in XOR-Address: 0x{:02x}",
            other
        ))),
    }
}

/// Query a STUN server to discover our public server-reflexive endpoint.
pub async fn query_stun(
    stun_server: SocketAddr,
    local_socket: &UdpSocket,
    timeout: Duration,
) -> Result<SocketAddr, TransportError> {
    let mut tx_id = [0u8; 12];
    for b in tx_id.iter_mut() {
        *b = rand::random();
    }

    let req = build_stun_binding_request(tx_id);
    local_socket
        .send_to(&req, stun_server)
        .await
        .map_err(|e| TransportError::SendFailed(e.to_string()))?;

    let mut buf = [0u8; 1024];
    let res = tokio::time::timeout(timeout, local_socket.recv_from(&mut buf)).await;

    match res {
        Ok(Ok((len, _from))) => parse_stun_binding_response(&buf[..len], &tx_id),
        Ok(Err(e)) => Err(TransportError::ReceiveFailed(e.to_string())),
        Err(_) => Err(TransportError::Timeout),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_sdp_attribute_round_trip() {
        let candidate = IceCandidate::new(
            "1",
            1,
            TransportProtocol::Udp,
            "192.168.1.100".parse().unwrap(),
            5004,
            CandidateType::Host,
            None,
            None,
        );

        let attr = candidate.to_sdp_attribute();
        assert!(attr.contains("typ host"));

        let parsed = IceCandidate::parse_sdp_attribute(&attr).unwrap();
        assert_eq!(candidate, parsed);

        // Server reflexive candidate with raddr and rport
        let srflx_candidate = IceCandidate::new(
            "2",
            1,
            TransportProtocol::Udp,
            "203.0.113.5".parse().unwrap(),
            5004,
            CandidateType::Srflx,
            Some("192.168.1.100".parse().unwrap()),
            Some(5004),
        );

        let srflx_attr = srflx_candidate.to_sdp_attribute();
        assert!(srflx_attr.contains("typ srflx"));
        assert!(srflx_attr.contains("raddr 192.168.1.100"));
        assert!(srflx_attr.contains("rport 5004"));

        let srflx_parsed = IceCandidate::parse_sdp_attribute(&srflx_attr).unwrap();
        assert_eq!(srflx_candidate, srflx_parsed);
    }

    #[test]
    fn stun_binding_request_and_response_parsing() {
        let tx_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let req = build_stun_binding_request(tx_id);
        assert_eq!(req.len(), 20);

        // Construct mock STUN Binding Response with XOR-MAPPED-ADDRESS
        // Expected public IP: 198.51.100.1, Port: 32768
        let target_ip = Ipv4Addr::new(198, 51, 100, 1);
        let target_port: u16 = 32768;

        let mut resp = Vec::new();
        resp.extend_from_slice(&STUN_BINDING_RESPONSE.to_be_bytes()); // Type: 0x0101
        let attr_len = 12u16; // 8 bytes attribute value + 4 bytes header
        resp.extend_from_slice(&attr_len.to_be_bytes());
        resp.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
        resp.extend_from_slice(&tx_id);

        // Attribute XOR-MAPPED-ADDRESS
        resp.extend_from_slice(&ATTR_XOR_MAPPED_ADDRESS.to_be_bytes());
        resp.extend_from_slice(&8u16.to_be_bytes()); // Length 8
        resp.push(0); // Reserved
        resp.push(1); // IPv4
        let xor_port = target_port ^ ((STUN_MAGIC_COOKIE >> 16) as u16);
        resp.extend_from_slice(&xor_port.to_be_bytes());
        let xor_ip = u32::from(target_ip) ^ STUN_MAGIC_COOKIE;
        resp.extend_from_slice(&xor_ip.to_be_bytes());

        let discovered = parse_stun_binding_response(&resp, &tx_id).unwrap();
        assert_eq!(discovered.ip(), IpAddr::V4(target_ip));
        assert_eq!(discovered.port(), target_port);
    }

    #[test]
    fn turn_allocate_request_and_response_parsing() {
        let tx_id = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2];
        let req = build_turn_allocate_request(tx_id);
        assert!(req.len() >= 20);

        // Construct mock TURN Allocate Response with XOR-RELAYED-ADDRESS and LIFETIME
        let relay_ip = Ipv4Addr::new(192, 0, 2, 88);
        let relay_port: u16 = 49152;

        let mut resp = Vec::new();
        resp.extend_from_slice(&TURN_ALLOCATE_RESPONSE.to_be_bytes()); // Type: 0x0103
        let attr_len = 20u16; // 12 bytes XOR addr + 8 bytes lifetime
        resp.extend_from_slice(&attr_len.to_be_bytes());
        resp.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
        resp.extend_from_slice(&tx_id);

        // XOR-RELAYED-ADDRESS attribute
        resp.extend_from_slice(&ATTR_XOR_RELAYED_ADDRESS.to_be_bytes());
        resp.extend_from_slice(&8u16.to_be_bytes());
        resp.push(0);
        resp.push(1); // IPv4
        let xor_port = relay_port ^ ((STUN_MAGIC_COOKIE >> 16) as u16);
        resp.extend_from_slice(&xor_port.to_be_bytes());
        let xor_ip = u32::from(relay_ip) ^ STUN_MAGIC_COOKIE;
        resp.extend_from_slice(&xor_ip.to_be_bytes());

        // LIFETIME attribute
        resp.extend_from_slice(&ATTR_LIFETIME.to_be_bytes());
        resp.extend_from_slice(&4u16.to_be_bytes());
        resp.extend_from_slice(&300u32.to_be_bytes()); // 300 seconds

        let alloc = parse_turn_allocate_response(&resp, &tx_id).unwrap();
        assert_eq!(alloc.relayed_addr.ip(), IpAddr::V4(relay_ip));
        assert_eq!(alloc.relayed_addr.port(), relay_port);
        assert_eq!(alloc.lifetime_secs, 300);
    }
}

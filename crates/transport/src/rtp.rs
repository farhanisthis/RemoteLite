//! RFC 3550 Real-Time Transport Protocol (RTP) and RFC 6184 H.264 payload formatting.

use bytes::{BufMut, Bytes, BytesMut};

/// Standard RTP header size in bytes.
pub const RTP_HEADER_SIZE: usize = 12;

/// Maximum payload size per RTP packet (Ethernet MTU 1500 - IP 20 - UDP 8 - RTP 12 - safe margin).
pub const MAX_RTP_PAYLOAD_SIZE: usize = 1200;

/// Standard RTP Header (RFC 3550 §5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtpHeader {
    pub version: u8,
    pub padding: bool,
    pub extension: bool,
    pub marker: bool,
    pub payload_type: u8,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
}

impl Default for RtpHeader {
    fn default() -> Self {
        Self {
            version: 2,
            padding: false,
            extension: false,
            marker: false,
            payload_type: 96, // Dynamic payload type commonly used for H.264
            sequence_number: 0,
            timestamp: 0,
            ssrc: 0x12345678,
        }
    }
}

impl RtpHeader {
    /// Serialize RTP header into a 12-byte buffer.
    pub fn write_to(&self, buf: &mut BytesMut) {
        let b0 = (self.version << 6)
            | (if self.padding { 1 << 5 } else { 0 })
            | (if self.extension { 1 << 4 } else { 0 });
        let b1 = (if self.marker { 1 << 7 } else { 0 }) | (self.payload_type & 0x7F);

        buf.put_u8(b0);
        buf.put_u8(b1);
        buf.put_u16(self.sequence_number);
        buf.put_u32(self.timestamp);
        buf.put_u32(self.ssrc);
    }
}

/// A complete RTP packet ready for transmission over the network.
#[derive(Debug, Clone)]
pub struct RtpPacket {
    pub header: RtpHeader,
    pub payload: Bytes,
}

impl RtpPacket {
    /// Serialize the entire RTP packet (header + payload) to bytes.
    pub fn serialize(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(RTP_HEADER_SIZE + self.payload.len());
        self.header.write_to(&mut buf);
        buf.put_slice(&self.payload);
        buf.freeze()
    }
}

/// Packetize an H.264 NAL unit into one or more RTP packets according to RFC 6184.
///
/// If `nalu.len() <= MAX_RTP_PAYLOAD_SIZE`, outputs a Single NAL Unit packet.
/// If `nalu.len() > MAX_RTP_PAYLOAD_SIZE`, outputs Fragmented Unit (FU-A) packets.
pub fn packetize_h264_nalu(
    nalu: &[u8],
    header_template: &RtpHeader,
    next_sequence: &mut u16,
    is_last_nalu_in_frame: bool,
) -> Vec<RtpPacket> {
    if nalu.is_empty() {
        return Vec::new();
    }

    let mut packets = Vec::new();

    if nalu.len() <= MAX_RTP_PAYLOAD_SIZE {
        // Single NAL Unit Packet (RFC 6184 §5.6)
        let mut header = header_template.clone();
        header.sequence_number = *next_sequence;
        *next_sequence = next_sequence.wrapping_add(1);
        header.marker = is_last_nalu_in_frame;

        packets.push(RtpPacket {
            header,
            payload: Bytes::copy_from_slice(nalu),
        });
    } else {
        // Fragmentation Unit (FU-A) Packet (RFC 6184 §5.8)
        let nalu_header = nalu[0];
        let fnr = nalu_header & 0xE0; // forbidden_zero_bit + nal_ref_idc
        let nalu_type = nalu_header & 0x1F;

        // FU indicator: FNR bits + Type 28 (FU-A)
        let fu_indicator = fnr | 28;
        let payload_data = &nalu[1..];
        let chunks = payload_data.chunks(MAX_RTP_PAYLOAD_SIZE - 2);
        let total_chunks = chunks.len();

        for (idx, chunk) in chunks.enumerate() {
            let is_start = idx == 0;
            let is_end = idx == total_chunks - 1;

            let mut fu_header = nalu_type;
            if is_start {
                fu_header |= 0x80; // S bit
            }
            if is_end {
                fu_header |= 0x40; // E bit
            }

            let mut fu_payload = BytesMut::with_capacity(2 + chunk.len());
            fu_payload.put_u8(fu_indicator);
            fu_payload.put_u8(fu_header);
            fu_payload.put_slice(chunk);

            let mut header = header_template.clone();
            header.sequence_number = *next_sequence;
            *next_sequence = next_sequence.wrapping_add(1);
            header.marker = is_last_nalu_in_frame && is_end;

            packets.push(RtpPacket {
                header,
                payload: fu_payload.freeze(),
            });
        }
    }

    packets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtp_header_serialization() {
        let header = RtpHeader {
            version: 2,
            padding: false,
            extension: false,
            marker: true,
            payload_type: 96,
            sequence_number: 1001,
            timestamp: 90000,
            ssrc: 0x11223344,
        };

        let mut buf = BytesMut::new();
        header.write_to(&mut buf);
        assert_eq!(buf.len(), 12);
        assert_eq!(buf[0], 0x80); // V=2
        assert_eq!(buf[1], 0x80 | 96); // M=1, PT=96
    }

    #[test]
    fn packetize_small_nalu_is_single_packet() {
        let nalu = vec![0x67, 0x42, 0x80, 0x1F]; // SPS
        let mut seq = 1;
        let packets = packetize_h264_nalu(&nalu, &RtpHeader::default(), &mut seq, false);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].payload.as_ref(), &nalu[..]);
        assert_eq!(seq, 2);
    }

    #[test]
    fn packetize_large_nalu_uses_fu_a_fragmentation() {
        let large_nalu = vec![0x65; 3000]; // IDR slice > 1200 MTU
        let mut seq = 10;
        let packets = packetize_h264_nalu(&large_nalu, &RtpHeader::default(), &mut seq, true);
        assert!(
            packets.len() > 1,
            "Must fragment into multiple FU-A packets"
        );
        assert!(
            packets.last().unwrap().header.marker,
            "Last packet must have marker bit set"
        );
    }
}

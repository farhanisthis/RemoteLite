//! RFC 4566 / RFC 8866 Session Description Protocol (SDP) for WebRTC H.264 video.

use crate::TransportError;

/// Parameters for generating a WebRTC SDP offer or answer.
#[derive(Debug, Clone)]
pub struct SdpSessionParams {
    pub session_id: String,
    pub host_ip: String,
    pub video_port: u16,
    pub ice_ufrag: String,
    pub ice_pwd: String,
    pub dtls_fingerprint_sha256: String,
    pub ssrc: u32,
}

impl Default for SdpSessionParams {
    fn default() -> Self {
        Self {
            session_id: "142981729837".to_string(),
            host_ip: "127.0.0.1".to_string(),
            video_port: 9000,
            ice_ufrag: "openremote-ufrag".to_string(),
            ice_pwd: "openremote-ice-password-secure".to_string(),
            dtls_fingerprint_sha256: "AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99".to_string(),
            ssrc: 0x12345678,
        }
    }
}

/// Generate a compliant WebRTC SDP Offer for H.264 screen streaming.
pub fn generate_h264_sdp_offer(params: &SdpSessionParams) -> String {
    format!(
        "v=0\r\n\
         o=- {session_id} 2 IN IP4 {host_ip}\r\n\
         s=-\r\n\
         t=0 0\r\n\
         a=group:BUNDLE video\r\n\
         a=msid-semantic: WMS openremote-screen\r\n\
         m=video {video_port} UDP/TLS/RTP/SAVPF 96\r\n\
         c=IN IP4 {host_ip}\r\n\
         a=rtpmap:96 H264/90000\r\n\
         a=fmtp:96 packetization-mode=1;profile-level-id=42001f;level-asymmetry-allowed=1\r\n\
         a=rtcp-fb:96 nack\r\n\
         a=rtcp-fb:96 nack pli\r\n\
         a=rtcp-fb:96 ccm fir\r\n\
         a=sendonly\r\n\
         a=mid:video\r\n\
         a=ice-ufrag:{ice_ufrag}\r\n\
         a=ice-pwd:{ice_pwd}\r\n\
         a=fingerprint:sha-256 {dtls_fingerprint}\r\n\
         a=setup:actpass\r\n\
         a=ssrc:{ssrc} cname:openremote-screen\r\n\
         a=ssrc:{ssrc} msid:openremote-screen video-track-1\r\n",
        session_id = params.session_id,
        host_ip = params.host_ip,
        video_port = params.video_port,
        ice_ufrag = params.ice_ufrag,
        ice_pwd = params.ice_pwd,
        dtls_fingerprint = params.dtls_fingerprint_sha256,
        ssrc = params.ssrc,
    )
}

/// Generate a matching WebRTC SDP Answer from the receiving peer (Android).
pub fn generate_h264_sdp_answer(params: &SdpSessionParams) -> String {
    format!(
        "v=0\r\n\
         o=- {session_id} 2 IN IP4 {host_ip}\r\n\
         s=-\r\n\
         t=0 0\r\n\
         a=group:BUNDLE video\r\n\
         m=video {video_port} UDP/TLS/RTP/SAVPF 96\r\n\
         c=IN IP4 {host_ip}\r\n\
         a=rtpmap:96 H264/90000\r\n\
         a=fmtp:96 packetization-mode=1;profile-level-id=42001f;level-asymmetry-allowed=1\r\n\
         a=rtcp-fb:96 nack\r\n\
         a=rtcp-fb:96 nack pli\r\n\
         a=recvonly\r\n\
         a=mid:video\r\n\
         a=ice-ufrag:{ice_ufrag}\r\n\
         a=ice-pwd:{ice_pwd}\r\n\
         a=fingerprint:sha-256 {dtls_fingerprint}\r\n\
         a=setup:active\r\n",
        session_id = params.session_id,
        host_ip = params.host_ip,
        video_port = params.video_port,
        ice_ufrag = params.ice_ufrag,
        ice_pwd = params.ice_pwd,
        dtls_fingerprint = params.dtls_fingerprint_sha256,
    )
}

/// Verify that an SDP message negotiates H.264 video.
pub fn verify_h264_sdp(sdp: &str) -> Result<(), TransportError> {
    if !sdp.contains("m=video") {
        return Err(TransportError::ReceiveFailed(
            "SDP does not contain a video media line".into(),
        ));
    }
    if !sdp.to_lowercase().contains("h264") {
        return Err(TransportError::ReceiveFailed(
            "SDP does not negotiate H264 codec".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdp_offer_and_answer_generation() {
        let params = SdpSessionParams::default();
        let offer = generate_h264_sdp_offer(&params);
        let answer = generate_h264_sdp_answer(&params);

        assert!(verify_h264_sdp(&offer).is_ok());
        assert!(verify_h264_sdp(&answer).is_ok());

        assert!(offer.contains("a=sendonly"));
        assert!(answer.contains("a=recvonly"));
        assert!(offer.contains("a=rtpmap:96 H264/90000"));
    }
}

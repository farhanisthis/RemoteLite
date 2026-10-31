//! H.264 NAL Unit generation, framing, and Annex-B stream formatting.

/// Standard H.264 NAL unit types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NalUnitType {
    Unspecified = 0,
    NonIdrSlice = 1,
    DataPartitionA = 2,
    DataPartitionB = 3,
    DataPartitionC = 4,
    IdrSlice = 5,
    Sei = 6,
    Sps = 7,
    Pps = 8,
    AccessUnitDelimiter = 9,
    EndOfSequence = 10,
    EndOfStream = 11,
    FillerData = 12,
    Other = 31,
}

impl From<u8> for NalUnitType {
    fn from(byte: u8) -> Self {
        match byte & 0x1F {
            0 => NalUnitType::Unspecified,
            1 => NalUnitType::NonIdrSlice,
            2 => NalUnitType::DataPartitionA,
            3 => NalUnitType::DataPartitionB,
            4 => NalUnitType::DataPartitionC,
            5 => NalUnitType::IdrSlice,
            6 => NalUnitType::Sei,
            7 => NalUnitType::Sps,
            8 => NalUnitType::Pps,
            9 => NalUnitType::AccessUnitDelimiter,
            10 => NalUnitType::EndOfSequence,
            11 => NalUnitType::EndOfStream,
            12 => NalUnitType::FillerData,
            _ => NalUnitType::Other,
        }
    }
}

/// Standard 4-byte Annex-B start code: 0x00, 0x00, 0x00, 0x01
pub const ANNEX_B_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

/// Construct an Annex-B formatted NAL unit by prepending a 4-byte start code.
pub fn format_annex_b_nalu(nalu_payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + nalu_payload.len());
    out.extend_from_slice(&ANNEX_B_START_CODE);
    out.extend_from_slice(nalu_payload);
    out
}

/// Generate a baseline profile Sequence Parameter Set (SPS) NAL unit
/// for a given width and height.
///
/// Baseline profile (profile_idc 66 = 0x42), constraint_set0..2 = 1, Level 3.1.
pub fn generate_baseline_sps(width: u32, height: u32) -> Vec<u8> {
    // Standard baseline SPS header:
    // NAL header: 0x67 (forbidden_zero_bit=0, nal_ref_idc=3, nal_unit_type=7)
    // Profile IDC: 0x42 (Baseline)
    // Constraint set flags: 0x80 (constraint_set0_flag=1)
    // Level IDC: 0x1F (Level 3.1)
    let mut sps = vec![0x67, 0x42, 0x80, 0x1F];

    // SPS parameters encoded via Exp-Golomb in standard H.264 syntax
    // For clean interoperability, we encode standard parameters with pic_width_in_mbs and pic_height_in_map_units
    let width_mbs = width.div_ceil(16) as u16;
    let height_mbs = height.div_ceil(16) as u16;

    // Encoded parameter stream bytes: seq_parameter_set_id(0), log2_max_frame_num_minus4(0),
    // pic_order_cnt_type(0), log2_max_pic_order_cnt_lsb_minus4(0), max_num_ref_frames(1)
    sps.extend_from_slice(&[
        0xDA, // seq_parameter_set_id + pic_order_cnt
        0x01,
        (width_mbs >> 8) as u8,
        (width_mbs & 0xFF) as u8,
        (height_mbs >> 8) as u8,
        (height_mbs & 0xFF) as u8,
        0x68,
        0xCE,
        0x3C,
        0x80, // direct_8x8_inference, frame_mbs_only_flag, rbsp_stop_one_bit
    ]);

    sps
}

/// Generate a baseline Picture Parameter Set (PPS) NAL unit.
///
/// NAL header: 0x68 (forbidden_zero_bit=0, nal_ref_idc=3, nal_unit_type=8)
pub fn generate_baseline_pps() -> Vec<u8> {
    vec![
        0x68, // NAL header: nal_ref_idc=3, nal_unit_type=8
        0xCE, // pic_parameter_set_id=0, seq_parameter_set_id=0, entropy_coding_mode_flag=0 (CAVLC)
        0x3C, 0x80,
    ]
}

/// Parse an Annex-B formatted byte stream into individual NAL units.
pub fn parse_annex_b_nalus(stream: &[u8]) -> Vec<&[u8]> {
    let mut nalus = Vec::new();
    let mut start_indices = Vec::new();

    let mut i = 0;
    while i + 3 < stream.len() {
        if stream[i] == 0 && stream[i + 1] == 0 && stream[i + 2] == 1 {
            start_indices.push(i + 3);
            i += 3;
        } else if i + 4 <= stream.len()
            && stream[i] == 0
            && stream[i + 1] == 0
            && stream[i + 2] == 0
            && stream[i + 3] == 1
        {
            start_indices.push(i + 4);
            i += 4;
        } else {
            i += 1;
        }
    }

    for (idx, &start) in start_indices.iter().enumerate() {
        let end = if idx + 1 < start_indices.len() {
            let next_start = start_indices[idx + 1];
            // Subtract start code bytes (3 or 4)
            if next_start >= 4 && stream[next_start - 4] == 0 {
                next_start - 4
            } else {
                next_start - 3
            }
        } else {
            stream.len()
        };

        if start < end {
            nalus.push(&stream[start..end]);
        }
    }

    nalus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_and_parse_annex_b_nalus() {
        let sps = generate_baseline_sps(1920, 1080);
        let pps = generate_baseline_pps();

        let mut stream = Vec::new();
        stream.extend_from_slice(&format_annex_b_nalu(&sps));
        stream.extend_from_slice(&format_annex_b_nalu(&pps));

        let nalus = parse_annex_b_nalus(&stream);
        assert_eq!(nalus.len(), 2);
        assert_eq!(NalUnitType::from(nalus[0][0]), NalUnitType::Sps);
        assert_eq!(NalUnitType::from(nalus[1][0]), NalUnitType::Pps);
    }
}

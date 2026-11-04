//! H.264 Annex-B byte streams (start-code separated, as scrcpy sends them) and the
//! length-prefixed AVCC form MP4 samples use.

const NAL_SPS: u8 = 7;
const NAL_PPS: u8 = 8;

/// The NAL units of an Annex-B buffer, without their start codes.
pub fn nal_units(data: &[u8]) -> Vec<&[u8]> {
    let mut units = Vec::new();
    let mut start: Option<usize> = None;
    let mut index = 0;
    while index + 3 <= data.len() {
        if data[index] == 0 && data[index + 1] == 0 && data[index + 2] == 1 {
            if let Some(begin) = start {
                units.push(trim_trailing_zeros(&data[begin..index]));
            }
            index += 3;
            start = Some(index);
        } else {
            index += 1;
        }
    }
    if let Some(begin) = start {
        units.push(&data[begin..]);
    }
    units.retain(|unit| !unit.is_empty());
    units
}

/// A 4-byte start code leaves a zero before the next unit's 3-byte code.
fn trim_trailing_zeros(unit: &[u8]) -> &[u8] {
    let end = unit
        .iter()
        .rposition(|&byte| byte != 0)
        .map_or(0, |last| last + 1);
    &unit[..end]
}

/// Each NAL unit prefixed with its 4-byte big-endian length.
pub fn to_avcc(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 16);
    for unit in nal_units(data) {
        out.extend_from_slice(&(unit.len() as u32).to_be_bytes());
        out.extend_from_slice(unit);
    }
    out
}

/// The first SPS and PPS in a codec-config packet.
pub fn parameter_sets(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let units = nal_units(data);
    let find = |kind: u8| units.iter().copied().find(|unit| unit[0] & 0x1f == kind);
    Some((find(NAL_SPS)?, find(NAL_PPS)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annexb_to_avcc_prefixes_lengths() {
        let annexb = [0, 0, 0, 1, 0x65, 0xAA, 0, 0, 1, 0x41, 0xBB];
        assert_eq!(
            to_avcc(&annexb),
            vec![0, 0, 0, 2, 0x65, 0xAA, 0, 0, 0, 2, 0x41, 0xBB]
        );
    }

    #[test]
    fn finds_parameter_sets() {
        let config = [0, 0, 0, 1, 0x67, 1, 2, 0, 0, 0, 1, 0x68, 3];
        let (sps, pps) = parameter_sets(&config).unwrap();
        assert_eq!(sps, &[0x67, 1, 2]);
        assert_eq!(pps, &[0x68, 3]);
        assert!(parameter_sets(&[0, 0, 1, 0x65, 1]).is_none());
    }
}

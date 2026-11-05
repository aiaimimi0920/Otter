use super::*;
#[test]
fn network_colorimetry_is_strict_and_late_parameter_sets_are_rejected() {
    for (full_range, transfer, accepted) in [
        (false, 13, true),
        (false, 2, true),
        (false, 0, false),
        (true, 13, false),
    ] {
        let mut w = Writer::default();
        w.number(66, 8);
        w.number(0, 8);
        w.number(30, 8);
        w.ue(0);
        w.ue(0);
        w.ue(2);
        w.ue(1);
        w.bit(false);
        w.ue(1);
        w.ue(1);
        w.bit(true);
        w.bit(true);
        w.bit(false);
        w.bit(true);
        w.bit(false);
        w.bit(false);
        w.bit(true);
        w.number(5, 3);
        w.bit(full_range);
        w.bit(true);
        w.number(1, 8);
        w.number(transfer, 8);
        w.number(1, 8);
        for _ in 0..6 {
            w.bit(false);
        }
        let mut bytes = vec![0, 0, 0, 1, 0x67];
        bytes.extend(w.bytes());
        bytes.extend_from_slice(&keyframe(2, 1, 66)[sps(2, 1, 66).len()..]);
        assert_eq!(
            validate(&bytes, 32, 32, true).is_ok(),
            accepted,
            "full={full_range} transfer={transfer}"
        );
    }
    let mut late = vec![0, 0, 1, 0x41, 1];
    late.extend(sps(2, 1, 66));
    assert!(validate(&late, 32, 32, false).is_err());
}
#[derive(Default)]
struct Writer(Vec<bool>);
impl Writer {
    fn bit(&mut self, value: bool) {
        self.0.push(value);
    }
    fn number(&mut self, value: u32, count: usize) {
        for shift in (0..count).rev() {
            self.bit(value & (1 << shift) != 0);
        }
    }
    fn ue(&mut self, value: u32) {
        let code = value + 1;
        let size = (32 - code.leading_zeros()) as usize;
        for _ in 1..size {
            self.bit(false);
        }
        self.number(code, size);
    }
    fn bytes(mut self) -> Vec<u8> {
        self.bit(true);
        while !self.0.len().is_multiple_of(8) {
            self.bit(false);
        }
        self.0
            .chunks(8)
            .map(|chunk| {
                chunk
                    .iter()
                    .fold(0, |value, bit| (value << 1) | u8::from(*bit))
            })
            .collect()
    }
}
fn sps(width_mbs: u32, refs: u32, profile: u32) -> Vec<u8> {
    let mut w = Writer::default();
    w.number(profile, 8);
    w.number(0, 8);
    w.number(30, 8);
    w.ue(0);
    w.ue(0);
    w.ue(2);
    w.ue(refs);
    w.bit(false);
    w.ue(width_mbs - 1);
    w.ue(1);
    w.bit(true);
    w.bit(true);
    w.bit(false);
    w.bit(false);
    let mut bytes = vec![0, 0, 0, 1, 0x67];
    bytes.extend(w.bytes());
    bytes
}
fn keyframe(width_mbs: u32, refs: u32, profile: u32) -> Vec<u8> {
    let mut bytes = sps(width_mbs, refs, profile);
    let mut pps = Writer::default();
    pps.ue(0);
    pps.ue(0);
    pps.bit(false);
    pps.bit(false);
    pps.ue(0);
    pps.ue(0);
    pps.ue(0);
    bytes.extend([0, 0, 1, 0x68]);
    bytes.extend(pps.bytes());
    bytes.extend([0, 0, 1, 0x65, 1]);
    bytes
}
#[test]
fn baseline_keyframe_and_delta_follow_main_contract() {
    let bytes = keyframe(2, 1, 66);
    assert_eq!(validate(&bytes, 32, 32, true), Ok(()));
    assert!(validate(&bytes, 32, 32, false).is_err());
    assert_eq!(validate(&[0, 0, 1, 0x41, 1], 32, 32, false), Ok(()));
    assert!(validate(&[0, 0, 1, 0x41, 1], 32, 32, true).is_err());
}
#[test]
fn actual_sps_overrides_untrusted_wire_claims() {
    assert!(validate(&keyframe(2, 1, 66), 16, 32, true).is_err());
    assert!(validate(&keyframe(257, 1, 66), 32, 32, true).is_err());
    assert!(validate(&keyframe(2, 5, 66), 32, 32, true).is_err());
    assert!(validate(&keyframe(2, 1, 100), 32, 32, true).is_err());
}
#[test]
fn bounded_framing_and_parameter_reader_reject_hostile_inputs() {
    for bytes in [
        &[][..],
        &[0, 0, 1],
        &[0, 0, 1, 0xe5],
        &[0, 0, 1, 0x65, 1],
        &[0, 0, 1, 0x67],
    ] {
        assert!(validate(bytes, 32, 32, true).is_err());
    }
    assert!(validate(&vec![0; 1024 * 1024 + 1], 32, 32, true).is_err());
    assert!(validate(&[0, 0, 1, 9].repeat(257), 32, 32, false).is_err());
    assert!(bits::Bits::new(&[0; 1025]).is_err());
    assert!(bits::Bits::new(&[0, 0, 3, 4]).is_err());
    assert!(bits::Bits::new(&[0, 0, 3]).is_err());
    assert!(bits::Bits::new(&[0; 8]).unwrap().ue().is_err());
    let mut bits = bits::Bits::new(&[0, 0, 3, 1]).unwrap();
    assert_eq!(bits.read(24), Ok(1));
    assert!(bits.read(1).is_err());
}

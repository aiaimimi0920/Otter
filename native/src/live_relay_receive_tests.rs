use super::*;
use crate::live_relay_frame::ColorSpace;
fn header(codec: Codec) -> [u8; 64] {
    Metadata {
        epoch: 1,
        frame_id: 2,
        capture_timestamp_ms: 3,
        encode_timestamp_ms: 4,
        width: 2,
        height: 2,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec,
    }
    .encode_header(16)
    .unwrap()
}
#[test]
fn every_fragment_boundary_preserves_payload_and_budget_until_frame_drop() {
    let budget = Budget::new(16);
    let payload: Vec<u8> = (0..16).collect();
    for split in 0..=16 {
        let mut pending =
            PendingFrame::begin(&header(Codec::RawBgra), Profile::Legacy, &budget).unwrap();
        assert_eq!(budget.used(), 16);
        pending.push(&payload[..split]).unwrap();
        pending.push(&payload[split..]).unwrap();
        assert_eq!(pending.remaining(), 0);
        let frame = pending.finish().unwrap();
        assert_eq!(frame.metadata().frame_id, 2);
        assert_eq!(frame.payload(), payload);
        assert_eq!(budget.used(), 16);
        drop(frame);
        assert_eq!(budget.used(), 0);
    }
}
#[test]
fn cancellation_truncation_and_oversize_never_publish_or_leak_budget() {
    let budget = Budget::new(16);
    for length in 0..16 {
        let mut pending =
            PendingFrame::begin(&header(Codec::RawBgra), Profile::Legacy, &budget).unwrap();
        pending.push(&vec![0; length]).unwrap();
        assert!(matches!(pending.finish(), Err(Error::Length)));
        assert_eq!(budget.used(), 0);
    }
    let mut pending =
        PendingFrame::begin(&header(Codec::RawBgra), Profile::Legacy, &budget).unwrap();
    assert_eq!(pending.push(&[0; 17]), Err(Error::Length));
    assert_eq!(pending.push(&[0; 16]), Err(Error::Poisoned));
    assert!(matches!(pending.finish(), Err(Error::Poisoned)));
    assert_eq!(budget.used(), 0);
    let pending = PendingFrame::begin(&header(Codec::RawBgra), Profile::Legacy, &budget).unwrap();
    drop(pending);
    assert_eq!(budget.used(), 0);
}
#[test]
fn negotiation_gates_codec_before_any_body_allocation() {
    let budget = Budget::new(16);
    for (protocol, profile) in [
        ("loom.live.v1", Profile::Legacy),
        ("loom.live.jpeg.v1", Profile::Jpeg),
        ("loom.live.h264.v1", Profile::H264),
    ] {
        assert_eq!(Profile::negotiated(Some(protocol)), Ok(profile));
        for codec in [Codec::RawBgra, Codec::Jpeg, Codec::H264] {
            let result = PendingFrame::begin(&header(codec), profile, &budget);
            let expected = codec == Codec::RawBgra
                || profile == Profile::H264
                || (codec == Codec::Jpeg && profile == Profile::Jpeg);
            assert_eq!(result.is_ok(), expected);
            drop(result);
            assert_eq!(budget.used(), 0);
        }
    }
    for protocol in [None, Some(""), Some("loom.live.v2"), Some(" loom.live.v1")] {
        assert_eq!(Profile::negotiated(protocol), Err(Error::Profile));
    }
}
#[test]
fn shared_budget_blocks_second_owner_and_invalid_header_has_no_claim() {
    let budget = Budget::new(16);
    let first = PendingFrame::begin(&header(Codec::RawBgra), Profile::Legacy, &budget).unwrap();
    assert!(matches!(
        PendingFrame::begin(&header(Codec::RawBgra), Profile::Legacy, &budget),
        Err(Error::Budget)
    ));
    assert_eq!(budget.used(), 16);
    drop(first);
    assert_eq!(budget.used(), 0);
    assert!(matches!(
        PendingFrame::begin(&[0; 64], Profile::Legacy, &budget),
        Err(Error::Wire(_))
    ));
    assert_eq!(budget.used(), 0);
}

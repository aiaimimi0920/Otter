use super::*;
use crate::{
    byte_budget::Budget,
    live_relay_frame::ColorSpace,
    live_relay_receive::{PendingFrame, Profile},
};
fn frame(epoch: u64, id: u64, width: u32, keyframe: bool, codec: Codec) -> Frame {
    let metadata = Metadata {
        epoch,
        frame_id: id,
        capture_timestamp_ms: 1,
        encode_timestamp_ms: 1,
        width,
        height: 2,
        dropped_frames: 0,
        keyframe,
        color_space: ColorSpace::Srgb,
        codec,
    };
    let size = if codec == Codec::RawBgra {
        width as usize * 8
    } else {
        4
    };
    let mut pending = PendingFrame::begin(
        &metadata.encode_header(size).unwrap(),
        Profile::H264,
        &Budget::new(size),
    )
    .unwrap();
    pending.push(&vec![0; size]).unwrap();
    pending.finish().unwrap()
}
#[test]
fn missing_keyframe_requests_once_and_first_idr_starts_chain() {
    let mut chain = Chain::default();
    assert_eq!(
        chain.accept(&frame(1, 1, 2, false, Codec::H264)),
        Ok(Action::RequestKeyframe { epoch: 1 })
    );
    assert_eq!(
        chain.accept(&frame(1, 2, 2, false, Codec::H264)),
        Ok(Action::WaitForKeyframe)
    );
    assert_eq!(
        chain.accept(&frame(1, 3, 2, true, Codec::H264)),
        Ok(Action::Decode {
            reset_decoder: true,
            timestamp: 3
        })
    );
    assert_eq!(chain.decoded(3).unwrap().frame_id, 3);
    assert_eq!(
        chain.accept(&frame(1, 4, 2, false, Codec::H264)),
        Ok(Action::Decode {
            reset_decoder: false,
            timestamp: 4
        })
    );
}
#[test]
fn gaps_epoch_and_size_changes_clear_reference_chain() {
    for (epoch, id, width) in [(1, 3, 2), (2, 1, 2), (1, 2, 4)] {
        let mut chain = Chain::default();
        chain.accept(&frame(1, 1, 2, true, Codec::H264)).unwrap();
        assert_eq!(
            chain.accept(&frame(epoch, id, width, false, Codec::H264)),
            Ok(Action::RequestKeyframe { epoch })
        );
        assert!(chain.pending.is_empty());
        assert_eq!(
            chain.accept(&frame(epoch, id + 1, width, true, Codec::H264)),
            Ok(Action::Decode {
                reset_decoder: true,
                timestamp: (id + 1) as i64
            })
        );
    }
}
#[test]
fn pending_limit_and_output_mismatch_fail_closed() {
    let mut chain = Chain::default();
    for id in 1..=4 {
        chain
            .accept(&frame(1, id, 2, id == 1, Codec::H264))
            .unwrap();
    }
    assert_eq!(
        chain.accept(&frame(1, 5, 2, false, Codec::H264)),
        Err(Error::PendingLimit)
    );
    assert!(chain.pending.is_empty());
    assert!(chain.fail());
    assert!(!chain.fail());
    assert_eq!(
        chain.accept(&frame(1, 6, 2, true, Codec::H264)),
        Ok(Action::IgnoreFallback)
    );
    let mut chain = Chain::default();
    chain.accept(&frame(1, 1, 2, true, Codec::H264)).unwrap();
    assert_eq!(chain.decoded(2), Err(Error::UnexpectedOutput));
    assert!(chain.pending.is_empty());
}
#[test]
fn sticky_fallback_survives_images_and_timeline_cannot_wrap() {
    let mut chain = Chain::default();
    assert!(chain.fail());
    assert!(!chain.fail());
    assert_eq!(
        chain.accept(&frame(1, 1, 2, true, Codec::Jpeg)),
        Ok(Action::Image)
    );
    assert_eq!(
        chain.accept(&frame(1, 2, 2, true, Codec::H264)),
        Ok(Action::IgnoreFallback)
    );
    let mut chain = Chain::default();
    assert_eq!(
        chain.accept(&frame(1, u64::MAX, 2, true, Codec::H264)),
        Err(Error::Timeline)
    );
    assert!(chain.pending.is_empty());
}
#[test]
fn waiting_keyframe_request_is_scoped_to_epoch() {
    let mut chain = Chain::default();
    assert_eq!(
        chain.accept(&frame(1, 1, 2, false, Codec::H264)),
        Ok(Action::RequestKeyframe { epoch: 1 })
    );
    assert_eq!(
        chain.accept(&frame(2, 1, 2, false, Codec::H264)),
        Ok(Action::RequestKeyframe { epoch: 2 })
    );
    assert_eq!(
        chain.accept(&frame(2, 2, 2, false, Codec::H264)),
        Ok(Action::WaitForKeyframe)
    );
}

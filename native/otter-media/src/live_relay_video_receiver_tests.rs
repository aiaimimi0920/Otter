use super::*;
use crate::{
    byte_budget::Budget,
    live_relay_frame::{ColorSpace, Metadata},
    live_relay_receive::{PendingFrame, Profile},
};
fn frame(epoch: u64, id: u64, keyframe: bool, codec: Codec, bytes: &[u8]) -> Frame {
    let metadata = Metadata {
        epoch,
        frame_id: id,
        capture_timestamp_ms: 17,
        encode_timestamp_ms: 19,
        width: 64,
        height: 64,
        dropped_frames: 0,
        keyframe,
        color_space: ColorSpace::Srgb,
        codec,
    };
    let mut pending = PendingFrame::begin(
        &metadata.encode_header(bytes.len()).unwrap(),
        Profile::H264,
        &Budget::new(bytes.len()),
    )
    .unwrap();
    pending.push(bytes).unwrap();
    pending.finish().unwrap()
}
#[test]
fn epoch_and_close_are_checked_before_codec_or_keyframe_request() {
    let mut receiver = Receiver::default();
    let stop = AtomicBool::new(false);
    for epoch in [0, 1, 3] {
        assert!(matches!(
            receiver.receive(frame(2, 1, false, Codec::H264, &[1]), epoch, &stop),
            Err(Error::Epoch)
        ));
    }
    assert!(matches!(
        receiver.receive(frame(2, 1, false, Codec::H264, &[1]), 2, &stop),
        Ok(Outcome::RequestKeyframe { epoch: 2 })
    ));
    assert!(matches!(
        receiver.receive(frame(2, 2, false, Codec::H264, &[1]), 2, &stop),
        Ok(Outcome::Pending)
    ));
    receiver.close();
    assert!(matches!(
        receiver.receive(frame(2, 3, true, Codec::H264, &[1]), 2, &stop),
        Err(Error::Closed)
    ));
}
#[test]
fn image_passes_unique_frame_and_cancellation_is_terminal() {
    let mut receiver = Receiver::default();
    let raw = vec![255; 64 * 64 * 4];
    let Outcome::Image(image) = receiver
        .receive(
            frame(1, 1, true, Codec::RawBgra, &raw),
            1,
            &AtomicBool::new(false),
        )
        .unwrap()
    else {
        panic!("image path");
    };
    assert_eq!(image.payload(), raw);
    assert!(matches!(
        receiver.receive(
            frame(1, 2, true, Codec::RawBgra, &raw),
            1,
            &AtomicBool::new(true)
        ),
        Err(Error::Closed)
    ));
    assert!(matches!(
        receiver.receive(
            frame(1, 3, true, Codec::RawBgra, &raw),
            1,
            &AtomicBool::new(false)
        ),
        Err(Error::Closed)
    ));
}
#[test]
#[ignore = "requires OTTER_NATIVE_H264_FIXTURES and system decoder; no network or UI"]
fn actual_receiver_recovers_gap_and_releases_codec_on_image_fallback_and_close() {
    let root = std::env::var("OTTER_NATIVE_H264_FIXTURES").unwrap();
    let bytes = std::fs::read(std::path::Path::new(&root).join("red64.h264")).unwrap();
    let stop = AtomicBool::new(false);
    let mut receiver = Receiver::default();
    for id in [1, 4] {
        let Outcome::Decoded(output) = receiver
            .receive(frame(1, id, true, Codec::H264, &bytes), 1, &stop)
            .unwrap()
        else {
            panic!("expected IDR output");
        };
        let metadata = output.metadata();
        assert_eq!(
            (
                metadata.frame_id,
                metadata.codec,
                metadata.capture_timestamp_ms,
                metadata.encode_timestamp_ms
            ),
            (id, Codec::RawBgra, 17, 19)
        );
        assert_eq!(output.pixels().len(), 64 * 64 * 4);
        let storage = output.pixels().clone();
        let cache = crate::live_relay_viewer_frames::ViewerPixels::new(1).unwrap();
        cache.commit(output, false).unwrap();
        let presented = cache
            .take(crate::live_relay_viewer_frames::Cursor {
                epoch: 1,
                frame_id: id,
            })
            .unwrap()
            .unwrap();
        assert!(presented.pixels().same_storage(&storage));
        assert!(DecoderOwner::create().is_err());
        if id == 1 {
            assert!(matches!(
                receiver.receive(frame(1, 3, false, Codec::H264, &[1]), 1, &stop),
                Ok(Outcome::RequestKeyframe { epoch: 1 })
            ));
            drop(DecoderOwner::create().unwrap());
        }
    }
    let raw = vec![255; 64 * 64 * 4];
    assert!(matches!(
        receiver.receive(frame(1, 5, true, Codec::RawBgra, &raw), 1, &stop),
        Ok(Outcome::Image(_))
    ));
    drop(DecoderOwner::create().unwrap());
    assert!(matches!(
        receiver.receive(
            frame(1, 6, true, Codec::H264, &[0, 0, 1, 0x65, 0]),
            1,
            &stop
        ),
        Ok(Outcome::Fallback { epoch: 1, .. })
    ));
    drop(DecoderOwner::create().unwrap());
    assert!(matches!(
        receiver.receive(frame(1, 7, true, Codec::H264, &bytes), 1, &stop),
        Ok(Outcome::Pending)
    ));
    assert!(matches!(
        receiver.receive(frame(1, 8, true, Codec::RawBgra, &raw), 1, &stop),
        Ok(Outcome::Image(_))
    ));
    assert!(matches!(
        receiver.receive(frame(1, 9, true, Codec::H264, &bytes), 1, &stop),
        Ok(Outcome::Pending)
    ));
    let mut fresh = Receiver::default();
    assert!(matches!(
        fresh.receive(frame(2, 1, true, Codec::H264, &bytes), 2, &stop),
        Ok(Outcome::Decoded(_))
    ));
    fresh.close();
    drop(DecoderOwner::create().unwrap());
}

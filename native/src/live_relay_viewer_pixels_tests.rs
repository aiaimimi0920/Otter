use super::*;
use crate::{
    live_relay_frame::{Codec, ColorSpace},
    live_relay_pixels::PixelFrame,
    pixels::Pixels,
};
fn metadata(id: u64) -> Metadata {
    Metadata {
        epoch: 1,
        frame_id: id,
        capture_timestamp_ms: 1,
        encode_timestamp_ms: 2,
        width: 1,
        height: 1,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::RawBgra,
    }
}
#[test]
fn decoded_frame_rejects_compressed_hdr_and_wrong_length() {
    for change in 0..3 {
        let mut m = metadata(1);
        match change {
            0 => m.codec = Codec::H264,
            1 => m.color_space = ColorSpace::Hdr10,
            _ => m.width = 2,
        }
        assert!(PixelFrame::new(m, Pixels::from(vec![255; 4])).is_err());
    }
}
#[test]
fn decoded_cache_reuses_storage_and_releases_evicted_and_closed_frames() {
    let frames = ViewerPixels::new(1).unwrap();
    let mut weak = Vec::new();
    for id in 1..=4 {
        let pixels = Pixels::from(vec![255; 4]);
        weak.push(pixels.downgrade());
        frames
            .commit(PixelFrame::new(metadata(id), pixels).unwrap(), false)
            .unwrap();
    }
    assert!(weak[0].upgrade().is_none());
    assert_eq!(frames.status().unwrap().overwritten, 1);
    let output = frames
        .take(Cursor {
            epoch: 1,
            frame_id: 3,
        })
        .unwrap()
        .unwrap();
    assert!(weak[1].upgrade().is_none());
    assert!(std::sync::Arc::ptr_eq(
        &weak[2].upgrade().unwrap(),
        &output.pixels().downgrade().upgrade().unwrap()
    ));
    frames.close();
    assert!(weak[3].upgrade().is_none());
    assert!(weak[2].upgrade().is_some());
    drop(output);
    assert!(weak[2].upgrade().is_none());
    let pixels = Pixels::from(vec![255; 4]);
    let late = pixels.downgrade();
    assert_eq!(
        frames.commit(PixelFrame::new(metadata(5), pixels).unwrap(), false),
        Err(Error::Closed)
    );
    assert!(late.upgrade().is_none());
}

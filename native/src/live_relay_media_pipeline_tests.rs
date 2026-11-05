use super::*;
use crate::{
    byte_budget::Budget,
    live_relay_frame::{Codec, ColorSpace, Metadata},
    live_relay_receive::{PendingFrame, Profile},
    live_relay_viewer_frames::Cursor,
    live_relay_viewer_identity::Snapshot,
};
fn session() -> (Arc<ViewerSession>, Snapshot) {
    let bytes=serde_json::to_vec(&serde_json::json!({"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source:test","sourceHookId":"hook:test","revision":1,"viewerDevices":["viewer:test"]},"epoch":1,"closed":false})).unwrap();
    let snapshot = Snapshot::parse(&bytes, "live:test").unwrap();
    let session =
        Arc::new(ViewerSession::new(snapshot.identity().clone(), "viewer:test".into()).unwrap());
    session.activate(&snapshot).unwrap();
    (session, snapshot)
}
fn frame(codec: Codec, bytes: &[u8]) -> Frame {
    let m = Metadata {
        epoch: 1,
        frame_id: 1,
        capture_timestamp_ms: 4,
        encode_timestamp_ms: 5,
        width: 64,
        height: 64,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec,
    };
    let mut pending = PendingFrame::begin(
        &m.encode_header(bytes.len()).unwrap(),
        Profile::H264,
        &Budget::new(bytes.len()),
    )
    .unwrap();
    pending.push(bytes).unwrap();
    pending.finish().unwrap()
}
#[test]
fn raw_pipeline_publishes_then_old_generation_is_permanently_retired() {
    let (session, snapshot) = session();
    let mut pipeline = Pipeline::new(session.clone(), session.admit(1).unwrap()).unwrap();
    assert_eq!(
        pipeline.receive(frame(Codec::RawBgra, &vec![255; 64 * 64 * 4])),
        Ok(Progress::Published(Commit::Accepted))
    );
    let output = session
        .take(Cursor {
            epoch: 1,
            frame_id: 1,
        })
        .unwrap()
        .unwrap();
    assert_eq!(output.metadata().capture_timestamp_ms, 4);
    assert!(output.pixels().iter().all(|b| *b == 255));
    session.suspend().unwrap();
    assert!(matches!(
        pipeline.check(),
        Err(Error::Session(session::Error::Suspended))
    ));
    session.activate(&snapshot).unwrap();
    assert_eq!(pipeline.check(), Err(Error::Retired));
    assert_eq!(
        pipeline.receive(frame(Codec::RawBgra, &vec![255; 64 * 64 * 4])),
        Err(Error::Retired)
    );
}
#[test]
fn foreign_permit_is_rejected_before_creating_a_codec_pipeline() {
    let (first, _) = session();
    let (second, _) = session();
    assert!(matches!(
        Pipeline::new(second, first.admit(1).unwrap()),
        Err(Error::Session(session::Error::StalePermit))
    ));
}
#[test]
#[ignore = "requires OTTER_NATIVE_H264_FIXTURES and system H264 decoder"]
fn actual_h264_pipeline_publishes_and_suspend_releases_decoder_before_next_frame() {
    let root = std::env::var("OTTER_NATIVE_H264_FIXTURES").unwrap();
    let bytes = std::fs::read(std::path::Path::new(&root).join("red64.h264")).unwrap();
    let (session, snapshot) = session();
    let mut pipeline = Pipeline::new(session.clone(), session.admit(1).unwrap()).unwrap();
    assert_eq!(
        pipeline.receive(frame(Codec::H264, &bytes)),
        Ok(Progress::Published(Commit::Accepted))
    );
    assert!(crate::live_mf::DecoderOwner::create().is_err());
    let output = session
        .take(Cursor {
            epoch: 1,
            frame_id: 1,
        })
        .unwrap()
        .unwrap();
    assert!(output
        .pixels()
        .chunks_exact(4)
        .all(|p| p[2] > 200 && p[0] < 40 && p[1] < 40 && p[3] == 255));
    session.suspend().unwrap();
    assert!(pipeline.check().is_err());
    drop(crate::live_mf::DecoderOwner::create().unwrap());
    session.activate(&snapshot).unwrap();
    let mut fresh = Pipeline::new(session.clone(), session.admit(1).unwrap()).unwrap();
    assert_eq!(
        fresh.receive(frame(Codec::H264, &bytes)),
        Ok(Progress::Published(Commit::Accepted))
    );
    session.close();
    assert!(fresh.check().is_err());
    drop(crate::live_mf::DecoderOwner::create().unwrap());
}

use super::*;
use crate::{
    byte_budget::Budget,
    live_relay_frame::{Codec, ColorSpace, Metadata},
    live_relay_receive::{PendingFrame, Profile},
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
fn frame() -> Frame {
    frame_with_id(1)
}
fn frame_with_id(id: u64) -> Frame {
    let m = Metadata {
        epoch: 1,
        frame_id: id,
        capture_timestamp_ms: 1,
        encode_timestamp_ms: 1,
        width: 1,
        height: 1,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::RawBgra,
    };
    let mut pending = PendingFrame::begin(
        &m.encode_header(4).unwrap(),
        Profile::Legacy,
        &Budget::new(4),
    )
    .unwrap();
    pending.push(&[255; 4]).unwrap();
    pending.finish().unwrap()
}
#[test]
fn saturated_input_revokes_generation_instead_of_silently_dropping_video() {
    let (session, _) = session();
    let permit = session.admit(1).unwrap();
    let (sender, _queued) = mpsc::sync_channel(INPUT_CAPACITY);
    let (_reply, replies) = mpsc::sync_channel(REPLY_CAPACITY);
    let worker = Worker {
        sender: Some(sender),
        replies,
        stop: Arc::new(AtomicBool::new(false)),
        fallback: Arc::new(AtomicBool::new(false)),
        session: session.clone(),
        permit: permit.clone(),
        thread: None,
    };
    worker.submit(frame()).unwrap();
    worker.submit(frame()).unwrap();
    assert_eq!(worker.submit(frame()), Err(SubmitError::Full));
    assert!(permit.stop().load(Ordering::Acquire));
    assert!(session.validate(&permit).is_err());
}
#[test]
fn saturated_replies_terminate_the_loop_without_blocking() {
    let (session, _) = session();
    let permit = session.admit(1).unwrap();
    let (sender, commands) = mpsc::sync_channel(2);
    let (reply, _undrained) = mpsc::sync_channel(1);
    sender.send(Command::Frame(frame_with_id(1))).unwrap();
    sender.send(Command::Frame(frame_with_id(2))).unwrap();
    assert_eq!(
        run(
            &session,
            &permit,
            &AtomicBool::new(false),
            commands,
            reply,
            &AtomicBool::new(false)
        ),
        Exit::RepliesFull
    );
}
fn wait(mut predicate: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(std::time::Instant::now() < deadline, "worker deadline");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn decoder_fallback_preference_survives_subsequent_raw_progress() {
    let (session, _) = session();
    let mut worker = Worker::start(session.clone(), session.admit(1).unwrap()).unwrap();
    let metadata = Metadata {
        epoch: 1,
        frame_id: 1,
        capture_timestamp_ms: 1,
        encode_timestamp_ms: 1,
        width: 64,
        height: 64,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::H264,
    };
    let malformed_au = [0u8, 0, 1, 0x65, 1];
    let mut pending = PendingFrame::begin(
        &metadata.encode_header(malformed_au.len()).unwrap(),
        Profile::H264,
        &Budget::new(1024),
    )
    .unwrap();
    pending.push(&malformed_au).unwrap();
    worker.submit(pending.finish().unwrap()).unwrap();
    wait(|| matches!(worker.try_reply(), Some(Progress::Fallback { .. })));
    assert!(worker.prefers_legacy());
    worker.submit(frame_with_id(2)).unwrap();
    wait(|| matches!(worker.try_reply(), Some(Progress::Published(_))));
    assert!(worker.prefers_legacy());
    worker.request_stop();
    wait(|| worker.try_join().is_some());
}
#[test]
fn worker_publishes_stops_joins_and_old_stop_cannot_revoke_new_generation() {
    let (session, snapshot) = session();
    let permit = session.admit(1).unwrap();
    let mut worker = Worker::start(session.clone(), permit).unwrap();
    assert!(worker.wait_handle().is_some());
    worker.submit(frame()).unwrap();
    wait(|| {
        worker.try_reply()
            == Some(Progress::Published(
                crate::live_relay_viewer_frames::Commit::Accepted,
            ))
    });
    worker.request_stop();
    let mut exit = None;
    wait(|| {
        exit = worker.try_join();
        exit.is_some()
    });
    assert!(matches!(
        exit,
        Some(Exit::Stopped) | Some(Exit::Pipeline(_))
    ));
    assert!(worker.wait_handle().is_none());
    session.activate(&snapshot).unwrap();
    let current = session.admit(1).unwrap();
    worker.request_stop();
    worker.revoke();
    drop(worker);
    assert!(session.validate(&current).is_ok());
}
#[test]
fn idle_worker_observes_session_close_without_any_frame() {
    let (session, _) = session();
    let mut worker = Worker::start(session.clone(), session.admit(1).unwrap()).unwrap();
    session.close();
    wait(|| worker.try_join().is_some());
    assert_eq!(worker.submit(frame()), Err(SubmitError::Stopped));
}
#[test]
#[ignore = "requires OTTER_NATIVE_H264_FIXTURES and system H264 decoder"]
fn actual_worker_decodes_h264_and_reaps_codec_after_idle_close() {
    let root = std::env::var("OTTER_NATIVE_H264_FIXTURES").unwrap();
    let bytes = std::fs::read(std::path::Path::new(&root).join("red64.h264")).unwrap();
    let m = Metadata {
        epoch: 1,
        frame_id: 1,
        capture_timestamp_ms: 1,
        encode_timestamp_ms: 1,
        width: 64,
        height: 64,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::H264,
    };
    let mut pending = PendingFrame::begin(
        &m.encode_header(bytes.len()).unwrap(),
        Profile::H264,
        &Budget::new(bytes.len()),
    )
    .unwrap();
    pending.push(&bytes).unwrap();
    let (session, _) = session();
    let mut worker = Worker::start(session.clone(), session.admit(1).unwrap()).unwrap();
    worker.submit(pending.finish().unwrap()).unwrap();
    wait(|| {
        worker.try_reply()
            == Some(Progress::Published(
                crate::live_relay_viewer_frames::Commit::Accepted,
            ))
    });
    let pixels = session
        .take(crate::live_relay_viewer_frames::Cursor {
            epoch: 1,
            frame_id: 1,
        })
        .unwrap()
        .unwrap();
    assert!(pixels
        .pixels()
        .chunks_exact(4)
        .all(|p| p[2] > 200 && p[0] < 40 && p[1] < 40 && p[3] == 255));
    assert!(crate::live_mf::DecoderOwner::create().is_err());
    session.close();
    wait(|| worker.try_join().is_some());
    drop(crate::live_mf::DecoderOwner::create().unwrap());
    eprintln!("native-media-worker: h264-published=true idle-close-joined=true decoder-permit-returned=true");
}

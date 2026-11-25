use super::*;
use crate::{
    byte_budget::Budget,
    live_relay_frame::{Codec, ColorSpace},
    live_relay_receive::{PendingFrame, Profile},
};
use std::sync::{Arc, Barrier};
#[test]
fn simultaneous_close_and_commit_leave_no_buffer_or_lease() {
    for _ in 0..32 {
        let budget = Budget::new(4);
        let frames = Arc::new(ViewerFrames::new(1).unwrap());
        let candidate = frame(&budget, 1, 1);
        let barrier = Arc::new(Barrier::new(2));
        let worker = {
            let frames = frames.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                frames.commit(candidate, false)
            })
        };
        barrier.wait();
        frames.close();
        assert!(matches!(
            worker.join().unwrap(),
            Ok(Commit::Accepted) | Err(Error::Closed)
        ));
        let status = frames.status().unwrap();
        assert!(status.closed);
        assert_eq!(status.buffered, 0);
        assert_eq!(budget.used(), 0);
    }
}
fn frame(budget: &Arc<Budget>, epoch: u64, frame_id: u64) -> Frame {
    let header = Metadata {
        epoch,
        frame_id,
        capture_timestamp_ms: 1,
        encode_timestamp_ms: 1,
        width: 1,
        height: 1,
        dropped_frames: 0,
        keyframe: true,
        color_space: ColorSpace::Srgb,
        codec: Codec::RawBgra,
    }
    .encode_header(4)
    .unwrap();
    let mut pending = PendingFrame::begin(&header, Profile::Legacy, budget).unwrap();
    pending.push(&[1; 4]).unwrap();
    pending.finish().unwrap()
}
#[test]
fn stale_duplicate_and_warmup_do_not_advance_cursor_or_retain_budget() {
    let budget = Budget::new(64);
    let frames = ViewerFrames::new(2).unwrap();
    assert_eq!(
        frames.commit(frame(&budget, 2, 9), false),
        Ok(Commit::Accepted)
    );
    for (epoch, id) in [(1, 100), (2, 8), (2, 9)] {
        assert_eq!(
            frames.commit(frame(&budget, epoch, id), false),
            Err(Error::Stale)
        );
        assert_eq!(
            frames.commit(frame(&budget, epoch, id), true),
            Ok(Commit::IgnoredWarmup)
        );
        assert_eq!(budget.used(), 4);
    }
    assert_eq!(
        frames.status().unwrap().cursor,
        Cursor {
            epoch: 2,
            frame_id: 9
        }
    );
    assert_eq!(
        frames.commit(frame(&budget, 3, 1), false),
        Ok(Commit::Accepted)
    );
    assert_eq!(
        frames
            .latest_after(Cursor {
                epoch: 2,
                frame_id: 999
            })
            .unwrap()
            .unwrap()
            .epoch,
        3
    );
    drop(frames);
    assert_eq!(budget.used(), 0);
}
#[test]
fn three_slots_evict_oldest_and_take_transfers_lease_without_copy() {
    let budget = Budget::new(64);
    let frames = ViewerFrames::new(1).unwrap();
    for id in 1..=4 {
        frames.commit(frame(&budget, 1, id), false).unwrap();
    }
    assert_eq!(frames.status().unwrap().buffered, 3);
    assert_eq!(frames.status().unwrap().overwritten, 1);
    assert_eq!(budget.used(), 12);
    assert!(frames
        .take(Cursor {
            epoch: 2,
            frame_id: 3
        })
        .unwrap()
        .is_none());
    assert!(frames
        .take(Cursor {
            epoch: 1,
            frame_id: 1
        })
        .unwrap()
        .is_none());
    let output = frames
        .take(Cursor {
            epoch: 1,
            frame_id: 3,
        })
        .unwrap()
        .unwrap();
    assert_eq!(output.payload(), &[1; 4]);
    assert_eq!(frames.status().unwrap().buffered, 1);
    assert_eq!(budget.used(), 8);
    frames.close();
    assert_eq!(budget.used(), 4);
    drop(output);
    assert_eq!(budget.used(), 0);
}
#[test]
fn close_rejects_late_worker_and_is_idempotent() {
    let budget = Budget::new(64);
    let frames = Arc::new(ViewerFrames::new(1).unwrap());
    frames.commit(frame(&budget, 1, 1), false).unwrap();
    let late = frame(&budget, 1, 2);
    let barrier = Arc::new(Barrier::new(2));
    let worker = {
        let frames = frames.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            barrier.wait();
            frames.commit(late, true)
        })
    };
    frames.close();
    frames.close();
    barrier.wait();
    assert_eq!(worker.join().unwrap(), Err(Error::Closed));
    assert_eq!(budget.used(), 0);
    let status = frames.status().unwrap();
    assert!(status.closed);
    assert_eq!(status.buffered, 0);
    assert!(matches!(
        frames.take(Cursor {
            epoch: 1,
            frame_id: 1
        }),
        Err(Error::Closed)
    ));
    assert_eq!(status.cursor.frame_id, 1);
}
#[test]
fn poisoned_owner_can_still_retire_all_frames() {
    let budget = Budget::new(64);
    let frames = ViewerFrames::new(1).unwrap();
    frames.commit(frame(&budget, 1, 1), false).unwrap();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = frames.0.lock().unwrap();
        panic!("fixture poison");
    }));
    frames.close();
    assert_eq!(budget.used(), 0);
    assert_eq!(frames.status(), Err(Error::Poisoned));
    assert_eq!(
        frames.commit(frame(&budget, 1, 2), false),
        Err(Error::Poisoned)
    );
    assert_eq!(budget.used(), 0);
}

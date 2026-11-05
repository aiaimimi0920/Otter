//! 滚动后最多采样五帧，连续两次完全一致才接受；仅保留前后两帧，不缓存采样历史。
use crate::long_capture::Frame;
use std::sync::atomic::{AtomicBool, Ordering};
pub(super) fn sample(
    cancel: &AtomicBool,
    mut capture: impl FnMut() -> Result<Frame, String>,
    mut wait: impl FnMut() -> Result<(), String>,
) -> Result<Frame, String> {
    let mut previous: Option<Frame> = None;
    let mut equal = 0;
    for index in 0..5 {
        check(cancel)?;
        if index > 0 {
            wait()?;
            check(cancel)?;
        }
        let current = capture()?;
        check(cancel)?;
        if let Some(old) = previous.as_ref() {
            if old.dimensions() != current.dimensions() {
                return Err("Capture dimensions changed while settling".into());
            }
            let mut same = true;
            for (a, b) in old.bytes().chunks(65536).zip(current.bytes().chunks(65536)) {
                check(cancel)?;
                if a != b {
                    same = false;
                    break;
                }
            }
            equal = if same { equal + 1 } else { 0 };
            if equal >= 2 {
                return Ok(current);
            }
        }
        previous = Some(current);
    }
    Err("Capture did not stabilize; automatic scrolling stopped".into())
}
fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err("Capture stabilization cancelled".into())
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn frame(value: u8) -> Frame {
        Frame::new(2, 2, vec![value; 16].into()).unwrap()
    }
    #[test]
    fn transient_frames_settle_without_retaining_history() {
        let mut values = [1, 2, 2, 2].into_iter();
        let mut count = 0;
        let result = sample(
            &AtomicBool::new(false),
            || {
                count += 1;
                Ok(frame(values.next().unwrap()))
            },
            || Ok(()),
        )
        .unwrap();
        assert_eq!(count, 4);
        assert_eq!(result.bytes(), &[2; 16]);
    }
    #[test]
    fn unstable_and_cancelled_samples_are_bounded() {
        let mut count = 0;
        assert!(sample(
            &AtomicBool::new(false),
            || {
                count += 1;
                Ok(frame(count))
            },
            || Ok(())
        )
        .is_err());
        assert_eq!(count, 5);
        let cancel = AtomicBool::new(false);
        assert!(sample(
            &cancel,
            || Ok(frame(1)),
            || {
                cancel.store(true, Ordering::Release);
                Ok(())
            }
        )
        .is_err());
        assert!(sample(
            &AtomicBool::new(true),
            || panic!("cancelled capture"),
            || panic!("cancelled wait")
        )
        .is_err());
    }
}

use super::*;
use crate::long_capture::{CaptureMode, CaptureSource, Frame, Session};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
fn frame() -> Frame {
    Frame::new(
        4,
        16,
        (0..16).flat_map(|y| vec![y; 16]).collect::<Vec<_>>().into(),
    )
    .unwrap()
}
struct Source {
    scroll: bool,
    fail: Arc<AtomicBool>,
    commits: Arc<Mutex<Vec<bool>>>,
}
impl CaptureSource for Source {
    fn can_scroll(&self) -> bool {
        self.scroll
    }
    fn can_move(&self) -> bool {
        true
    }
    fn capture(&mut self, _: CaptureMode, _: &AtomicBool) -> std::result::Result<Frame, String> {
        Ok(frame())
    }
    fn prepare_move(
        &mut self,
        movement: (i32, i32),
        _: &AtomicBool,
    ) -> std::result::Result<Frame, String> {
        assert_eq!(movement.1, 0);
        if self.fail.load(Ordering::Acquire) {
            Err("fixture".into())
        } else {
            Ok(frame())
        }
    }
    fn finish_move(&mut self, accepted: bool) {
        if accepted {
            self.scroll = false;
        }
        self.commits.lock().unwrap().push(accepted);
    }
}
#[test]
fn hidden_move_commits_ui_reference_only_after_acceptance_and_refreshes_preview() {
    unsafe {
        let input = super::super::tests::input();
        let fail = Arc::new(AtomicBool::new(false));
        let commits = Arc::new(Mutex::new(Vec::new()));
        let session = Session::new(
            Axis::Vertical,
            frame(),
            MatchOptions {
                minimum_overlap: 8,
                maximum_shift: 8,
                maximum_mean_error: 3.0,
            },
        )
        .unwrap();
        *input.job.borrow_mut() = SessionJob::start_with_source(
            session,
            Source {
                scroll: true,
                fail: fail.clone(),
                commits: commits.clone(),
            },
        )
        .unwrap();
        let window = super::super::tests::hidden(&input);
        assert!(input.job.borrow().can_scroll());
        *input.outline.borrow_mut() = Some(
            super::super::outline::Outline::new(window.0, input.movement.region(), false, false)
                .unwrap(),
        );
        let desktop = Region {
            left: 0,
            top: 0,
            width: 1000,
            height: 1000,
        };
        dispatch_with(window.0, &input, 110, desktop);
        assert_eq!(input.movement.region.get().left, 0);
        assert!(input.movement.pending.get().is_some());
        dispatch_with(window.0, &input, 110, desktop); // 在途直接命令也不能排第二次移动。
        super::super::tests::settle(window.0, &input);
        assert!(!input.job.borrow().can_scroll());
        assert_eq!(
            input.job.borrow_mut().scroll_capture(true),
            Err(crate::long_capture::DispatchError::Unavailable)
        );
        for id in [105, 106, 107] {
            assert!(
                !windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                    GetDlgItem(Some(window.0), id).unwrap()
                )
                .as_bool()
            );
        }
        assert!(input.job.borrow().can_move());
        let mut label = [0u16; 64];
        let length = GetDlgItemTextW(window.0, 103, &mut label);
        assert!(String::from_utf16_lossy(&label[..length as usize]).contains("手动滚动"));
        assert_eq!(input.movement.region.get().left, 1);
        assert!(input.movement.pending.get().is_none());
        assert_eq!(
            input.preview.borrow().as_ref().map(|p| (p.width, p.height)),
            Some((5, 16))
        );
        assert_eq!(*commits.lock().unwrap(), [true]);
        fail.store(true, Ordering::Release);
        dispatch_with(window.0, &input, 109, desktop);
        super::super::tests::settle(window.0, &input);
        assert_eq!(input.movement.region.get().left, 1);
        assert!(input.movement.pending.get().is_none());
        assert_eq!(*commits.lock().unwrap(), [true, false]);
        input
            .automatic
            .borrow_mut()
            .start(std::time::Instant::now());
        dispatch_with(window.0, &input, 110, desktop);
        assert!(!input.job.borrow().is_pending());
        input.automatic.borrow_mut().stop();
    }
}
#[test]
fn move_steps_are_overlapping_bounded_and_clamped_at_negative_desktop_edges() {
    let desktop = Region {
        left: -500,
        top: -500,
        width: 1500,
        height: 1500,
    };
    let region = Region {
        left: -490,
        top: -490,
        width: 400,
        height: 100,
    };
    assert_eq!(candidate(region, 110, desktop).unwrap().1, 64);
    assert_eq!(candidate(region, 111, desktop).unwrap().2, 20);
    assert_eq!(candidate(region, 109, desktop).unwrap().1, -10);
    assert_eq!(candidate(region, 108, desktop).unwrap().2, -10);
    assert!(candidate(
        Region {
            left: -500,
            ..region
        },
        109,
        desktop
    )
    .is_none());
    assert!(candidate(Region { width: 1, ..region }, 110, desktop).is_none());
    assert!(candidate(region, 999, desktop).is_none());
    assert!(candidate(
        region,
        110,
        Region {
            width: 1,
            ..desktop
        }
    )
    .is_none());
}
#[test]
fn unsupported_source_disables_all_movement_controls() {
    unsafe {
        let input = super::super::tests::input();
        let window = super::super::tests::hidden(&input);
        for id in 108..=111 {
            assert!(
                !windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                    GetDlgItem(Some(window.0), id).unwrap()
                )
                .as_bool()
            );
        }
    }
}

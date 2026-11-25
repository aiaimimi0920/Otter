//! 宿主位置提交的观察点就必须已经同步编辑条；最终位置正确不足以排除中间追随。
use super::*;
use std::cell::RefCell;
use windows::Win32::UI::Shell::*;
const OBSERVER: usize = 602;
struct Probe {
    panel: HWND,
    samples: RefCell<Vec<(RECT, RECT)>>,
}
struct Observer {
    hwnd: HWND,
    probe: Option<Box<Probe>>,
}
impl Observer {
    unsafe fn new(hwnd: HWND, panel: HWND) -> Self {
        let probe = Box::new(Probe {
            panel,
            samples: RefCell::default(),
        });
        SetWindowSubclass(
            hwnd,
            Some(observe),
            OBSERVER,
            &*probe as *const Probe as usize,
        )
        .ok()
        .unwrap();
        Self {
            hwnd,
            probe: Some(probe),
        }
    }
    fn samples(&self) -> &RefCell<Vec<(RECT, RECT)>> {
        &self.probe.as_ref().unwrap().samples
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        unsafe {
            if !RemoveWindowSubclass(self.hwnd, Some(observe), OBSERVER).as_bool()
                && IsWindow(Some(self.hwnd)).as_bool()
            {
                // 回调数据不能先于仍活着的 subclass 释放，展开时也不能二次 panic。
                let _ = Box::leak(self.probe.take().unwrap());
                if !std::thread::panicking() {
                    panic!("edit-strip-move: subclass removal failed; probe retained");
                }
                eprintln!("edit-strip-move: subclass removal failed; probe retained");
            }
        }
    }
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    value: usize,
) -> LRESULT {
    if msg == WM_WINDOWPOSCHANGED {
        let probe = &*(value as *const Probe);
        let (mut host, mut panel) = (RECT::default(), RECT::default());
        if GetWindowRect(hwnd, &mut host).is_ok() && GetWindowRect(probe.panel, &mut panel).is_ok()
        {
            probe.samples.borrow_mut().push((host, panel));
        }
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn fast_pin_moves_submit_the_toolbar_with_the_complete_frame_not_after_the_host() {
    unsafe {
        let f = Fixture::new();
        crate::desktop::edit_mode::toolbar_mode(f.owner.0, &f.state, true).unwrap();
        let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(f.panel.0, SW_SHOWNOACTIVATE);
        SendMessageW(f.owner.0, WM_ENTERSIZEMOVE, None, None);
        assert!(crate::desktop::pin_window::preview::active(&f.state));
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        let viewport = f.state.canvas.borrow().viewport;
        let probe = Observer::new(f.owner.0, f.panel.0);
        for i in 0..40 {
            let target = RECT {
                left: 260 + i * 3,
                top: 320 + i * 2,
                right: 580 + i * 3,
                bottom: 560 + i * 2,
            };
            let expected = moving_positions(f.owner.0, &f.state, target).unwrap()[0].1;
            probe.samples().borrow_mut().clear();
            SetWindowPos(
                f.owner.0,
                None,
                target.left,
                target.top,
                320,
                240,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            let samples = probe.samples().borrow();
            assert!(!samples.is_empty());
            assert!(
                samples
                    .iter()
                    .all(|(host, panel)| *host != target || *panel == expected),
                "{:?}",
                samples
            );
            let mut actual = RECT::default();
            GetWindowRect(f.panel.0, &mut actual).unwrap();
            assert_eq!(
                actual, expected,
                "pre-submit refresh must not snap the toolbar back"
            );
        }
        drop(probe);
        SendMessageW(f.owner.0, WM_EXITSIZEMOVE, None, None);
        let c = f.state.canvas.borrow();
        assert!(stamp.matches(&c.editor));
        assert_eq!(c.viewport, viewport);
    }
}
#[test]
fn movement_observer_unhooks_before_probe_is_dropped_during_unwind() {
    unsafe {
        let f = Fixture::new();
        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _probe = Observer::new(f.owner.0, f.panel.0);
            assert!(GetWindowSubclass(f.owner.0, Some(observe), OBSERVER, None).as_bool());
            panic!("intentional probe unwind");
        }));
        assert!(failure.is_err());
        assert!(!GetWindowSubclass(f.owner.0, Some(observe), OBSERVER, None).as_bool());
        SetWindowPos(
            f.owner.0,
            None,
            280,
            360,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
}

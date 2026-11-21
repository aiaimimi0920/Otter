//! 宿主位置提交的观察点就必须已经同步编辑条；最终位置正确不足以排除中间追随。
use super::*;
use windows::Win32::UI::Shell::*;
struct Probe {
    panel: HWND,
    samples: Vec<(RECT, RECT)>,
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
        let probe = &mut *(value as *mut Probe);
        let (mut host, mut panel) = (RECT::default(), RECT::default());
        if GetWindowRect(hwnd, &mut host).is_ok() && GetWindowRect(probe.panel, &mut panel).is_ok()
        {
            probe.samples.push((host, panel));
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
        let mut probe = Probe {
            panel: f.panel.0,
            samples: Vec::new(),
        };
        SetWindowSubclass(
            f.owner.0,
            Some(observe),
            602,
            &mut probe as *mut Probe as usize,
        )
        .ok()
        .unwrap();
        for i in 0..40 {
            let target = RECT {
                left: 260 + i * 3,
                top: 320 + i * 2,
                right: 580 + i * 3,
                bottom: 560 + i * 2,
            };
            let expected = moving_positions(f.owner.0, &f.state, target).unwrap()[0].1;
            probe.samples.clear();
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
            assert!(!probe.samples.is_empty());
            assert!(
                probe
                    .samples
                    .iter()
                    .all(|(host, panel)| *host != target || *panel == expected),
                "{:?}",
                probe.samples
            );
            let mut actual = RECT::default();
            GetWindowRect(f.panel.0, &mut actual).unwrap();
            assert_eq!(
                actual, expected,
                "pre-submit refresh must not snap the toolbar back"
            );
        }
        RemoveWindowSubclass(f.owner.0, Some(observe), 602)
            .ok()
            .unwrap();
        SendMessageW(f.owner.0, WM_EXITSIZEMOVE, None, None);
        let c = f.state.canvas.borrow();
        assert!(stamp.matches(&c.editor));
        assert_eq!(c.viewport, viewport);
    }
}

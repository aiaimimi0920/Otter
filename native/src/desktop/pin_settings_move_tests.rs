//! Check more than the drag endpoint: the first host WINDOWPOSCHANGED observation must already see the committed child-strip target position.
use super::*;
use windows::Win32::UI::Shell::*;
struct Probe {
    panel: HWND,
    target: RECT,
    expected: RECT,
    samples: Vec<(bool, RECT, RECT)>,
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
        if GetWindowRect(hwnd, &mut host).is_ok() && host == probe.target {
            let ok = GetWindowRect(probe.panel, &mut panel).is_ok();
            probe.samples.push((ok, panel, probe.expected));
        }
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn sidebar_commits_with_pin_and_toolbar_during_every_drag_step() {
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        let width = field(panel, settings::WIDTH);
        SetWindowTextW(width, w!("456")).unwrap();
        SendMessageW(f.owner.0, WM_ENTERSIZEMOVE, None, None);
        assert!(crate::desktop::pin_window::preview::active(&f.state));
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        let mut probe = Probe {
            panel,
            target: RECT::default(),
            expected: RECT::default(),
            samples: Vec::new(),
        };
        SetWindowSubclass(f.owner.0, Some(observe), 608, &mut probe as *mut _ as usize)
            .ok()
            .unwrap();
        for i in 0..40 {
            let x = 260 + i * 3;
            let y = 320 + i * 2;
            probe.target = RECT {
                left: x,
                top: y,
                right: x + 320,
                bottom: y + 240,
            };
            let positions =
                tool_panel::moving_positions(f.owner.0, &f.state, probe.target).unwrap();
            probe.expected = positions
                .iter()
                .find(|(hwnd, _)| *hwnd == panel)
                .expect("sidebar missing from frame batch")
                .1;
            SetWindowPos(
                f.owner.0,
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            let mut actual = RECT::default();
            GetWindowRect(panel, &mut actual).unwrap();
            assert_eq!(actual, probe.expected);
            assert_eq!(
                field(panel, settings::WIDTH),
                width,
                "drag rebuilt the EDIT"
            );
            let mut text = [0u16; 16];
            let n = GetWindowTextW(width, &mut text) as usize;
            assert_eq!(
                String::from_utf16_lossy(&text[..n]),
                "456",
                "drag cleared the width draft"
            );
        }
        RemoveWindowSubclass(f.owner.0, Some(observe), 608)
            .ok()
            .unwrap();
        assert_eq!(probe.samples.len(), 40);
        for (ok, actual, expected) in probe.samples {
            assert!(ok);
            assert_eq!(actual, expected, "sidebar followed after the pin committed");
        }
        SendMessageW(f.owner.0, WM_EXITSIZEMOVE, None, None);
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
        assert!(f.state.error.borrow().is_none());
        let mut text = [0u16; 16];
        let n = GetWindowTextW(width, &mut text) as usize;
        assert_eq!(String::from_utf16_lossy(&text[..n]), "456");
        let before = history::frame(f.owner.0).unwrap();
        enter(&f, width, "456");
        dispatch(&f);
        assert_eq!(
            history::frame(f.owner.0).unwrap(),
            crate::desktop::pin_settings_geometry::width(
                before,
                456,
                GetDpiForWindow(f.owner.0).max(96)
            )
            .unwrap()
        );
    }
}
#[test]
fn closed_sidebar_is_not_in_the_moving_window_batch() {
    unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        settings::close(&f.state);
        let positions = tool_panel::moving_positions(
            f.owner.0,
            &f.state,
            RECT {
                left: 300,
                top: 400,
                right: 620,
                bottom: 640,
            },
        )
        .unwrap();
        assert!(!positions.iter().any(|(hwnd, _)| *hwnd == panel));
    }
}

#[test]
fn drag_geometry_does_not_refresh_sidebar_fields_but_timer_and_exit_do() {
    crate::desktop::workspace_window_fixture::with_private_desktop(|| unsafe {
        let f = Fixture::new();
        let panel = open(&f);
        let connect = field(panel, 201);
        let mut rect = RECT::default();
        GetWindowRect(f.owner.0, &mut rect).unwrap();
        let set_busy = |busy: bool| {
            f.state.authorization.borrow_mut().request =
                busy.then_some(crate::desktop::device_authorization_ui::Request::ProjectionTargets);
        };
        let enabled = || IsWindowEnabled(connect).as_bool();
        assert!(enabled());
        // Control case: the same translation outside the drag loop still refreshes through the original path.
        set_busy(true);
        SetWindowPos(
            f.owner.0,
            None,
            rect.left + 16,
            rect.top,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        assert!(!enabled(), "non-drag geometry must still refresh");
        set_busy(false);
        SendMessageW(f.owner.0, WM_ENTERSIZEMOVE, None, None);
        assert!(enabled());
        set_busy(true);
        rect.left += 32;
        rect.right += 32;
        SendMessageW(
            f.owner.0,
            WM_MOVING,
            None,
            Some(LPARAM(&mut rect as *mut RECT as isize)),
        );
        let while_moving = enabled();
        SetWindowPos(
            f.owner.0,
            None,
            rect.left,
            rect.top,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let after_move = enabled();
        let wake = crate::desktop::window_geometry_wake::MESSAGE;
        let mut queued = MSG::default();
        assert!(PeekMessageW(&mut queued, Some(f.owner.0), wake, wake, PM_REMOVE).as_bool());
        let before_wake = enabled();
        DispatchMessageW(&queued);
        let after_wake = enabled();
        SendMessageW(f.owner.0, WM_TIMER, Some(WPARAM(0xffff)), None);
        assert!(!enabled(), "non-geometry messages must still refresh");
        set_busy(false);
        SendMessageW(f.owner.0, WM_EXITSIZEMOVE, None, None);
        assert!(enabled(), "must refresh on drag exit");
        assert_eq!(
            [while_moving, after_move, before_wake, after_wake],
            [true; 4]
        );
    });
}

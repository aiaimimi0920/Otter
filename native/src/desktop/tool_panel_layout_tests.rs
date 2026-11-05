//! 真实隐藏 BUTTON 的几何和消息回归；不设置全局 hook、热键或前台窗口。
use super::*;
const PREVIOUS: windows::core::PCWSTR = w!("Otter.LayoutTest.PreviousProc");
const COUNT: windows::core::PCWSTR = w!("Otter.LayoutTest.PositionCount");

struct Fixture {
    panel: Window,
    _owner: Window,
    _state: Box<State>,
}
impl Fixture {
    unsafe fn new() -> Self {
        let state = Box::new(State::new(None, false).unwrap());
        let owner = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("hidden panel layout fixture"),
                WS_POPUP,
                0,
                0,
                640,
                480,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        SetWindowLongPtrW(owner.0, GWLP_USERDATA, (&*state as *const State) as isize);
        let panel = create(owner.0, &state, false).unwrap();
        Self {
            panel,
            _owner: owner,
            _state: state,
        }
    }
}
struct PositionProbe {
    window: HWND,
    previous: isize,
    count: Box<usize>,
}
impl PositionProbe {
    unsafe fn new(window: HWND) -> Self {
        let previous = GetWindowLongPtrW(window, GWLP_WNDPROC);
        assert_ne!(previous, 0);
        let mut count = Box::new(0usize);
        SetPropW(window, PREVIOUS, Some(HANDLE(previous as *mut _))).unwrap();
        SetPropW(
            window,
            COUNT,
            Some(HANDLE((&mut *count as *mut usize).cast())),
        )
        .unwrap();
        assert_eq!(
            SetWindowLongPtrW(window, GWLP_WNDPROC, probe as *const () as isize),
            previous
        );
        Self {
            window,
            previous,
            count,
        }
    }
    unsafe fn count(&self) -> usize {
        *self.count
    }
}
impl Drop for PositionProbe {
    fn drop(&mut self) {
        unsafe {
            SetWindowLongPtrW(self.window, GWLP_WNDPROC, self.previous);
            let _ = RemovePropW(self.window, COUNT);
            let _ = RemovePropW(self.window, PREVIOUS);
        }
    }
}
unsafe extern "system" fn probe(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_WINDOWPOSCHANGING {
        let count = GetPropW(hwnd, COUNT).0 as *mut usize;
        if !count.is_null() {
            *count = (*count).saturating_add(1);
        }
    }
    let previous: WNDPROC = std::mem::transmute(GetPropW(hwnd, PREVIOUS).0);
    CallWindowProcW(previous, hwnd, message, wp, lp)
}

#[test]
fn tool_panel_layout_resize_and_scroll_preserve_exact_geometry_and_release_children() {
    unsafe {
        let fixture = Fixture::new();
        let windows: Vec<_> = entries()
            .map(|(id, _)| GetDlgItem(Some(fixture.panel.0), id as i32).unwrap())
            .collect();
        let panel = fixture.panel.0;
        let owner = fixture._owner.0;
        SetWindowPos(
            panel,
            None,
            0,
            0,
            220,
            150,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let probe = PositionProbe::new(windows[0]);
        windows::Win32::UI::Controls::SetScrollPos(panel, SB_HORZ, 150, true);
        windows::Win32::UI::Controls::SetScrollPos(panel, SB_VERT, 100, true);
        layout(panel);
        assert!(GetScrollPos(panel, SB_HORZ) > 0 && GetScrollPos(panel, SB_VERT) > 0);
        assert!(probe.count() > 0);
        let count = probe.count();
        layout(panel);
        assert_eq!(probe.count(), count);
        assert_geometry(panel);
        assert!(!IsWindowVisible(panel).as_bool());
        drop(probe);
        drop(fixture);
        assert!(!IsWindow(Some(panel)).as_bool() && !IsWindow(Some(owner)).as_bool());
        assert!(windows
            .into_iter()
            .all(|hwnd| !IsWindow(Some(hwnd)).as_bool()));
    }
}
unsafe fn assert_geometry(panel: HWND) {
    let offset = scroll::sync(panel);
    let dpi = GetDpiForWindow(panel);
    for (index, (id, _)) in entries().enumerate() {
        let control = GetDlgItem(Some(panel), id as i32).unwrap();
        let expected = geometry::button(index, super::super::tool_commands::TOOLS.len(), dpi);
        let mut actual = RECT::default();
        GetWindowRect(control, &mut actual).unwrap();
        let mut origin = POINT {
            x: actual.left,
            y: actual.top,
        };
        assert!(ScreenToClient(panel, &mut origin).as_bool());
        assert_eq!(
            (
                origin.x,
                origin.y,
                actual.right - actual.left,
                actual.bottom - actual.top
            ),
            (
                expected.left - offset.x,
                expected.top - offset.y,
                expected.right - expected.left,
                expected.bottom - expected.top
            )
        );
    }
}
#[test]
fn tool_panel_layout_unchanged_positions_do_not_resend_windowpos_messages() {
    unsafe {
        let fixture = Fixture::new();
        let button = GetDlgItem(Some(fixture.panel.0), entries().next().unwrap().0 as i32).unwrap();
        let probe = PositionProbe::new(button);
        assert_geometry(fixture.panel.0);
        for _ in 0..3 {
            layout(fixture.panel.0);
        }
        assert_eq!(
            probe.count(),
            0,
            "unchanged layout sent redundant positioning messages"
        );
        assert!(!IsWindowVisible(fixture.panel.0).as_bool());
    }
}
#[test]
fn tool_panel_layout_corrects_real_geometry_and_keeps_all_controls_and_hidden_state() {
    unsafe {
        let fixture = Fixture::new();
        let button = GetDlgItem(Some(fixture.panel.0), entries().next().unwrap().0 as i32).unwrap();
        SetWindowPos(button, None, 2, 3, 1, 1, SWP_NOZORDER | SWP_NOACTIVATE).unwrap();
        let probe = PositionProbe::new(button);
        layout(fixture.panel.0);
        assert!(probe.count() > 0);
        let count = probe.count();
        layout(fixture.panel.0);
        assert_eq!(probe.count(), count);
        assert_geometry(fixture.panel.0);
        assert!(!IsWindowVisible(fixture.panel.0).as_bool());
        assert!(entries().all(|(id, _)| GetDlgItem(Some(fixture.panel.0), id as i32).is_ok()));
    }
}

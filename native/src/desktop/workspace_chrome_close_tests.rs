//! 隔离桌面的真实关闭消息：首个销毁回调之前，目标图框及端点已经一起不可见。
use super::*;
struct Probe {
    target: Vec<HWND>,
    other: Vec<HWND>,
    observed: Cell<bool>,
    partial: Cell<bool>,
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let probe = &*(data as *const Probe);
    let result = DefSubclassProc(hwnd, msg, wp, lp);
    if msg == WM_DESTROY || (msg == WM_WINDOWPOSCHANGED && !IsWindowVisible(hwnd).as_bool()) {
        probe.observed.set(true);
        probe.partial.set(
            probe.partial.get()
                || probe.target.iter().any(|h| IsWindowVisible(*h).as_bool())
                || probe.other.iter().any(|h| !IsWindowVisible(*h).as_bool()),
        );
    }
    result
}
#[test]
fn close_hides_target_frame_and_ports_before_destroy_without_blinking_other_pin() {
    unsafe {
        let mut app = visible_app();
        let host = app.documents[0].window.0;
        selection::observe(&app.documents[0].state.chrome, host, WM_LBUTTONDOWN);
        app.sync_workspace_chrome(true);
        let ui = &app.chrome.ui;
        let mut target = vec![host, border_handle(ui, 0)];
        target.extend(drawer::moving(ui, host));
        assert!(
            target.len() > 2,
            "fixture must include the selected pin drawer"
        );
        let mut other = vec![app.documents[1].window.0, border_handle(ui, 1)];
        for (hwnd, endpoint, _) in ui.input_windows.borrow().iter() {
            if *endpoint == 0 {
                target.push(*hwnd);
            } else {
                other.push(*hwnd);
            }
        }
        assert!(target
            .iter()
            .chain(&other)
            .all(|h| IsWindowVisible(*h).as_bool()));
        let probe = Probe {
            target,
            other,
            observed: Cell::new(false),
            partial: Cell::new(false),
        };
        SetWindowSubclass(host, Some(observe), 620, &probe as *const _ as usize).unwrap();
        SendMessageW(host, WM_CLOSE, None, None);
        assert!(probe.observed.get());
        assert!(
            !probe.partial.get(),
            "content destruction exposed an orphan frame or hid another pin"
        );
        assert!(!IsWindow(Some(host)).as_bool());
        assert!(probe.target.iter().all(|h| !IsWindowVisible(*h).as_bool()));
        app.shutdown();
    }
}

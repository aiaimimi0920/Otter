//! 自有离屏NOACTIVATE窗口，核真实EDIT/菜单/重入/回收，不使用全局输入或真实profile。
use super::*;
use crate::live_relay_viewer_business::Business;
use serde_json::json;
use windows::Win32::UI::Controls::{EM_GETSEL, EM_SETSEL};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
fn business(value: u32) -> Arc<Business> {
    Arc::new(Business::from_snapshot(&serde_json::to_vec(&json!({"session":{"observationCapabilities":["adapter"]},"observations":[{"observationId":"o","sequence":1,"state":"stable","source":"app_adapter","confidence":"high","observedAtMs":1,"value":value}]})).unwrap()).unwrap())
}
unsafe fn create() -> (RelayWindow, Arc<ViewerSession>, Permit) {
    let (media, permit) = media();
    let window = RelayWindow::new(media.clone(), permit.clone(), 128, 96).unwrap();
    SetWindowPos(
        window.hwnd,
        None,
        -10000,
        -10000,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )
    .unwrap();
    (window, media, permit)
}
unsafe fn open(window: &RelayWindow) -> (HWND, HWND) {
    SendMessageW(
        window.hwnd,
        WM_SYSCOMMAND,
        Some(WPARAM(observation::OPEN)),
        Some(LPARAM(0)),
    );
    window.observation_test().unwrap()
}
unsafe fn text(hwnd: HWND) -> String {
    let mut b = vec![0; GetWindowTextLengthW(hwnd) as usize + 1];
    let n = GetWindowTextW(hwnd, &mut b);
    String::from_utf16(&b[..n as usize]).unwrap()
}
#[test]
fn explicit_menu_reuses_one_readonly_capture_excluded_window_and_preserves_selection_on_heartbeats()
{
    let _com = Apartment::new().unwrap();
    unsafe {
        let (window, media, permit) = create();
        let b = business(7);
        media.publish_business(&permit, b.clone()).unwrap();
        window.refresh().unwrap();
        assert!(window.observation_test().is_none());
        let (panel, edit) = open(&window);
        assert_ne!(GetForegroundWindow(), panel);
        assert_eq!(
            GetWindowLongPtrW(edit, GWL_STYLE) as u32 & ES_READONLY as u32,
            ES_READONLY as u32
        );
        assert!(text(edit).contains("值：7"));
        assert_eq!(open(&window), (panel, edit));
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(2)), Some(LPARAM(7)));
        // 普通media/input/state notice即使发布新的Business Arc，也不应重写EDIT及清空用户选区。
        media
            .publish_business(&permit, Arc::new(b.as_ref().clone()))
            .unwrap();
        window.refresh().unwrap();
        let selection = SendMessageW(edit, EM_GETSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;
        assert_eq!((selection & 0xffff, selection >> 16), (2, 7));
        media.publish_business(&permit, business(9)).unwrap();
        window.refresh().unwrap();
        let selection = SendMessageW(edit, EM_GETSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;
        assert_eq!((selection & 0xffff, selection >> 16), (2, 7));
        assert!(text(edit).contains("值：9"));
        assert!(!text(edit).contains("值：7"));
        let mut affinity = 0;
        GetWindowDisplayAffinity(panel, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        window.close().unwrap();
        assert!(!IsWindow(Some(panel)).as_bool());
        assert!(!IsWindow(Some(edit)).as_bool());
        assert!(media.validate(&permit).is_ok());
    }
}
#[test]
fn closing_inspector_keeps_media_but_revocation_closes_reopened_inspector_and_drops_snapshot() {
    let _com = Apartment::new().unwrap();
    unsafe {
        let (window, media, permit) = create();
        let b = business(7);
        let weak = Arc::downgrade(&b);
        media.publish_business(&permit, b).unwrap();
        window.refresh().unwrap();
        let (panel, _) = open(&window);
        SendMessageW(panel, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        window.refresh().unwrap();
        assert!(window.is_alive());
        assert!(window.observation_test().is_none());
        assert!(media.validate(&permit).is_ok());
        let (panel, edit) = open(&window);
        media.close();
        window.refresh().unwrap();
        assert!(!window.is_alive());
        assert!(!IsWindow(Some(panel)).as_bool());
        assert!(!IsWindow(Some(edit)).as_bool());
        assert!(weak.upgrade().is_none());
    }
}
unsafe extern "system" fn close_parent(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    parent: usize,
) -> LRESULT {
    let result = DefSubclassProc(hwnd, message, wp, lp);
    if message == WM_SETTEXT {
        let _ = DestroyWindow(HWND(parent as *mut _));
    }
    result
}
#[test]
fn reentrant_parent_close_during_edit_update_never_restores_inspector_or_borrowed_snapshot() {
    let _com = Apartment::new().unwrap();
    unsafe {
        let (window, media, permit) = create();
        media.publish_business(&permit, business(1)).unwrap();
        window.refresh().unwrap();
        let (panel, edit) = open(&window);
        assert!(SetWindowSubclass(edit, Some(close_parent), 1, window.hwnd.0 as usize).as_bool());
        media.publish_business(&permit, business(2)).unwrap();
        window.refresh().unwrap();
        assert!(!window.is_alive());
        assert!(window.state.observation.borrow().is_none());
        assert!(!IsWindow(Some(panel)).as_bool());
        assert!(!IsWindow(Some(edit)).as_bool());
    }
}

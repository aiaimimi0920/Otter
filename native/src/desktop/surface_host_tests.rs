//! 隐藏 HWND 运行测试；不显示窗口，不注册热键，不读取用户配置。
use super::*;
use windows::{
    core::w,
    Win32::UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};
struct Window(HWND);
#[test]
fn hit_before_paint_tracks_current_viewport_and_keeps_render_target_lazy() {
    let _apartment = super::super::Apartment::new().unwrap();
    let window = Window(unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!("Otter hit test"),
            WS_POPUP,
            0,
            0,
            160,
            100,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    });
    let scene =
        Scene::parse(br#"{"id":"root","type":"view","layout":{"width":"50%","height":"100%"}}"#)
            .unwrap();
    let mut host = Host::bind(window.0, scene).unwrap();
    assert_eq!(host.hit_test([60, 20]).unwrap(), Some("root"));
    assert_eq!(host.hit_test([100, 20]).unwrap(), None);
    assert!(host.target.is_none());
    unsafe {
        SetWindowPos(
            window.0,
            None,
            0,
            0,
            320,
            100,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
        .unwrap();
    }
    assert_eq!(host.hit_test([100, 20]).unwrap(), Some("root"));
    assert!(host.target.is_none());
    assert_eq!(host.hit_test([-1, 20]).unwrap(), None);
    unsafe {
        SetWindowPos(
            window.0,
            None,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
        .unwrap();
    }
    assert_eq!(host.hit_test([0, 0]).unwrap(), None);
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
#[test]
fn hidden_hwnd_draw_resize_and_release_reuses_prepared_text() {
    let _apartment = super::super::Apartment::new().unwrap();
    let window = Window(unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!("Otter Surface test"),
            WS_POPUP,
            0,
            0,
            160,
            100,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    });
    let scene =
        Scene::parse(br#"{"id":"root","type":"text","props":{"text":"Native Surface text"}}"#)
            .unwrap();
    let mut host = Host::bind(window.0, scene).unwrap();
    host.paint().unwrap();
    let layout = host.presentation().text().items()[0].layout() as *const _;
    let resources = host.target.as_ref().unwrap().clone();
    host.paint().unwrap();
    assert_eq!(resources.0, host.target.as_ref().unwrap().0);
    assert_eq!(resources.1, host.target.as_ref().unwrap().1);
    assert_eq!(
        layout,
        host.presentation().text().items()[0].layout() as *const _
    );
    let dpi = unsafe { GetDpiForWindow(window.0) };
    unsafe {
        SetWindowPos(
            window.0,
            None,
            0,
            0,
            240,
            120,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
        .unwrap();
    }
    host.paint().unwrap();
    assert_eq!(
        host.presentation().size(),
        logical(
            D2D_SIZE_U {
                width: 240,
                height: 120
            },
            dpi
        )
    );
    let bad = Scene::parse(br#"{"id":"bad","type":"image"}"#).unwrap();
    assert!(host.replace(bad).is_err());
    assert_eq!(host.presentation().scene().root().id(), "root");
    assert!(!unsafe { IsWindowVisible(window.0) }.as_bool());
    drop(host);
}

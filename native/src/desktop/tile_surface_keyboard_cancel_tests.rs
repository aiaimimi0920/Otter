//! Esc分层、取消后的Tab位置及同步回收；系统下拉框只在未切换的专属desktop显示。
use super::*;
use windows::Win32::{
    System::{StationsAndDesktops::*, Threading::GetCurrentThreadId},
    UI::Shell::*,
};

struct Desktop {
    old: HDESK,
    owned: HDESK,
}
#[link(name = "imm32")]
unsafe extern "system" {
    fn ImmDisableIME(thread: u32) -> i32;
}
impl Desktop {
    fn new() -> Self {
        unsafe {
            let old = GetThreadDesktop(GetCurrentThreadId()).unwrap();
            let name = windows::core::HSTRING::from(format!(
                "Otter.Tile.Cancel.{}.{}",
                std::process::id(),
                GetCurrentThreadId()
            ));
            // 刻意不申请SWITCHDESKTOP；测试既不切换输入desktop，也不操作用户的窗口。
            let owned = CreateDesktopW(
                &name,
                None,
                None,
                DESKTOP_CONTROL_FLAGS(0),
                DESKTOP_CREATEWINDOW.0
                    | DESKTOP_CREATEMENU.0
                    | DESKTOP_READOBJECTS.0
                    | DESKTOP_WRITEOBJECTS.0,
                None,
            )
            .unwrap();
            if let Err(error) = SetThreadDesktop(owned) {
                let _ = CloseDesktop(owned);
                panic!("isolated desktop assignment failed: {error}");
            }
            let desktop = Self { old, owned };
            // 未切换desktop上的系统IME可能阻塞销毁；只禁用本测试专属线程的真实IME。
            // 下方人工组合消息仅核子类状态位，不冒充已安装IME或候选窗验收。
            assert_ne!(ImmDisableIME(GetCurrentThreadId()), 0);
            desktop
        }
    }
    fn finish(mut self) {
        unsafe {
            SetThreadDesktop(self.old).unwrap();
            CloseDesktop(self.owned).unwrap();
        }
        self.owned = HDESK::default();
    }
}
impl Drop for Desktop {
    fn drop(&mut self) {
        if !self.owned.is_invalid() {
            unsafe {
                let _ = SetThreadDesktop(self.old);
                let _ = CloseDesktop(self.owned);
            }
        }
    }
}
struct Parent(HWND);
impl Drop for Parent {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
unsafe extern "system" fn parent(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if matches!(msg, text::DISMISS | text::CANCEL_EDIT) {
        let count = GetPropW(hwnd, w!("Otter.Test.Dismiss")).0 as usize + 1;
        let _ = SetPropW(
            hwnd,
            w!("Otter.Test.Dismiss"),
            Some(HANDLE(count as *mut _)),
        );
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
#[test]
fn native_select_escape_closes_list_before_editor_and_ime_preserves_select_policy() {
    std::thread::spawn(native_select_on_private_desktop)
        .join()
        .unwrap();
}
fn native_select_on_private_desktop() {
    let desktop = Desktop::new();
    {
        unsafe {
            let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    lpfnWndProc: Some(parent),
                    hInstance: instance,
                    lpszClassName: w!("Otter.Test.Tile.Cancel"),
                    ..Default::default()
                }),
                0
            );
            let owner = Parent(
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("Otter.Test.Tile.Cancel"),
                    w!(""),
                    WS_POPUP,
                    0,
                    0,
                    100,
                    100,
                    None,
                    None,
                    Some(instance),
                    None,
                )
                .unwrap(),
            );
            let popup = text::popup::Popup::new(owner.0).unwrap();
            let scene = crate::surface_scene::Scene::parse(
                &serde_json::to_vec(&json!({
                    "id":"select","type":"select","props":{"value":"a","options":[
                        {"label":"A","value":"a"},{"label":"B","value":"b"}]}
                }))
                .unwrap(),
            )
            .unwrap();
            let options = crate::surface_select::Options::parse(scene.root()).unwrap();
            let control = crate::desktop::surface_select_control::Control::new(
                popup.hwnd().unwrap(),
                RECT {
                    left: 0,
                    top: 0,
                    right: 100,
                    bottom: 24,
                },
                options,
            )
            .unwrap();
            let h = control.hwnd().unwrap();
            popup.install_select(h).unwrap();
            popup.show(owner.0, [0, 0, 100, 24]).unwrap();
            control.show().unwrap();
            control.focus().unwrap();
            // 使用真实Combo默认过程，而不是mock dropped状态或伪造父通知。
            SendMessageW(h, WM_IME_STARTCOMPOSITION, None, None);
            SendMessageW(h, WM_IME_ENDCOMPOSITION, None, None);
            SendMessageW(h, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
            assert_ne!(SendMessageW(h, CB_GETDROPPEDSTATE, None, None).0, 0);
            key(h, WM_KEYDOWN, 27, false);
            key(h, WM_KEYDOWN, 27, true);
            key(h, WM_CHAR, 27, false);
            key(h, WM_KEYUP, 27, false);
            assert_eq!(SendMessageW(h, CB_GETDROPPEDSTATE, None, None).0, 0);
            assert_eq!(GetPropW(owner.0, w!("Otter.Test.Dismiss")).0 as usize, 0);
            assert_eq!(control.value().unwrap(), Some("a"));
            assert!(control.is_alive());
            key(h, WM_KEYDOWN, 27, false);
            key(h, WM_KEYUP, 27, false);
            assert_eq!(GetPropW(owner.0, w!("Otter.Test.Dismiss")).0 as usize, 1);
            let p = popup.hwnd().unwrap();
            let _ = SetFocus(None);
            popup.hide();
            drop(control);
            drop(popup);
            assert!(!IsWindow(Some(h)).as_bool());
            assert!(!IsWindow(Some(p)).as_bool());
        }
    }
    desktop.finish();
}
fn controls() -> Value {
    json!({"id":"root","type":"column","layout":{"width":64,"height":64},"children":[
        {"id":"first","type":"input","layout":{"width":64,"height":16,"minHeight":16},"props":{"value":"原文🙂"}},
        {"id":"select","type":"select","layout":{"width":64,"height":16,"minHeight":16},"props":{"value":"a","options":[{"label":"A","value":"a"},{"label":"B","value":"b"}]}},
        {"id":"slider","type":"slider","layout":{"width":64,"height":16,"minHeight":16},"props":{"value":2,"min":0,"max":10,"step":2}},
        {"id":"send","type":"button","layout":{"width":64,"height":16,"minHeight":16},"events":{"click":"go"}}
    ]})
}
#[test]
fn cancelled_text_select_and_slider_keep_tab_origin_without_closing_tile_or_replaying() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let foreground = unsafe { GetForegroundWindow() };
    let (f, w) = setup(controls());
    for node in ["first", "select", "slider"] {
        navigate(w.hwnd, &w.state, false);
        assert_eq!(focus(&w), node);
        let h = if node == "first" {
            text::active_child(&w.state)
        } else {
            child(&w)
        };
        let p = unsafe { GetParent(h).unwrap() };
        unsafe {
            key(h, WM_KEYDOWN, 27, false);
        }
        assert!(w.alive());
        assert_eq!(focus(&w), node);
        assert!(!unsafe { IsWindow(Some(h)) }.as_bool());
        assert!(!unsafe { IsWindow(Some(p)) }.as_bool());
        assert!(events(&f).is_empty());
    }
    navigate(w.hwnd, &w.state, false);
    assert_eq!(focus(&w), "send");
    navigate(w.hwnd, &w.state, true);
    assert_eq!(focus(&w), "slider");
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
}
#[test]
fn cancel_retires_remote_edit_suffix_but_preserves_accepted_local_value() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for remote in [false, true] {
        let mut value = json!({"id":"slider","type":"slider","layout":{"width":64,"height":64},
            "props":{"value":2,"min":0,"max":10,"step":2}});
        if remote {
            value["events"] = json!({"input":"preview","change":"save"});
        }
        let (f, w) = setup(value);
        navigate(w.hwnd, &w.state, false);
        let h = child(&w);
        let edit = w
            .state
            .keyboard_value
            .borrow()
            .as_ref()
            .unwrap()
            .edit
            .clone();
        unsafe {
            key(h, WM_KEYDOWN, 39, false);
        }
        assert_eq!(events(&f).len(), if remote { 2 } else { 0 });
        unsafe {
            key(h, WM_KEYDOWN, 27, false);
        }
        assert!(!edit.active());
        assert!(events(&f).is_empty());
        assert_eq!(focus(&w), "slider");
        navigate(w.hwnd, &w.state, false);
        assert_eq!(
            w.state
                .keyboard_value
                .borrow()
                .as_ref()
                .unwrap()
                .displayed_value(),
            if remote { json!(2.) } else { json!(4.) }
        );
    }
}
#[test]
fn late_dismiss_cannot_cancel_a_new_popup_even_if_windows_reuses_its_handle() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (_f, w) = setup(controls());
    navigate(w.hwnd, &w.state, false);
    let old = text::active_child(&w.state);
    let p = unsafe { GetParent(old).unwrap() };
    let epoch = unsafe { GetPropW(p, w!("Otter.Tile.Text.Epoch")).0 as usize };
    unsafe {
        key(old, WM_KEYDOWN, 27, false);
    }
    navigate(w.hwnd, &w.state, false);
    let successor = child(&w);
    unsafe {
        SendMessageW(
            w.hwnd,
            text::DISMISS,
            Some(WPARAM(epoch)),
            Some(LPARAM(p.0 as isize)),
        );
    }
    assert_eq!(focus(&w), "select");
    assert_eq!(child(&w), successor);
    assert!(unsafe { IsWindow(Some(successor)) }.as_bool());
}
unsafe extern "system" fn cancel_during_destroy(
    h: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        retire(&*(data as *const State));
        let _ = RemoveWindowSubclass(h, Some(cancel_during_destroy), 0x43584c);
    }
    DefSubclassProc(h, msg, wp, lp)
}
#[test]
fn reentrant_cancel_and_preparation_gap_never_restore_retired_keyboard_eligibility() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for reentrant in [false, true] {
        let (f, w) = setup(controls());
        navigate(w.hwnd, &w.state, false);
        let h = text::active_child(&w.state);
        if reentrant {
            unsafe {
                assert!(SetWindowSubclass(
                    h,
                    Some(cancel_during_destroy),
                    0x43584c,
                    (&*w.state as *const State) as usize
                )
                .as_bool());
            }
        } else {
            f.shared
                .lock()
                .unavailable(f.job.generation, &f.job.key, "preparing");
        }
        unsafe {
            key(h, WM_KEYDOWN, 27, false);
        }
        assert!(w.alive());
        assert!(w.state.keyboard.borrow().focus.is_none());
        assert!(w.state.text.borrow().is_none());
        assert!(w.state.keyboard_value.borrow().is_none());
        assert!(events(&f).is_empty());
    }
}

#[test]
fn external_deactivation_does_not_retain_escape_only_navigation_privilege() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for value in [false, true] {
        let (f, w) = setup(controls());
        navigate(w.hwnd, &w.state, false);
        if value {
            navigate(w.hwnd, &w.state, false);
        }
        let h = if value {
            child(&w)
        } else {
            text::active_child(&w.state)
        };
        let popup = unsafe { GetParent(h).unwrap() };
        let epoch = unsafe { GetPropW(popup, w!("Otter.Tile.Text.Epoch")).0 as usize };
        unsafe {
            SendMessageW(
                w.hwnd,
                text::DISMISS,
                Some(WPARAM(epoch)),
                Some(LPARAM(popup.0 as isize)),
            );
        }
        assert!(w.alive());
        assert!(w.state.keyboard.borrow().focus.is_none());
        assert!(w.state.text.borrow().is_none());
        assert!(w.state.keyboard_value.borrow().is_none());
        assert!(events(&f).is_empty());
    }
}
unsafe extern "system" fn successor_during_destroy(
    h: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        let state = &*(data as *const State);
        navigate(GetAncestor(h, GA_ROOTOWNER), state, false);
        let _ = RemoveWindowSubclass(h, Some(successor_during_destroy), 0x535543);
    }
    DefSubclassProc(h, msg, wp, lp)
}
#[test]
fn synchronous_successor_from_value_destruction_survives_old_escape_cleanup() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (_f, w) = setup(controls());
    navigate(w.hwnd, &w.state, false);
    navigate(w.hwnd, &w.state, false);
    assert_eq!(focus(&w), "select");
    let h = child(&w);
    unsafe {
        assert!(SetWindowSubclass(
            h,
            Some(successor_during_destroy),
            0x535543,
            (&*w.state as *const State) as usize
        )
        .as_bool());
        key(h, WM_KEYDOWN, 27, false);
    }
    assert_eq!(focus(&w), "first");
    assert!(w.state.text.borrow().is_some());
    assert!(unsafe { IsWindow(Some(text::active_child(&w.state))) }.as_bool());
}
#[test]
fn escape_repeat_after_cancel_does_not_close_tile_but_next_fresh_escape_still_exits() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (_f, w) = setup(controls());
    navigate(w.hwnd, &w.state, false);
    let h = text::active_child(&w.state);
    unsafe {
        key(h, WM_KEYDOWN, 27, false);
        key(w.hwnd, WM_KEYDOWN, 27, true);
        key(w.hwnd, WM_KEYUP, 27, false);
    }
    assert!(w.alive());
    assert_eq!(focus(&w), "first");
    unsafe {
        key(w.hwnd, WM_KEYDOWN, 27, false);
    }
    assert!(!w.alive());
}

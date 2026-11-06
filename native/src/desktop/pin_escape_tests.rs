//! 隔离 desktop 上的真实宿主消息回归；不注入输入、不代替普通前台手测。
use super::*;
use crate::desktop::{Apartment, Window};
use crate::editor::Tool;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

thread_local! { static MOVE_ENTRIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
unsafe extern "system" fn observed_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_ENTERSIZEMOVE {
        MOVE_ENTRIES.set(MOVE_ENTRIES.get() + 1);
    }
    crate::desktop::procedure::window_proc(hwnd, msg, wp, lp)
}

struct Keys([u8; 256]);
impl Keys {
    unsafe fn held(held: &[VIRTUAL_KEY]) -> Self {
        let mut original = [0; 256];
        GetKeyboardState(&mut original).unwrap();
        let mut keys = [0; 256];
        for key in held {
            keys[key.0 as usize] = 0x80;
        }
        SetKeyboardState(&keys).unwrap();
        Self(original)
    }
}
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe { SetKeyboardState(&self.0).unwrap() };
    }
}
struct Fixture {
    panel: Window,
    owner: Window,
    state: Box<State>,
    class: HSTRING,
    instance: HINSTANCE,
}
impl Fixture {
    unsafe fn new(pinned: bool) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let class = HSTRING::from(format!(
            "Otter.Native.PinEscape.{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: PCWSTR(class.as_ptr()),
                lpfnWndProc: Some(observed_proc),
                ..Default::default()
            }),
            0
        );
        let image = crate::image_decode::Image {
            width: 4,
            height: 4,
            pixels: [10, 20, 30, 255].repeat(16).into(),
        };
        let state = Box::new(State::new(Some(image), pinned).unwrap());
        let owner = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(class.as_ptr()),
                w!("pin escape fixture"),
                WS_POPUP,
                0,
                0,
                320,
                240,
                None,
                None,
                Some(instance),
                Some((&*state as *const State).cast()),
            )
            .unwrap(),
        );
        let panel = crate::desktop::tool_panel::create(owner.0, &state, false).unwrap();
        Self {
            panel,
            owner,
            state,
            class,
            instance,
        }
    }
    unsafe fn escape(&self, repeat: bool) {
        SendMessageW(
            self.owner.0,
            WM_KEYDOWN,
            Some(WPARAM(VK_ESCAPE.0 as usize)),
            Some(LPARAM(if repeat { 1 << 30 } else { 0 })),
        );
    }
    unsafe fn alive(&self) -> bool {
        self.state.lifetime.owns(self.owner.0) && IsWindow(Some(self.owner.0)).as_bool()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            if self.alive() {
                DestroyWindow(self.owner.0).unwrap();
            }
            self.panel.0 = HWND::default();
            self.owner.0 = HWND::default();
            UnregisterClassW(PCWSTR(self.class.as_ptr()), Some(self.instance)).unwrap();
        }
    }
}

#[test]
fn idle_pinned_escape_closes_only_its_host_and_owned_panel() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let _keys = Keys::held(&[]);
        let selected = Fixture::new(true);
        let other = Fixture::new(true);
        assert_eq!(
            SendMessageW(selected.owner.0, WM_NCHITTEST, None, None).0,
            HTCAPTION as isize
        );
        selected.escape(false);
        assert!(
            !selected.alive(),
            "idle pinned Esc must close the target host"
        );
        assert!(!IsWindow(Some(selected.panel.0)).as_bool());
        assert!(!selected.state.exit_requested.get());
        assert!(other.alive());
    }
}

#[test]
fn editing_and_selection_escape_cancel_without_closing_the_pin() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let _keys = Keys::held(&[]);
        let fixture = Fixture::new(true);
        SendMessageW(fixture.owner.0, WM_KEYDOWN, Some(WPARAM(0x42)), None);
        assert_eq!(fixture.state.canvas.borrow().tool, Tool::Brush);
        let mut client = RECT::default();
        GetClientRect(fixture.owner.0, &mut client).unwrap();
        // 钉图内容会等比居中；客户区左上角可能是留白，不是有效的落笔位置。
        let center = LPARAM((client.right / 2) as isize | ((client.bottom / 2) as isize) << 16);
        SendMessageW(fixture.owner.0, WM_LBUTTONDOWN, None, Some(center));
        assert!(fixture.state.canvas.borrow().editor.draft().is_some());
        fixture.escape(false);
        assert!(fixture.alive());
        assert!(fixture.state.canvas.borrow().editor.draft().is_none());
        assert_eq!(fixture.state.canvas.borrow().tool, Tool::Brush);
        fixture.state.canvas.borrow_mut().select_tool(Tool::Select);
        fixture.state.canvas.borrow_mut().selected = Some((0, 0));
        fixture.escape(false);
        assert!(fixture.alive());
        assert!(fixture.state.canvas.borrow().selected.is_none());
        fixture.state.canvas.borrow_mut().select_tool(Tool::Pan);
        fixture.escape(true);
        assert!(
            fixture.alive(),
            "repeat after cancellation must not close the pin"
        );
        fixture.escape(false);
        assert!(!fixture.alive());
    }
}

#[test]
fn pan_gesture_cancels_before_a_fresh_escape_can_close() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let _keys = Keys::held(&[]);
        let fixture = Fixture::new(true);
        SendMessageW(
            fixture.owner.0,
            WM_LBUTTONDOWN,
            None,
            Some(LPARAM(1 | (1 << 16))),
        );
        assert!(fixture.state.canvas.borrow().drag.is_some());
        fixture.escape(false);
        assert!(fixture.alive());
        assert!(fixture.state.canvas.borrow().drag.is_none());
        fixture.escape(true);
        assert!(fixture.alive());
        fixture.escape(false);
        assert!(!fixture.alive());
    }
}

#[test]
fn modifiers_modal_capture_and_group_lock_do_not_delete_the_pin() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let _keys = Keys::held(&[]);
        let fixture = Fixture::new(true);
        for key in [VK_CONTROL, VK_SHIFT, VK_MENU] {
            let _modifier = Keys::held(&[key]);
            fixture.escape(false);
            assert!(fixture.alive());
        }
        for flag in [
            &*fixture.state.file_dialog,
            &*fixture.state.capture_active,
            &fixture.state.group_locked,
            &fixture.state.group_hidden,
        ] {
            flag.set(true);
            fixture.escape(false);
            assert!(fixture.alive());
            flag.set(false);
        }
        let modal = std::rc::Rc::new(std::cell::Cell::new(true));
        fixture.state.modals.register(&modal);
        fixture.escape(false);
        assert!(fixture.alive());
        modal.set(false);
        fixture.escape(false);
        assert!(!fixture.alive());
    }
}

#[test]
fn unpinned_workspace_escape_does_not_close_the_window() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let _keys = Keys::held(&[]);
        let fixture = Fixture::new(false);
        fixture.escape(false);
        assert!(fixture.alive());
    }
}

unsafe fn move_escape(fixture: &Fixture, repeat: bool) -> MSG {
    PostMessageW(
        Some(fixture.owner.0),
        WM_KEYDOWN,
        WPARAM(VK_ESCAPE.0 as usize),
        LPARAM(if repeat { 1 << 30 } else { 0 }),
    )
    .unwrap();
    let mut message = MSG::default();
    assert!(PeekMessageW(
        &mut message,
        Some(fixture.owner.0),
        WM_KEYDOWN,
        WM_KEYDOWN,
        PM_REMOVE
    )
    .as_bool());
    message
}

#[test]
fn system_move_escape_becomes_close_without_exiting_or_rolling_back_first() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let _keys = Keys::held(&[]);
        let selected = Fixture::new(true);
        let other = Fixture::new(true);
        SendMessageW(selected.owner.0, WM_ENTERSIZEMOVE, None, None);
        SetWindowPos(
            selected.owner.0,
            None,
            160,
            180,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let message = move_escape(&selected, false);
        assert!(
            selected.alive(),
            "hook must not destroy a window inside the system loop"
        );
        assert_eq!(
            message.message, WM_CLOSE,
            "system loop must not receive Esc and roll back"
        );
        assert_eq!(message.hwnd, selected.owner.0);
        let mut rect = RECT::default();
        GetWindowRect(selected.owner.0, &mut rect).unwrap();
        assert_eq!((rect.left, rect.top), (160, 180));
        DispatchMessageW(&message);
        assert!(!selected.alive());
        assert!(!IsWindow(Some(selected.panel.0)).as_bool());
        assert!(other.alive());
        // 已退出的线程过滤器不能污染下一次普通移动。
        assert_eq!(move_escape(&other, false).message, WM_KEYDOWN);
        SendMessageW(other.owner.0, WM_ENTERSIZEMOVE, None, None);
        SendMessageW(other.owner.0, WM_EXITSIZEMOVE, None, None);
        assert!(other.alive());
    }
}

#[test]
fn system_move_release_repeat_modifiers_and_unpinned_do_not_delete() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let _keys = Keys::held(&[]);
        for (pinned, repeat, modifier, escape) in [
            (true, false, false, false),
            (true, true, false, true),
            (true, false, true, true),
            (false, false, false, true),
        ] {
            let fixture = Fixture::new(pinned);
            let _modifier = Keys::held(if modifier { &[VK_CONTROL] } else { &[] });
            SendMessageW(fixture.owner.0, WM_ENTERSIZEMOVE, None, None);
            if escape {
                assert_eq!(move_escape(&fixture, repeat).message, WM_KEYDOWN);
            }
            SendMessageW(fixture.owner.0, WM_EXITSIZEMOVE, None, None);
            assert!(fixture.alive());
        }
    }
}

#[test]
fn actual_system_move_loop_dispatches_escape_as_close() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let _keys = Keys::held(&[]);
        let fixture = Fixture::new(true);
        let _ = ShowWindow(fixture.owner.0, SW_SHOWNOACTIVATE);
        MOVE_ENTRIES.set(0);
        PostMessageW(
            Some(fixture.owner.0),
            WM_KEYDOWN,
            WPARAM(VK_RIGHT.0 as usize),
            LPARAM(0),
        )
        .unwrap();
        PostMessageW(
            Some(fixture.owner.0),
            WM_KEYDOWN,
            WPARAM(VK_ESCAPE.0 as usize),
            LPARAM(0),
        )
        .unwrap();
        // 真正进入 DefWindowProc 的系统移动循环；只向隔离测试窗口排队，不注入桌面输入。
        SendMessageW(
            fixture.owner.0,
            WM_SYSCOMMAND,
            Some(WPARAM(SC_MOVE as usize)),
            None,
        );
        assert!(
            MOVE_ENTRIES.get() > 0,
            "system move must actually enter its loop"
        );
        assert!(
            !fixture.alive(),
            "native move loop must dispatch close instead of cancelling position"
        );
    }
}

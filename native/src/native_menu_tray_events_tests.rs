//! 真实 WinEvent hook 验证托盘根菜单周期；不将自有 HWND 冒充 Explorer 验收。
use super::*;
use std::cell::{Cell, RefCell};
use windows::Win32::{
    System::Threading::{GetCurrentProcessId, GetCurrentThreadId},
    UI::Accessibility::*,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Observed {
    event: u32,
    hwnd: usize,
    object: i32,
    child: i32,
    visible: bool,
}
thread_local! {
    static EVENTS: RefCell<Vec<Observed>> = const { RefCell::new(Vec::new()) };
    static DESTROY_ON: Cell<u32> = const { Cell::new(0) };
}
struct Watch(HWINEVENTHOOK);
impl Watch {
    fn new() -> Self {
        EVENTS.with(|events| events.borrow_mut().clear());
        DESTROY_ON.set(0);
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_MENUPOPUPSTART,
                EVENT_SYSTEM_MENUPOPUPEND,
                None,
                Some(observe),
                GetCurrentProcessId(),
                GetCurrentThreadId(),
                WINEVENT_OUTOFCONTEXT,
            )
        };
        assert!(!hook.is_invalid());
        Self(hook)
    }
    fn events(&self) -> Vec<Observed> {
        unsafe {
            let mut message = MSG::default();
            for _ in 0..128 {
                if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        EVENTS.with(|events| events.borrow().clone())
    }
    fn pair(&self, root: HWND) {
        let events = self.events();
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0].event, EVENT_SYSTEM_MENUPOPUPSTART);
        assert_eq!(events[1].event, EVENT_SYSTEM_MENUPOPUPEND);
        for event in events {
            assert_eq!(event.hwnd, root.0 as usize);
            assert_eq!(event.object, OBJID_CLIENT.0);
            assert_eq!(event.child, 0);
        }
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        DESTROY_ON.set(0);
        assert!(unsafe { UnhookWinEvent(self.0) }.as_bool());
    }
}
unsafe extern "system" fn observe(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object: i32,
    child: i32,
    _: u32,
    _: u32,
) {
    EVENTS.with(|events| {
        events.borrow_mut().push(Observed {
            event,
            hwnd: hwnd.0 as usize,
            object,
            child,
            visible: IsWindowVisible(hwnd).as_bool(),
        });
    });
    if DESTROY_ON.get() == event {
        DESTROY_ON.set(0);
        let _ = DestroyWindow(hwnd);
    }
}
fn open(owner: HWND) -> Cascade {
    let mut parent = tree().remove(0);
    parent.children = Some(tree());
    Cascade::open_with_tray_mode(owner, vec![parent], POINT { x: 30, y: 30 }, true).unwrap()
}

#[test]
fn tray_root_announces_one_visible_popup_across_child_navigation() {
    let watch = Watch::new();
    let owner = Owner::new();
    let mut ordinary = Cascade::open(owner.0, tree(), POINT { x: 30, y: 30 }).unwrap();
    ordinary.cancel();
    assert!(watch.events().is_empty());
    let mut menu = open(owner.0);
    let root = menu.pages[0].panel.handle().unwrap();
    assert_eq!(watch.events().len(), 1);
    assert!(watch.events()[0].visible);
    menu.event(0, Event::Move(8, 8)).unwrap();
    assert_eq!(menu.pages.len(), 2);
    menu.key(0x25).unwrap();
    assert_eq!(menu.pages.len(), 1);
    assert_eq!(watch.events().len(), 1);
    menu.cancel();
    watch.pair(root);
    assert!(!unsafe { IsWindow(Some(root)).as_bool() });
}

#[test]
fn activation_refusal_and_keyboard_selection_both_finish_the_popup() {
    for reject in [true, false] {
        let watch = Watch::new();
        let owner = Owner::new();
        let foreground = unsafe { GetForegroundWindow() };
        let mut root = HWND::default();
        let result = run_updated(
            owner.0,
            tree(),
            POINT { x: 30, y: 30 },
            foreground,
            (
                |hwnd| {
                    root = hwnd;
                    assert_eq!(watch.events().len(), 1);
                    assert!(watch.events()[0].visible);
                    if reject {
                        Err(Error::new(E_ACCESSDENIED, "synthetic activation refusal"))
                    } else {
                        Ok(foreground)
                    }
                },
                |hwnd| unsafe {
                    PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x28), LPARAM(0)).unwrap();
                    PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x0d), LPARAM(0)).unwrap();
                },
            ),
            |_| Ok(()),
            true,
            320,
        );
        if reject {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap(), Some(42));
        }
        watch.pair(root);
        assert!(unsafe { GetCapture() }.is_invalid());
    }
}

#[test]
fn external_hide_root_or_owner_destruction_finishes_exactly_once() {
    for action in 0..3 {
        let watch = Watch::new();
        let owner = Owner::new();
        let mut menu = open(owner.0);
        let root = menu.pages[0].panel.handle().unwrap();
        assert_eq!(watch.events().len(), 1);
        unsafe {
            match action {
                0 => {
                    let _ = ShowWindow(root, SW_HIDE);
                }
                1 => DestroyWindow(root).unwrap(),
                _ => DestroyWindow(owner.0).unwrap(),
            }
        }
        watch.pair(root);
        assert!(!watch.events()[1].visible);
        menu.cancel();
        watch.pair(root);
    }
}

#[test]
fn event_listener_can_destroy_the_popup_without_duplicate_end() {
    for event in [EVENT_SYSTEM_MENUPOPUPSTART, EVENT_SYSTEM_MENUPOPUPEND] {
        let watch = Watch::new();
        let owner = Owner::new();
        DESTROY_ON.set(event);
        let mut menu = open(owner.0);
        let root = menu.pages[0].panel.handle().unwrap();
        watch.events();
        if event == EVENT_SYSTEM_MENUPOPUPEND {
            unsafe {
                let _ = ShowWindow(root, SW_HIDE);
            }
            watch.events();
        }
        assert!(!unsafe { IsWindow(Some(root)).as_bool() });
        menu.cancel();
        watch.pair(root);
    }
}

//! 真实隐藏宿主的右键→菜单消息路由；只截获菜单边界，不替代普通前台菜单验收。
use super::*;
use crate::desktop::{Apartment, Window};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::{
    System::LibraryLoader::GetModuleHandleW,
    UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
};

#[derive(Default)]
struct Seen {
    count: Cell<u32>,
    last: Cell<(usize, isize)>,
    state: Cell<*const State>,
}
unsafe extern "system" fn observe(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if message == WM_CONTEXTMENU {
        let seen = &*(data as *const Seen);
        // 原生同步发送时不得继续借用 Canvas；在边界实际取得可变借用来保护重入。
        let _canvas = (*seen.state.get()).canvas.borrow_mut();
        seen.count.set(seen.count.get() + 1);
        seen.last.set((wp.0, lp.0));
        return LRESULT(0);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}
struct Fixture {
    owner: Window,
    state: Box<State>,
    seen: Box<Seen>,
    class: HSTRING,
    instance: HINSTANCE,
}
impl Fixture {
    unsafe fn new(pinned: bool) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let class = HSTRING::from(format!(
            "Otter.Native.ContextRoute.{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: PCWSTR(class.as_ptr()),
                lpfnWndProc: Some(crate::desktop::procedure::window_proc),
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
                w!("hidden context route fixture"),
                WS_POPUP,
                100,
                100,
                320,
                240,
                None,
                None,
                Some(instance),
                Some((&*state as *const State).cast()),
            )
            .unwrap(),
        );
        let seen = Box::<Seen>::default();
        seen.state.set(&*state);
        assert!(
            SetWindowSubclass(owner.0, Some(observe), 1, (&*seen as *const Seen) as usize)
                .as_bool()
        );
        Self {
            owner,
            state,
            seen,
            class,
            instance,
        }
    }
    unsafe fn send(&self, message: u32, wp: usize, lp: isize) {
        SendMessageW(self.owner.0, message, Some(WPARAM(wp)), Some(LPARAM(lp)));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            assert!(RemoveWindowSubclass(self.owner.0, Some(observe), 1).as_bool());
            DestroyWindow(self.owner.0).unwrap();
            UnregisterClassW(PCWSTR(self.class.as_ptr()), Some(self.instance)).unwrap();
        }
    }
}

#[test]
fn pinned_caption_right_release_routes_once_with_screen_coordinates_and_no_model_change() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new(true);
        let before = fixture.state.canvas.borrow().editor.document_stamp();
        let point = ((-120i16 as u16 as u32) | (140u32 << 16)) as isize;
        assert_eq!(
            SendMessageW(fixture.owner.0, WM_NCHITTEST, None, Some(LPARAM(point))).0,
            HTCAPTION as isize
        );
        fixture.send(WM_NCRBUTTONDOWN, HTCAPTION as usize, point);
        assert_eq!(fixture.seen.count.get(), 0);
        fixture.send(WM_NCRBUTTONUP, HTCAPTION as usize, point);
        assert_eq!(fixture.seen.count.get(), 1);
        assert_eq!(
            fixture.seen.last.get(),
            (fixture.owner.0 .0 as usize, point)
        );
        assert!(before.matches(&fixture.state.canvas.borrow().editor));
    }
}

#[test]
fn client_right_release_and_keyboard_menu_remain_single_dispatch() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new(true);
        fixture.state.canvas.borrow_mut().select_tool(Tool::Brush);
        fixture.send(WM_RBUTTONUP, 0, 8 | (10 << 16));
        assert_eq!(fixture.seen.count.get(), 1);
        fixture.send(WM_CONTEXTMENU, fixture.owner.0 .0 as usize, -1);
        assert_eq!(fixture.seen.count.get(), 2);
        assert_eq!(fixture.seen.last.get().1, -1);
    }
}

#[test]
fn unpinned_caption_right_release_is_not_reclassified_as_business_menu() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new(false);
        fixture.send(WM_NCRBUTTONUP, HTCAPTION as usize, 120 | (140 << 16));
        assert_eq!(fixture.seen.count.get(), 0);
    }
}

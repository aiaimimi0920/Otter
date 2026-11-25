//! Hidden owned history windows/standard buttons; no foreground requests, clipboard operations, or other-process interaction.
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};

struct TextProbe {
    child: HWND,
    messages: Cell<usize>,
    moves: Cell<usize>,
}
impl TextProbe {
    unsafe fn new(child: HWND) -> Box<Self> {
        let probe = Box::new(Self {
            child,
            messages: Cell::new(0),
            moves: Cell::new(0),
        });
        assert!(SetWindowSubclass(
            child,
            Some(observe_text),
            1,
            &*probe as *const Self as usize
        )
        .as_bool());
        probe
    }
}
impl Drop for TextProbe {
    fn drop(&mut self) {
        unsafe {
            let _ = RemoveWindowSubclass(self.child, Some(observe_text), 1);
        }
    }
}
unsafe extern "system" fn observe_text(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let probe = &*(data as *const TextProbe);
    match msg {
        WM_SETTEXT => probe.messages.set(probe.messages.get() + 1),
        WM_WINDOWPOSCHANGING => probe.moves.set(probe.moves.get() + 1),
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
unsafe fn caption(child: HWND) -> String {
    let mut text = [0; 128];
    let len = GetWindowTextW(child, &mut text) as usize;
    String::from_utf16(&text[..len]).unwrap()
}

#[test]
fn owned_buttons_display_both_lists_queue_stable_actions_and_close_without_quitting_application() {
    unsafe {
        let _com = crate::desktop::Apartment::new().unwrap();
        let source = crate::desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("history fixture"),
                WS_OVERLAPPED,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let shared: Shared = Default::default();
        shared
            .borrow_mut()
            .color(crate::stroke_style::Color(1, 2, 3));
        let image = crate::image_decode::Image {
            width: 480,
            height: 240,
            pixels: [0, 0, 255, 255].repeat(480 * 240).into(),
        };
        shared
            .borrow_mut()
            .screenshot(crate::capture_history::image::capture("id".into(), &image, 1).unwrap());
        let origin = Origin {
            unit: crate::workspace_id::UnitId::parse("fixture").unwrap(),
            lifetime: Default::default(),
        };
        let mut window = Window::new(source.0, origin, shared.clone()).unwrap();
        let snapshot = window.state.data.borrow().clone();
        assert!(Arc::ptr_eq(&snapshot, &shared.borrow().data));
        let hwnd = window.handle().unwrap();
        let color = GetDlgItem(Some(hwnd), 100).unwrap();
        let screenshot = GetDlgItem(Some(hwnd), 200).unwrap();
        assert!(IsChild(hwnd, color).as_bool());
        assert!(IsChild(hwnd, screenshot).as_bool());
        SendMessageW(color, BM_CLICK, None, None);
        assert!(
            matches!(shared.borrow_mut().action.take(), Some((_, Action::Color(_, hex))) if hex == "#010203")
        );
        SendMessageW(screenshot, BM_CLICK, None, None);
        assert!(
            matches!(shared.borrow_mut().action.take(), Some((_, Action::Reuse(id))) if id == "id")
        );
        let delete = GetDlgItem(Some(hwnd), 300).unwrap();
        SendMessageW(delete, BM_CLICK, None, None);
        assert!(
            matches!(shared.borrow_mut().action.take(), Some((_, Action::Delete(id))) if id == "id")
        );
        let caption_probe = TextProbe::new(color);
        let shot_probe = TextProbe::new(screenshot);
        let delete_probe = TextProbe::new(delete);
        assert_eq!(caption(color), "#010203");
        let screenshot_label = crate::ui_text::format(
            "capture-history-image",
            &[
                ("index", 1.into()),
                ("width", 480.into()),
                ("height", 240.into()),
            ],
        );
        assert_eq!(caption(screenshot), screenshot_label);
        assert_eq!(
            caption(delete),
            crate::ui_text::format("capture-history-delete", &[("index", 1.into())])
        );
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            400,
            180,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let mut before = RECT::default();
        GetWindowRect(screenshot, &mut before).unwrap();
        SendMessageW(
            hwnd,
            WM_MOUSEWHEEL,
            Some(WPARAM(((-120_i16) as u16 as usize) << 16)),
            None,
        );
        let mut after = RECT::default();
        GetWindowRect(screenshot, &mut after).unwrap();
        assert!(after.top < before.top);
        let wheel_scroll = window.state.scroll.get();
        SendMessageW(
            hwnd,
            WM_COMMAND,
            Some(WPARAM(300 | ((BN_SETFOCUS as usize) << 16))),
            Some(LPARAM(delete.0 as isize)),
        );
        assert!(window.state.scroll.get() > wheel_scroll);
        assert_eq!(GetDlgItem(Some(hwnd), 200).unwrap(), screenshot);
        for probe in [&caption_probe, &shot_probe, &delete_probe] {
            assert_eq!(probe.messages.get(), 0);
        }
        assert_eq!(caption(screenshot), screenshot_label);
        // A real scroll moves the controls; repeating at the lower bound must not move them.
        let moves = shot_probe.moves.get();
        SendMessageW(hwnd, WM_VSCROLL, Some(WPARAM(SB_BOTTOM.0 as usize)), None);
        let bottom = window.state.scroll.get();
        let at_bottom = shot_probe.moves.get();
        assert!(at_bottom > moves);
        SendMessageW(hwnd, WM_VSCROLL, Some(WPARAM(SB_BOTTOM.0 as usize)), None);
        SendMessageW(
            hwnd,
            WM_MOUSEWHEEL,
            Some(WPARAM(((-120_i16) as u16 as usize) << 16)),
            None,
        );
        assert_eq!(window.state.scroll.get(), bottom);
        assert_eq!(shot_probe.moves.get(), at_bottom);
        assert_eq!(shot_probe.messages.get(), 0);
        // Status alone does not run layout (which clamps scroll) or replace the observed data.
        window.state.scroll.set(i32::MAX);
        shared.borrow_mut().status = Some("updated status".into());
        window.update().unwrap();
        assert_eq!(&*window.state.status.borrow(), "updated status");
        assert_eq!(window.state.scroll.get(), i32::MAX);
        assert!(Arc::ptr_eq(&snapshot, &window.state.data.borrow()));
        shared
            .borrow_mut()
            .color(crate::stroke_style::Color(4, 5, 6));
        window.update().unwrap();
        assert!(!Arc::ptr_eq(&snapshot, &window.state.data.borrow()));
        assert_eq!(snapshot.colors[0].hex, "#010203");
        assert_eq!(window.state.data.borrow().colors[0].hex, "#040506");
        assert!(window.state.scroll.get() < i32::MAX);
        assert_eq!(caption_probe.messages.get(), 1);
        assert_eq!(caption(color), "#040506");
        drop((caption_probe, shot_probe, delete_probe));
        SendMessageW(hwnd, WM_CLOSE, None, None);
        assert!(!IsWindowVisible(hwnd).as_bool());
        assert!(window.handle().is_some());
        drop(window);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert!(IsWindow(Some(source.0)).as_bool());
    }
}
#[test]
fn scroll_is_bounded_and_destroyed_owner_cannot_receive_late_actions() {
    unsafe {
        let source = crate::desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                None,
                WS_OVERLAPPED,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let origin = Origin {
            unit: crate::workspace_id::UnitId::parse("fixture").unwrap(),
            lifetime: Default::default(),
        };
        let window = Window::new(source.0, origin, Default::default()).unwrap();
        let hwnd = window.handle().unwrap();
        window.state.scroll.set(i32::MAX);
        layout::resize(hwnd, &window.state).unwrap();
        assert!(window.state.scroll.get() < 5000);
        DestroyWindow(hwnd).unwrap();
        assert!(window.handle().is_none());
        assert!(window.state.view.borrow().action.is_none());
    }
}

#[test]
fn language_refresh_preserves_history_identity_scroll_and_existing_status() {
    use crate::ui_text::{format_for, text_for, DisplayLanguage};
    unsafe {
        let source = crate::desktop::Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                None,
                WS_OVERLAPPED,
                0,
                0,
                100,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let shared: Shared = Default::default();
        let image = crate::image_decode::Image {
            width: 2,
            height: 1,
            pixels: [0, 0, 255, 255].repeat(2).into(),
        };
        shared
            .borrow_mut()
            .screenshot(crate::capture_history::image::capture("id".into(), &image, 1).unwrap());
        shared.borrow_mut().status = Some("existing event snapshot".into());
        let origin = Origin {
            unit: crate::workspace_id::UnitId::parse("fixture").unwrap(),
            lifetime: Default::default(),
        };
        let window = Window::new(source.0, origin, shared.clone()).unwrap();
        let hwnd = window.handle().unwrap();
        let screenshot = GetDlgItem(Some(hwnd), 200).unwrap();
        let delete = GetDlgItem(Some(hwnd), 300).unwrap();
        let probe = TextProbe::new(screenshot);
        let snapshot = window.state.data.borrow().clone();
        let revision = window.revision;
        window.state.scroll.set(i32::MAX);
        for language in [
            DisplayLanguage::English,
            DisplayLanguage::SimplifiedChinese,
            DisplayLanguage::English,
        ] {
            window.refresh_language(language).unwrap();
            assert_eq!(window.handle(), Some(hwnd));
            assert_eq!(GetDlgItem(Some(hwnd), 200).unwrap(), screenshot);
            assert_eq!(caption(hwnd), text_for(language, "capture-history-title"));
            assert_eq!(
                caption(screenshot),
                format_for(
                    language,
                    "capture-history-image",
                    &[
                        ("index", 1.into()),
                        ("width", 2.into()),
                        ("height", 1.into())
                    ],
                )
            );
            assert_eq!(
                caption(delete),
                format_for(language, "capture-history-delete", &[("index", 1.into())])
            );
            assert_eq!(window.state.scroll.get(), i32::MAX);
            assert_eq!(window.revision, revision);
            assert_eq!(shared.borrow().revision, revision);
            assert!(Arc::ptr_eq(&snapshot, &window.state.data.borrow()));
            assert_eq!(&*window.state.status.borrow(), "existing event snapshot");
            assert!(shared.borrow().action.is_none());
            assert_eq!(probe.moves.get(), 0);
            let count = probe.messages.get();
            window.refresh_language(language).unwrap();
            assert_eq!(probe.messages.get(), count);
        }
    }
}

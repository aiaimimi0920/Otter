//! Wheel-message regressions with owned hidden HWNDs; change only thread key state, without global input injection or substituting for foreground manual tests.
use super::*;
use crate::desktop::{Apartment, Window};
use crate::editor::Tool;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

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
    window: Window,
    state: Box<State>,
    class: HSTRING,
    instance: HINSTANCE,
}
impl Fixture {
    unsafe fn new(pinned: bool) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let class = HSTRING::from(format!(
            "Otter.Native.PinWheel.{}",
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
        let state = Box::new(
            State::new(
                Some(crate::image_decode::Image {
                    width: 8,
                    height: 8,
                    pixels: [30, 60, 90, 255].repeat(64).into(),
                }),
                pinned,
            )
            .unwrap(),
        );
        let window = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(class.as_ptr()),
                w!("pin wheel fixture"),
                WS_POPUP,
                -400,
                -400,
                160,
                120,
                None,
                None,
                Some(instance),
                Some((&*state as *const State).cast()),
            )
            .unwrap(),
        );
        // Use the existing explicit CPU selection so real WM_PAINT regressions do not depend on the test machine's GPU.
        crate::desktop::render_commands::execute(
            window.0,
            &state,
            crate::desktop::render_commands::SOFTWARE,
        )
        .unwrap();
        Self {
            window,
            state,
            class,
            instance,
        }
    }
    unsafe fn wheel(&self, keys: &[VIRTUAL_KEY], delta: i16) {
        let _keys = Keys::held(keys);
        SendMessageW(
            self.window.0,
            WM_MOUSEWHEEL,
            Some(WPARAM((delta as u16 as usize) << 16)),
            Some(LPARAM(
                (-360i16 as u16 as isize) | ((-360i16 as u16 as isize) << 16),
            )),
        );
        animation::settle(self.window.0, &self.state);
    }
    unsafe fn rect(&self) -> RECT {
        let mut rect = RECT::default();
        GetWindowRect(self.window.0, &mut rect).unwrap();
        rect
    }
    fn opacity(&self, normal: f32, mini: f32, window: f32) {
        let canvas = self.state.canvas.borrow();
        for (actual, expected) in [
            (canvas.normal_opacity, normal),
            (canvas.minified_opacity, mini),
            (canvas.window_opacity, window),
        ] {
            assert!(
                (actual - expected).abs() < 0.00001,
                "{actual} != {expected}"
            );
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.window.0).unwrap();
            self.window.0 = HWND::default();
            UnregisterClassW(PCWSTR(self.class.as_ptr()), Some(self.instance)).unwrap();
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE).as_bool() {
                assert_eq!(message.wParam.0, 0, "unexpected ABI panic");
            }
        }
    }
}

#[test]
fn alt_wheel_changes_opacity_and_paints_without_resizing_or_editing_pixels() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new(true);
        let rect = fixture.rect();
        let stamp = fixture.state.canvas.borrow().editor.document_stamp();
        fixture.wheel(&[VK_MENU], -120);
        fixture.opacity(0.88, 0.9, 0.88);
        assert_eq!(fixture.rect(), rect);
        // Hidden HWNDs have no system update region; explicitly drive the original paint chain to verify opacity commits and unchanged pixels.
        SendMessageW(fixture.window.0, WM_PAINT, None, None);
        assert!(fixture.state.error.borrow().is_none());
        assert_eq!(fixture.state.canvas.borrow().confirmed_paints, 1);
        for tool in [Tool::Brush, Tool::Select, Tool::Text] {
            fixture.state.canvas.borrow_mut().select_tool(tool);
            fixture.wheel(&[VK_MENU], -30);
            fixture.opacity(0.85, 0.9, 0.85);
            fixture.wheel(&[VK_MENU], 30);
        }
        let canvas = fixture.state.canvas.borrow();
        assert!(stamp.matches(&canvas.editor));
        assert_eq!(
            &*canvas.image.as_ref().unwrap().pixels,
            [30, 60, 90, 255].repeat(64)
        );
    }
}

#[test]
fn alt_wheel_clamps_and_other_modifiers_keep_original_zoom_behavior() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new(true);
        fixture.wheel(&[VK_MENU], -30);
        fixture.opacity(0.97, 0.9, 0.97);
        for keys in [
            &[][..],
            &[VK_MENU, VK_SHIFT],
            &[VK_MENU, VK_LWIN],
            &[VK_MENU, VK_RWIN],
        ] {
            let rect = fixture.rect();
            fixture.wheel(keys, -120);
            fixture.opacity(0.97, 0.9, 0.97);
            assert_eq!(fixture.rect(), rect);
        }
        // Animation drives only visible hosts; this test always runs on a private desktop that is never switched to.
        let _ = ShowWindow(fixture.window.0, SW_SHOWNOACTIVATE);
        let rect = fixture.rect();
        fixture.wheel(&[VK_CONTROL], -120);
        assert_ne!(fixture.rect(), rect);
        fixture.opacity(0.97, 0.9, 0.97);
        fixture.wheel(&[VK_CONTROL, VK_MENU], -120);
        fixture.opacity(0.97, 0.9, 0.97);
        fixture.wheel(&[VK_MENU], i16::MIN);
        fixture.opacity(0.0, 0.9, 0.0);
        fixture.wheel(&[VK_MENU], i16::MAX);
        fixture.opacity(1.0, 0.9, 1.0);
        fixture.wheel(&[VK_MENU], 0);
        fixture.opacity(1.0, 0.9, 1.0);
    }
}

#[test]
fn minified_opacity_is_independent_and_survives_expand_collapse() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new(true);
        let stamp = fixture.state.canvas.borrow().editor.document_stamp();
        fixture.wheel(&[VK_MENU], -360);
        assert!(minify::collapse(fixture.window.0, &fixture.state, (-360, -360)).unwrap());
        fixture.opacity(0.64, 0.9, 0.9);
        fixture.wheel(&[VK_MENU], -120);
        fixture.opacity(0.64, 0.78, 0.78);
        assert!(restore_minified(fixture.window.0, &fixture.state).unwrap());
        fixture.opacity(0.64, 0.78, 0.64);
        assert!(minify::collapse(fixture.window.0, &fixture.state, (-360, -360)).unwrap());
        fixture.opacity(0.64, 0.78, 0.78);
        assert!(stamp.matches(&fixture.state.canvas.borrow().editor));
    }
}

#[test]
fn locked_or_drawing_pin_rejects_opacity_changes_without_discarding_draft() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new(true);
        fixture.state.group_locked.set(true);
        fixture.wheel(&[VK_MENU], -120);
        fixture.opacity(1.0, 0.9, 1.0);
        fixture.state.group_locked.set(false);
        fixture
            .state
            .canvas
            .borrow_mut()
            .editor
            .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
            .unwrap();
        fixture.wheel(&[VK_MENU], -120);
        fixture.opacity(1.0, 0.9, 1.0);
        assert!(fixture.state.canvas.borrow().editor.draft().is_some());
    }
}

#[test]
fn unpinned_workspace_keeps_viewport_zoom_instead_of_opacity() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new(false);
        let scale = fixture.state.canvas.borrow().viewport.scale;
        fixture.wheel(&[VK_MENU], -120);
        fixture.opacity(1.0, 0.9, 1.0);
        assert_ne!(fixture.state.canvas.borrow().viewport.scale, scale);
    }
}

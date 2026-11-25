//! Continuous wheel input must commit the current size without waiting for an idle message pump; use only owned HWNDs on an isolated desktop.
use super::*;
use crate::desktop::{Apartment, Window};
use crate::editor::Tool;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::{
    Graphics::Gdi::{GetUpdateRect, UpdateWindow, ValidateRect},
    System::{LibraryLoader::GetModuleHandleW, SystemServices::MK_CONTROL},
};

struct Fixture {
    window: Window,
    state: Box<State>,
    keys: [u8; 256],
    class: HSTRING,
    instance: HINSTANCE,
}
impl Fixture {
    unsafe fn new() -> Self {
        Self::with_renderer(Some(crate::desktop::render_commands::SOFTWARE))
    }
    unsafe fn with_renderer(command: Option<usize>) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let class = HSTRING::from(format!(
            "Otter.Native.ZoomFollow.{}",
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
                true,
            )
            .unwrap(),
        );
        let window = Window(
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                PCWSTR(class.as_ptr()),
                w!("zoom follow fixture"),
                WS_POPUP,
                100,
                100,
                160,
                120,
                None,
                None,
                Some(instance),
                Some((&*state as *const State).cast()),
            )
            .unwrap(),
        );
        let mut keys = [0; 256];
        GetKeyboardState(&mut keys).unwrap();
        let mut control = [0; 256];
        control[VK_CONTROL.0 as usize] = 0x80;
        SetKeyboardState(&control).unwrap();
        if let Some(command) = command {
            crate::desktop::render_commands::execute(window.0, &state, command).unwrap();
        }
        // This desktop is never foregrounded; only let the system generate real WM_PAINT for this test window.
        let _ = ShowWindow(window.0, SW_SHOWNOACTIVATE);
        let _ = InvalidateRect(Some(window.0), None, false);
        let _ = UpdateWindow(window.0);
        assert!(state.error.borrow().is_none());
        if state.canvas.borrow().software_selected() {
            assert!(state.canvas.borrow().confirmed_paints > 0);
        } else {
            // A private desktop returns DXGI_STATUS_OCCLUDED; this verifies only actual GPU drawing and message order.
            assert!(state.canvas.borrow().paints > 0);
        }
        Self {
            window,
            state,
            keys,
            class,
            instance,
        }
    }
    unsafe fn rect(&self) -> PinRect {
        let mut rect = RECT::default();
        GetWindowRect(self.window.0, &mut rect).unwrap();
        PinRect {
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        }
    }
    unsafe fn wheel(&self, delta: i16) {
        self.wheel_at(delta, (140, 140));
    }
    unsafe fn wheel_at(&self, delta: i16, pointer: (i32, i32)) {
        SendMessageW(
            self.window.0,
            WM_MOUSEWHEEL,
            Some(WPARAM(
                ((delta as u16 as usize) << 16) | MK_CONTROL.0 as usize,
            )),
            Some(LPARAM(
                (pointer.0 as u16 as isize) | ((pointer.1 as u16 as isize) << 16),
            )),
        );
        animation::settle(self.window.0, &self.state);
    }
}

#[path = "pin_zoom_animation_tests.rs"]
mod animation_tests;
#[path = "pin_minified_present_tests.rs"]
mod minified_tests;
#[path = "pin_move_preview_tests.rs"]
mod movement_tests;
#[path = "pin_zoom_present_tests.rs"]
mod presentation_tests;
#[path = "pin_zoom_preview_tests.rs"]
mod preview_tests;
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            SetKeyboardState(&self.keys).unwrap();
            if self.state.lifetime.owns(self.window.0) {
                DestroyWindow(self.window.0).unwrap();
            }
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
fn ctrl_wheel_presents_each_current_size_without_an_idle_message_pump() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        let stamp = fixture.state.canvas.borrow().editor.document_stamp();
        let source = fixture
            .state
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .clone();
        // Do not pump WM_PAINT between wheel events; cover both directions, consecutive full notches, and high-resolution small steps.
        let original = fixture.rect();
        let mut steps = 0.0f64;
        for delta in [
            120, 120, -120, 30, 30, 30, 30, -30, -30, -30, -30, 240, -240, -120,
        ] {
            steps += f64::from(delta) / 120.0;
            let factor = 1.1_f64.powf(steps);
            let expected = PinRect {
                x: (140.0 - f64::from(140 - original.x) * factor).round() as i32,
                y: (140.0 - f64::from(140 - original.y) * factor).round() as i32,
                width: (f64::from(original.width) * factor).round() as u32,
                height: (f64::from(original.height) * factor).round() as u32,
            };
            let paints = fixture.state.canvas.borrow().confirmed_paints;
            fixture.wheel(delta);
            assert_eq!(fixture.rect(), expected);
            assert!(fixture.state.error.borrow().is_none());
            assert!(
                fixture.state.canvas.borrow().confirmed_paints > paints,
                "each changed wheel size must be presented before the next input"
            );
            assert!(
                !GetUpdateRect(fixture.window.0, None, false).as_bool(),
                "wheel paint must not wait for the idle queue"
            );
            assert!(stamp.matches(&fixture.state.canvas.borrow().editor));
            assert!(
                source.same_storage(&fixture.state.canvas.borrow().image.as_ref().unwrap().pixels)
            );
        }
    }
}

#[test]
fn opposite_wheel_steps_restore_the_exact_rectangle_without_cumulative_drift() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        SetWindowPos(
            fixture.window.0,
            None,
            100,
            100,
            161,
            121,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let original = fixture.rect();
        for _ in 0..40 {
            fixture.wheel(120);
            fixture.wheel(-120);
            assert_eq!(
                fixture.rect(),
                original,
                "inverse wheel steps must not shrink or move the pin"
            );
        }
    }
}

#[test]
fn zero_delta_lock_and_drawing_do_not_force_zoom_or_discard_the_draft() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let fixture = Fixture::new();
        assert!(ValidateRect(Some(fixture.window.0), None).as_bool());
        let rect = fixture.rect();
        let paints = fixture.state.canvas.borrow().confirmed_paints;
        fixture.wheel(0);
        fixture.state.group_locked.set(true);
        fixture.wheel(120);
        fixture.state.group_locked.set(false);
        fixture
            .state
            .canvas
            .borrow_mut()
            .editor
            .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
            .unwrap();
        fixture.wheel(120);
        assert_eq!(fixture.rect(), rect);
        assert_eq!(fixture.state.canvas.borrow().confirmed_paints, paints);
        assert!(fixture.state.canvas.borrow().editor.draft().is_some());
        assert!(!GetUpdateRect(fixture.window.0, None, false).as_bool());
    }
}

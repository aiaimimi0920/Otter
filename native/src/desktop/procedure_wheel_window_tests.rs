//! 自建宿主的真实 WM_MOUSEWHEEL 路由；只改当前测试线程键态，不注入全局输入。
use super::*;
use crate::editor::Tool;
use desktop::Window;
#[path = "resize_wheel_window_tests.rs"]
mod queued;
use windows::{
    core::w,
    Win32::{System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};

struct Keys([u8; 256]);
impl Keys {
    unsafe fn held(held: &[VIRTUAL_KEY]) -> Self {
        let mut old = [0; 256];
        GetKeyboardState(&mut old).unwrap();
        let mut next = [0; 256];
        for key in held {
            next[key.0 as usize] = 0x80;
        }
        SetKeyboardState(&next).unwrap();
        Self(old)
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
}
impl Fixture {
    unsafe fn new() -> Self {
        let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let class = w!("Otter.Annotation.Wheel.TestHost");
        let registered = RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: class,
            lpfnWndProc: Some(desktop::procedure::window_proc),
            ..Default::default()
        });
        assert!(registered != 0 || GetLastError() == ERROR_CLASS_ALREADY_EXISTS);
        let state = Box::new(
            State::new(
                Some(crate::image_decode::Image {
                    width: 100,
                    height: 100,
                    pixels: vec![255; 40000].into(),
                }),
                true,
            )
            .unwrap(),
        );
        {
            let mut c = state.canvas.borrow_mut();
            for x in [10.0, 50.0] {
                c.editor
                    .begin(Tool::Rectangle, Point { x, y: 10.0 })
                    .unwrap();
                c.editor
                    .extend(Point {
                        x: x + 20.0,
                        y: 30.0,
                    })
                    .unwrap();
                assert!(c.editor.commit());
            }
            c.select_tool(Tool::Select);
            let revision = c.editor.revision();
            c.selected = Some((0, revision));
            c.multi_selection = Some((vec![0, 1], revision));
        }
        let window = Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class,
                w!("wheel fixture"),
                WS_POPUP,
                100,
                100,
                200,
                200,
                None,
                None,
                Some(instance),
                Some((&*state as *const State).cast()),
            )
            .unwrap(),
        );
        desktop::render_commands::execute(window.0, &state, desktop::render_commands::SOFTWARE)
            .unwrap();
        Self { window, state }
    }
    unsafe fn rect(&self) -> RECT {
        let mut rect = RECT::default();
        GetWindowRect(self.window.0, &mut rect).unwrap();
        rect
    }
    unsafe fn wheel(&self, held: &[VIRTUAL_KEY], delta: i16) {
        let _keys = Keys::held(held);
        SendMessageW(
            self.window.0,
            WM_MOUSEWHEEL,
            Some(WPARAM(((delta as u16 as usize) << 16) | 0x8)),
            Some(LPARAM(150 | (150 << 16))),
        );
    }
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.002, "{actual} != {expected}");
}

#[test]
fn ctrl_alt_wheel_changes_only_selected_annotations_and_preserves_selection_history_and_pin() {
    unsafe {
        for own in [false, true] {
            let f = Fixture::new();
            let before = f.rect();
            let view = f.state.canvas.borrow().viewport;
            let revision = f.state.canvas.borrow().editor.revision();
            f.state.canvas.borrow_mut().normal_opacity = 0.5;
            f.state.canvas.borrow_mut().window_opacity = 0.5;
            let held = if own {
                vec![VK_CONTROL, VK_MENU, VK_SHIFT]
            } else {
                vec![VK_CONTROL, VK_MENU]
            };
            f.wheel(&held, 120);
            let c = f.state.canvas.borrow();
            assert_eq!(c.editor.revision(), revision + 1);
            assert_eq!(c.selected_indices(), vec![0, 1]);
            assert_eq!(c.viewport, view);
            assert_eq!(c.window_opacity, 0.5);
            assert_eq!(c.normal_opacity, 0.5);
            assert!(c.selection_enabled);
            close(
                (c.editor.elements()[0].points[0].x + c.editor.elements()[0].points[1].x) / 2.0,
                if own {
                    20.0
                } else {
                    40.0 - 20.0 * 0.12f32.exp()
                },
            );
            drop(c);
            assert_eq!(f.rect(), before);
            assert!(!desktop::pin_window::animation::busy(&f.state));
            assert!(!desktop::pin_window::preview::active(&f.state));
            let mut c = f.state.canvas.borrow_mut();
            assert!(c.editor.undo());
            assert_eq!(c.editor.elements()[0].points[0], Point { x: 10.0, y: 10.0 });
            assert!(c.editor.redo());
            assert!(f.state.error.borrow().is_none());
        }
    }
}

#[test]
fn exact_modifiers_and_busy_or_empty_edit_states_do_not_fall_through_to_pin_zoom() {
    unsafe {
        for case in 0..8 {
            let f = Fixture::new();
            match case {
                0 => {
                    let mut c = f.state.canvas.borrow_mut();
                    c.selected = None;
                    c.multi_selection = None;
                }
                1 => f.state.group_locked.set(true),
                2 => f.state.file_dialog.set(true),
                3 => f.state.capture_active.set(true),
                4 => f.state.group_hidden.set(true),
                5 => desktop::edit_mode::execute(f.window.0, &f.state, false).unwrap(),
                _ => (),
            }
            let before = f.rect();
            let stamp = f.state.canvas.borrow().editor.document_stamp();
            let held = match case {
                6 => vec![VK_CONTROL, VK_SHIFT],
                7 => vec![VK_CONTROL, VK_MENU, VK_LWIN],
                _ => vec![VK_CONTROL, VK_MENU],
            };
            f.wheel(&held, 120);
            assert!(stamp.matches(&f.state.canvas.borrow().editor));
            assert_eq!(f.rect(), before);
            assert!(!desktop::pin_window::preview::active(&f.state));
        }
        let f = Fixture::new();
        let before = f.rect();
        let stamp = f.state.canvas.borrow().editor.document_stamp();
        f.wheel(&[VK_MENU], -120);
        close(f.state.canvas.borrow().window_opacity, 0.88);
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
        assert_eq!(f.rect(), before);
        f.wheel(&[VK_CONTROL], 120);
        assert!(f.rect().right - f.rect().left > before.right - before.left);
        assert!(stamp.matches(&f.state.canvas.borrow().editor));
    }
}

#[test]
fn blur_wheel_reuses_original_worker_and_commits_effect_and_geometry_atomically() {
    unsafe {
        let f = Fixture::new();
        let before = f.rect();
        {
            let mut c = f.state.canvas.borrow_mut();
            let pixels = c.image.as_ref().unwrap().pixels.clone();
            c.editor
                .begin_blur(
                    Point { x: 30.0, y: 30.0 },
                    crate::blur_image::BlurImage::new(100, 100, pixels, 4).unwrap(),
                )
                .unwrap();
            assert!(c.editor.commit());
            c.selected = Some((2, c.editor.revision()));
            c.multi_selection = None;
        }
        let revision = f.state.canvas.borrow().editor.revision();
        f.wheel(&[VK_CONTROL, VK_MENU], 120);
        assert!(f.state.resize_job.borrow().is_busy());
        assert_eq!(f.state.canvas.borrow().editor.revision(), revision);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while f.state.resize_job.borrow().is_busy() {
            assert!(std::time::Instant::now() < deadline);
            desktop::resize_input::poll(f.window.0, &f.state);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let mut c = f.state.canvas.borrow_mut();
        assert_eq!(c.editor.revision(), revision + 1);
        assert_eq!(c.selected_index(), Some(2));
        close(
            c.editor.elements()[2].blur.as_ref().unwrap().strength(),
            4.0 * 0.12f32.exp(),
        );
        assert!(c.editor.undo());
        close(
            c.editor.elements()[2].blur.as_ref().unwrap().strength(),
            4.0,
        );
        assert_eq!(f.rect(), before);
        assert!(f.state.error.borrow().is_none());
    }
}

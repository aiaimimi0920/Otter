//! 真实 Canvas/生产 WM_PAINT 软件路径；内部显式选择 CPU，不伪造当前机器硬件不可用。
use super::*;
use crate::desktop::{Apartment, State, Window};
use windows::{
    core::{w, Error},
    Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};

struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Canvas.Software.Test"), Some(self.0));
            let mut message = MSG::default();
            // 仅消费本测试线程最后宿主关闭产生的 WM_QUIT，不把它留给后续测试。
            while PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE).as_bool() {
                assert_eq!(message.wParam.0, 0, "unexpected ABI panic");
            }
        }
    }
}
pub(super) fn fixture(test: impl FnOnce(&State, HWND)) {
    fixture_mode(false, test);
}
pub(super) fn fixture_mode(pinned: bool, test: impl FnOnce(&State, HWND)) {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Canvas.Software.Test"),
                lpfnWndProc: Some(crate::desktop::procedure::window_proc),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let state = State::new(
            Some(Image {
                width: 8,
                height: 8,
                pixels: [30, 60, 90, 255].repeat(64).into(),
            }),
            pinned,
        )
        .unwrap();
        state.canvas.borrow_mut().software_renderer = true;
        let window = Window(
            CreateWindowExW(
                if pinned {
                    WS_EX_NOREDIRECTIONBITMAP
                } else {
                    WINDOW_EX_STYLE::default()
                },
                w!("Otter.Canvas.Software.Test"),
                w!(""),
                if pinned {
                    WS_POPUP
                } else {
                    WS_OVERLAPPEDWINDOW
                },
                0,
                0,
                160,
                120,
                None,
                None,
                Some(instance),
                Some((&state as *const State).cast()),
            )
            .unwrap(),
        );
        test(&state, window.0);
    }
}

#[test]
#[ignore = "may retain a real GPU graph after injected brush failure; run in an isolated process"]
fn gpu_brush_preparation_failure_blocks_canvas_without_publishing_target() {
    fixture(|state, hwnd| {
        let mut canvas = state.canvas.borrow_mut();
        let stamp = canvas.editor.document_stamp();
        let surface = RenderSurface::new(
            &canvas.factory,
            hwnd,
            D2D_SIZE_U {
                width: 160,
                height: 120,
            },
            96,
            true,
        )
        .unwrap();
        assert!(matches!(
            surface,
            RenderSurface::D3d12(_) | RenderSurface::D3d11(_)
        ));
        let error = canvas
            .creation_admission
            .attempt(|| {
                Target::prepare(surface, |_| {
                    Err(Error::new(E_FAIL, "brush preparation fixture"))
                })
            })
            .err()
            .unwrap();
        assert_eq!(error.code(), E_FAIL);
        assert!(error.message().contains("brush preparation fixture"));
        assert!(canvas.creation_blocked());
        assert!(canvas.target.is_none());
        canvas.discard_device();
        assert!(canvas.paint(hwnd, 160, 120, 96).is_err());
        assert!(canvas.target.is_none());
        assert!(stamp.matches(&canvas.editor));
        assert_eq!(canvas.paints, 0);
        assert_eq!(canvas.confirmed_paints, 0);
    });
}

#[test]
fn software_brush_preparation_failure_does_not_claim_unconfirmed_composition() {
    fixture(|state, _| {
        let mut canvas = state.canvas.borrow_mut();
        let surface = RenderSurface::software(
            &canvas.factory,
            D2D_SIZE_U {
                width: 16,
                height: 16,
            },
        )
        .unwrap();
        let error = canvas
            .creation_admission
            .attempt(|| {
                Target::prepare(surface, |_| {
                    Err(Error::new(E_FAIL, "software brush fixture"))
                })
            })
            .err()
            .unwrap();
        assert_eq!(error.code(), E_FAIL);
        assert!(!canvas.creation_blocked());
        assert!(canvas.target.is_none());
    });
}

#[test]
fn blocked_creation_survives_discard_and_repeated_window_paint_without_close() {
    fixture(|state, hwnd| unsafe {
        let stamp = state.canvas.borrow().editor.document_stamp();
        let error = state
            .canvas
            .borrow_mut()
            .creation_admission
            .attempt::<()>(|| {
                Err(crate::desktop::composition_creation::Failure::attached(
                    Error::new(E_FAIL, "blocked fixture"),
                ))
            })
            .unwrap_err();
        *state.error.borrow_mut() = Some(error);
        state.canvas.borrow_mut().discard_device();
        for _ in 0..3 {
            SendMessageW(hwnd, WM_PAINT, None, None);
            let mut message = MSG::default();
            assert!(
                !PeekMessageW(&mut message, Some(hwnd), WM_CLOSE, WM_CLOSE, PM_NOREMOVE).as_bool()
            );
            assert!(IsWindow(Some(hwnd)).as_bool());
            let canvas = state.canvas.borrow();
            assert!(canvas.target.is_none());
            assert!(canvas.creation_blocked());
            assert_eq!(canvas.paints, 0);
            assert_eq!(canvas.confirmed_paints, 0);
            assert!(stamp.matches(&canvas.editor));
            assert!(state
                .error
                .borrow()
                .as_ref()
                .unwrap()
                .message()
                .contains("blocked fixture"));
        }
    });
}

#[test]
fn workspace_software_canvas_defers_confirmation_and_stays_selected() {
    fixture(|state, hwnd| {
        let stamp = state.canvas.borrow().editor.document_stamp();
        for (width, height, dpi) in [(160, 120, 96), (192, 144, 144), (160, 120, 96)] {
            let before = state.canvas.borrow().confirmed_paints;
            let pending = {
                let mut canvas = state.canvas.borrow_mut();
                canvas.paint(hwnd, width, height, dpi).unwrap();
                assert!(canvas.target.as_ref().unwrap().surface.is_software());
                assert_eq!(canvas.confirmed_paints, before);
                canvas.take_submission(true).unwrap()
            };
            let presented = pending.present(hwnd).unwrap();
            assert!(state.canvas.borrow_mut().confirm_submission(presented));
            assert_eq!(state.canvas.borrow().confirmed_paints, before + 1);
            state.canvas.borrow_mut().discard_device();
        }
        assert!(stamp.matches(&state.canvas.borrow().editor));
    });
}

#[test]
fn stale_or_failed_software_submission_never_confirms_a_new_target() {
    fixture(|state, hwnd| {
        let old = {
            let mut canvas = state.canvas.borrow_mut();
            canvas.paint(hwnd, 160, 120, 96).unwrap();
            canvas.take_submission(true).unwrap()
        };
        let presented = old.present(hwnd).unwrap();
        state.canvas.borrow_mut().discard_device();
        assert!(!state.canvas.borrow_mut().confirm_submission(presented));
        let failed = {
            let mut canvas = state.canvas.borrow_mut();
            canvas.paint(hwnd, 160, 120, 96).unwrap();
            canvas.take_submission(true).unwrap()
        };
        assert!(failed.present(HWND::default()).is_err());
        assert_eq!(state.canvas.borrow().confirmed_paints, 0);
        let pending = {
            let mut canvas = state.canvas.borrow_mut();
            canvas.paint(hwnd, 160, 120, 96).unwrap();
            canvas.take_submission(true).unwrap()
        };
        let presented = pending.present(hwnd).unwrap();
        let mut canvas = state.canvas.borrow_mut();
        let target = std::rc::Rc::clone(&canvas.target.as_ref().unwrap().identity);
        canvas.editor = Editor::default(); // 只改变文档身份，保留同一 Target。
        assert!(std::rc::Rc::ptr_eq(
            &target,
            &canvas.target.as_ref().unwrap().identity
        ));
        assert!(!canvas.confirm_submission(presented));
        assert_eq!(canvas.confirmed_paints, 0);
    });
}

#[test]
fn production_paint_submits_cpu_and_nested_guard_keeps_error_and_counts() {
    fixture(|state, hwnd| unsafe {
        let marker = Error::new(E_FAIL, "deferred paint must not clear this marker");
        *state.error.borrow_mut() = Some(marker);
        state.render_active.set(true);
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.render_deferred.get());
        assert!(state.error.borrow().is_some());
        assert_eq!(state.canvas.borrow().confirmed_paints, 0);
        state.render_active.set(false);
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(!state.render_active.get());
        assert!(!state.render_deferred.get());
        assert!(state.error.borrow().is_none());
        let canvas = state.canvas.borrow();
        assert!(canvas.target.as_ref().unwrap().surface.is_software());
        assert_eq!(canvas.confirmed_paints, 1);
        assert_eq!(canvas.paints, 1);
        assert!(!IsWindowVisible(hwnd).as_bool());
    });
}

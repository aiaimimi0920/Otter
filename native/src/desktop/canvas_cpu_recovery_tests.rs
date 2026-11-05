//! 真实 D3D11→CPU 与透明钉图生产提交；故障来自无匹配 Pop 的 EndDraw，不伪造驱动移除。
use super::*;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
use windows::{
    core::Error,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

unsafe fn settle(state: &crate::desktop::State, hwnd: HWND) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while state.canvas.borrow().render_suspended() {
        assert!(Instant::now() < deadline, "retirement did not complete");
        let mut message = MSG::default();
        while PeekMessageW(&mut message, Some(hwnd), WM_TIMER, WM_TIMER, PM_REMOVE).as_bool() {
            DispatchMessageW(&message);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn pin_d3d11_failure_retires_then_presents_cpu_with_opacity_and_sticky_selection() {
    for direct_compatibility in [false, true] {
        software_tests::fixture_mode(true, |state, hwnd| unsafe {
            {
                let mut canvas = state.canvas.borrow_mut();
                canvas.software_renderer = false;
                canvas.compatibility_renderer = direct_compatibility;
                canvas.window_opacity = 0.5;
            }
            SendMessageW(hwnd, WM_PAINT, None, None);
            if !direct_compatibility {
                let mut canvas = state.canvas.borrow_mut();
                assert!(matches!(
                    canvas.target.as_ref().unwrap().surface,
                    RenderSurface::D3d12(_)
                ));
                assert!(canvas
                    .target
                    .as_mut()
                    .unwrap()
                    .surface
                    .draw(|target| {
                        target.PopAxisAlignedClip();
                        Ok(())
                    })
                    .is_err());
                drop(canvas);
                SendMessageW(hwnd, WM_PAINT, None, None);
                settle(state, hwnd);
                SendMessageW(hwnd, WM_PAINT, None, None);
            }
            let mut canvas = state.canvas.borrow_mut();
            assert!(matches!(
                canvas.target.as_ref().unwrap().surface,
                RenderSurface::D3d11(_)
            ));
            let stamp = canvas.editor.document_stamp();
            let count = canvas.confirmed_paints;
            assert!(canvas
                .target
                .as_mut()
                .unwrap()
                .surface
                .draw(|_| Err(Error::new(E_FAIL, "business")))
                .is_err());
            assert!(!canvas.recover_render());
            assert!(canvas
                .target
                .as_mut()
                .unwrap()
                .surface
                .draw(|target| {
                    target.PopAxisAlignedClip();
                    Ok(())
                })
                .is_err());
            drop(canvas);
            SendMessageW(hwnd, WM_PAINT, None, None);
            assert!(state.canvas.borrow().render_suspended());
            assert_eq!(
                GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_LAYERED.0,
                0
            );
            settle(state, hwnd);
            assert!(state.canvas.borrow().software_renderer);
            SendMessageW(hwnd, WM_PAINT, None, None);
            assert!(state.error.borrow().is_none());
            assert_eq!(state.canvas.borrow().confirmed_paints, count + 1);
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            assert_ne!(style & WS_EX_LAYERED.0, 0);
            assert_ne!(style & WS_EX_NOREDIRECTIONBITMAP.0, 0);
            assert!(stamp.matches(&state.canvas.borrow().editor));
            state.canvas.borrow_mut().discard_device();
            SendMessageW(hwnd, WM_PAINT, None, None);
            assert!(state
                .canvas
                .borrow()
                .target
                .as_ref()
                .unwrap()
                .surface
                .is_software());
            assert_eq!(state.canvas.borrow().confirmed_paints, count + 2);
        });
    }
}

#[test]
fn workspace_d3d11_failure_requires_real_retirement_before_cpu() {
    software_tests::fixture(|state, hwnd| unsafe {
        state.canvas.borrow_mut().software_renderer = false;
        state.canvas.borrow_mut().compatibility_renderer = true;
        SendMessageW(hwnd, WM_PAINT, None, None);
        let mut canvas = state.canvas.borrow_mut();
        assert!(matches!(
            canvas.target.as_ref().unwrap().surface,
            RenderSurface::D3d11(_)
        ));
        let stamp = canvas.editor.document_stamp();
        let count = canvas.confirmed_paints;
        // 错误码不能授权跳过 DComp。实际 EndDraw 失效才进入有界退休，不冒充驱动移除。
        assert!(!canvas.recover_render_after(D2DERR_RECREATE_TARGET));
        assert!(canvas
            .target
            .as_mut()
            .unwrap()
            .surface
            .draw(|target| {
                target.PopAxisAlignedClip();
                Ok(())
            })
            .is_err());
        drop(canvas);
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.canvas.borrow().render_suspended());
        assert!(!state.canvas.borrow().can_request_render());
        settle(state, hwnd);
        assert!(state.canvas.borrow().software_renderer);
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.error.borrow().is_none());
        assert!(state
            .canvas
            .borrow()
            .target
            .as_ref()
            .unwrap()
            .surface
            .is_software());
        assert!(stamp.matches(&state.canvas.borrow().editor));
        assert_eq!(state.canvas.borrow().confirmed_paints, count + 1);
    });
}

#[test]
fn initial_software_pin_presents_without_a_gpu_or_style_preparation_by_caller() {
    software_tests::fixture_mode(true, |state, hwnd| unsafe {
        state.canvas.borrow_mut().window_opacity = 0.25;
        let stamp = state.canvas.borrow().editor.document_stamp();
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.error.borrow().is_none());
        assert!(state
            .canvas
            .borrow()
            .target
            .as_ref()
            .unwrap()
            .surface
            .is_software());
        assert_eq!(state.canvas.borrow().confirmed_paints, 1);
        assert!(stamp.matches(&state.canvas.borrow().editor));
        assert_ne!(
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_LAYERED.0,
            0
        );
    });
}

thread_local! {
    static REENTER: Cell<u8> = const { Cell::new(0) };
    static BORROW_FAILED: Cell<bool> = const { Cell::new(false) };
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_STYLECHANGING {
        let mode = REENTER.replace(0);
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const crate::desktop::State;
        if !pointer.is_null() && mode != 0 {
            if mode == 2 {
                let _ = DestroyWindow(hwnd);
            } else {
                match (*pointer).canvas.try_borrow_mut() {
                    Ok(mut canvas) => {
                        if canvas
                            .replace_image(Image {
                                width: 2,
                                height: 2,
                                pixels: vec![255; 16].into(),
                            })
                            .is_err()
                        {
                            BORROW_FAILED.set(true);
                        }
                    }
                    Err(_) => BORROW_FAILED.set(true),
                }
                SendMessageW(hwnd, WM_PAINT, None, None);
            }
        }
    }
    crate::desktop::procedure::window_proc(hwnd, msg, wp, lp)
}

#[test]
fn style_reentry_rejects_old_document_frame_and_allows_next_software_paint() {
    software_tests::fixture_mode(true, |state, hwnd| unsafe {
        BORROW_FAILED.set(false);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
        REENTER.set(1);
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert_eq!(REENTER.get(), 0);
        assert!(!BORROW_FAILED.get());
        assert_eq!(state.canvas.borrow().confirmed_paints, 0);
        assert_eq!(
            GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_LAYERED.0,
            0
        );
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.error.borrow().is_none());
        assert_eq!(state.canvas.borrow().confirmed_paints, 1);
    });
}

#[test]
fn style_reentry_destroy_revokes_transaction_and_never_confirms_a_frame() {
    software_tests::fixture_mode(true, |state, hwnd| unsafe {
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
        REENTER.set(2);
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert_eq!(REENTER.get(), 0);
        assert!(!state.lifetime.owns(hwnd));
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert_eq!(state.canvas.borrow().confirmed_paints, 0);
    });
}

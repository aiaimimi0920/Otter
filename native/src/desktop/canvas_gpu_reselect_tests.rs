//! 真实生产菜单/WM_PAINT 的 GPU 回升和跨后端选择；无输入注入，隐藏 HWND 保文档。
use super::*;
use crate::desktop::{context_menu, render_commands, State};
use std::cell::Cell;
use windows::{
    core::Error,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

unsafe fn finish(state: &State, hwnd: HWND) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while state.canvas.borrow().render_suspended() {
        assert!(std::time::Instant::now() < deadline);
        let mut message = MSG::default();
        while PeekMessageW(&mut message, Some(hwnd), WM_TIMER, WM_TIMER, PM_REMOVE).as_bool() {
            DispatchMessageW(&message);
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    SendMessageW(hwnd, WM_PAINT, None, None);
}

#[test]
fn cpu_gpu_round_trips_keep_document_opacity_and_actual_menu_state() {
    for pinned in [false, true] {
        software_tests::fixture_mode(pinned, |state, hwnd| unsafe {
            state.group_locked.set(true);
            if pinned {
                state.canvas.borrow_mut().window_opacity = 0.5;
            }
            SendMessageW(hwnd, WM_PAINT, None, None);
            let stamp = state.canvas.borrow().editor.document_stamp();
            let original = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            for command in [
                render_commands::AUTOMATIC,
                render_commands::COMPATIBILITY,
                render_commands::AUTOMATIC,
                render_commands::SOFTWARE,
                render_commands::COMPATIBILITY,
                render_commands::SOFTWARE,
            ] {
                let before = state.canvas.borrow().confirmed_paints;
                *state.error.borrow_mut() = Some(Error::new(E_FAIL, "keep until new frame"));
                context_menu::execute(hwnd, state, command).unwrap();
                assert!(state.error.borrow().is_some());
                finish(state, hwnd);
                assert!(state.error.borrow().is_none());
                let canvas = state.canvas.borrow();
                assert!(stamp.matches(&canvas.editor));
                assert_eq!(canvas.confirmed_paints, before + 1);
                let surface = &canvas.target.as_ref().unwrap().surface;
                match command {
                    render_commands::AUTOMATIC => {
                        assert!(matches!(surface, RenderSurface::D3d12(_)))
                    }
                    render_commands::COMPATIBILITY => {
                        assert!(matches!(surface, RenderSurface::D3d11(_)))
                    }
                    _ => assert!(surface.is_software()),
                }
                assert_eq!(canvas.window_opacity, if pinned { 0.5 } else { 1.0 });
                drop(canvas);
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
                assert_eq!(
                    style & WS_EX_NOREDIRECTIONBITMAP.0,
                    original & WS_EX_NOREDIRECTIONBITMAP.0
                );
                assert_eq!(
                    style & WS_EX_LAYERED.0,
                    if pinned && command == render_commands::SOFTWARE {
                        WS_EX_LAYERED.0
                    } else {
                        0
                    }
                );
                let menu = CreatePopupMenu().unwrap();
                render_commands::append(menu, state).unwrap();
                for item in [
                    render_commands::AUTOMATIC,
                    render_commands::COMPATIBILITY,
                    render_commands::SOFTWARE,
                ] {
                    assert_eq!(
                        GetMenuState(menu, item as u32, MF_BYCOMMAND) & MF_CHECKED.0 != 0,
                        item == command
                    );
                }
                DestroyMenu(menu).unwrap();
                assert!(!IsWindowVisible(hwnd).as_bool());
            }
        });
    }
}

#[test]
fn gpu_reselect_rejects_active_paint_and_borrows_and_retries_known_cpu_failure() {
    software_tests::fixture_mode(true, |state, hwnd| unsafe {
        SendMessageW(hwnd, WM_PAINT, None, None);
        for command in [render_commands::AUTOMATIC, render_commands::COMPATIBILITY] {
            state.render_active.set(true);
            assert!(context_menu::execute(hwnd, state, command).is_err());
            state.render_active.set(false);
            {
                let _borrow = state.canvas.borrow();
                assert!(context_menu::execute(hwnd, state, command).is_err());
            }
            {
                let _borrow = state.canvas.borrow_mut();
                assert!(context_menu::execute(hwnd, state, command).is_err());
            }
        }
        let stamp = state.canvas.borrow().editor.document_stamp();
        state.canvas.borrow_mut().suspend_render();
        context_menu::execute(hwnd, state, render_commands::AUTOMATIC).unwrap();
        finish(state, hwnd);
        assert!(stamp.matches(&state.canvas.borrow().editor));
        assert!(matches!(
            state.canvas.borrow().target.as_ref().unwrap().surface,
            RenderSurface::D3d12(_)
        ));
        DestroyWindow(hwnd).unwrap();
        assert!(render_commands::execute(hwnd, state, render_commands::AUTOMATIC).is_err());
    });
}

thread_local! {
    static REENTER: Cell<u8> = const { Cell::new(0) };
    static SAFE: Cell<bool> = const { Cell::new(false) };
}

#[test]
fn failed_gpu_candidate_without_association_can_return_to_cpu_but_attached_cannot() {
    for attached in [false, true] {
        software_tests::fixture_mode(true, |state, hwnd| unsafe {
            SendMessageW(hwnd, WM_PAINT, None, None);
            let stamp = state.canvas.borrow().editor.document_stamp();
            context_menu::execute(hwnd, state, render_commands::AUTOMATIC).unwrap();
            // 复用类型化创建凭证注入；不伪造驱动故障或未知 DComp 的完成回执。
            crate::desktop::software_surface::window_style::Transition::gpu_guarded(hwnd, &|| {
                state.lifetime.owns(hwnd)
            })
            .unwrap()
            .commit()
            .unwrap();
            {
                let mut canvas = state.canvas.borrow_mut();
                let _ = canvas.creation_admission.attempt::<()>(|| {
                    let error = Error::new(E_FAIL, "candidate failure fixture");
                    Err(if attached {
                        crate::desktop::composition_creation::Failure::attached(error)
                    } else {
                        error.into()
                    })
                });
                canvas.suspend_render();
            }
            assert_eq!(
                GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_LAYERED.0,
                0
            );
            let result = context_menu::execute(hwnd, state, render_commands::SOFTWARE);
            if attached {
                assert!(result.is_err());
                assert!(state.canvas.borrow().creation_blocked());
            } else {
                result.unwrap();
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
                assert!(!state.canvas.borrow().creation_admission.retryable_failure());
            }
            assert!(stamp.matches(&state.canvas.borrow().editor));
        });
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_STYLECHANGING {
        let mode = if REENTER.get() < 3 {
            REENTER.replace(0)
        } else {
            0
        };
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
        if mode != 0 && !pointer.is_null() {
            let state = &*pointer;
            SAFE.set(render_commands::execute(hwnd, state, render_commands::SOFTWARE).is_err());
            if mode == 2 {
                let _ = DestroyWindow(hwnd);
            } else {
                match state.canvas.try_borrow_mut() {
                    Ok(mut canvas) => SAFE.set(
                        SAFE.get()
                            && canvas
                                .replace_image(Image {
                                    width: 2,
                                    height: 2,
                                    pixels: vec![255; 16].into(),
                                })
                                .is_ok(),
                    ),
                    Err(_) => SAFE.set(false),
                }
                SendMessageW(hwnd, WM_PAINT, None, None);
            }
        }
    }
    if msg == WM_WINDOWPOSCHANGING
        && ((matches!(REENTER.get(), 3 | 4)
            && (*(lp.0 as *const WINDOWPOS)).flags.contains(SWP_HIDEWINDOW))
            || (matches!(REENTER.get(), 5 | 6)
                && (*(lp.0 as *const WINDOWPOS)).flags.contains(SWP_SHOWWINDOW)))
    {
        let mode = REENTER.replace(0);
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
        if !pointer.is_null() {
            let state = &*pointer;
            SAFE.set(
                state.canvas.try_borrow_mut().is_ok()
                    && render_commands::execute(hwnd, state, render_commands::SOFTWARE).is_err(),
            );
            if matches!(mode, 3 | 6) {
                let _ = DestroyWindow(hwnd);
            } else {
                state.group_hidden.set(true);
                let _ = ShowWindow(hwnd, SW_HIDE); // 真实隐藏调用也不能被外层 SHOW 覆盖。
            }
        }
    }
    crate::desktop::procedure::window_proc(hwnd, msg, wp, lp)
}

#[test]
fn gpu_style_reentry_uses_current_document_and_destroy_revokes_creation() {
    for mode in [1, 2] {
        software_tests::fixture_mode(true, |state, hwnd| unsafe {
            SendMessageW(hwnd, WM_PAINT, None, None);
            assert_eq!(state.canvas.borrow().confirmed_paints, 1);
            context_menu::execute(hwnd, state, render_commands::AUTOMATIC).unwrap();
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
            SAFE.set(false);
            REENTER.set(mode);
            SendMessageW(hwnd, WM_PAINT, None, None);
            assert_eq!(REENTER.get(), 0);
            assert!(SAFE.get());
            if mode == 2 {
                assert!(!state.lifetime.owns(hwnd));
                assert!(!IsWindow(Some(hwnd)).as_bool());
                assert!(state.canvas.borrow().target.is_none());
                assert_eq!(state.canvas.borrow().confirmed_paints, 1);
            } else {
                assert!(state.error.borrow().is_none());
                assert_eq!(state.canvas.borrow().image.as_ref().unwrap().width, 2);
                assert!(matches!(
                    state.canvas.borrow().target.as_ref().unwrap().surface,
                    RenderSurface::D3d12(_)
                ));
                assert_eq!(state.canvas.borrow().confirmed_paints, 2);
            }
        });
    }
}

#[test]
#[ignore = "brief self-owned visible pin for production visibility reentry"]
fn gpu_visibility_reentry_destroy_or_group_hide_never_restores_an_unwanted_window() {
    for mode in [3, 4, 5, 6] {
        software_tests::fixture_mode(true, |state, hwnd| unsafe {
            let foreground = GetForegroundWindow();
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SendMessageW(hwnd, WM_PAINT, None, None);
            let count = state.canvas.borrow().confirmed_paints;
            context_menu::execute(hwnd, state, render_commands::AUTOMATIC).unwrap();
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
            REENTER.set(mode);
            SAFE.set(false);
            SendMessageW(hwnd, WM_PAINT, None, None);
            assert_eq!(REENTER.get(), 0);
            assert!(SAFE.get());
            assert!(!IsWindowVisible(hwnd).as_bool());
            assert_eq!(GetForegroundWindow(), foreground);
            if matches!(mode, 3 | 6) {
                assert!(!state.lifetime.owns(hwnd));
                assert!(state.canvas.borrow().target.is_none());
                // SHOW 阶段发生在 GPU draw 后，销毁不能再清错误或再发一帧。
                assert_eq!(
                    state.canvas.borrow().confirmed_paints,
                    count + u64::from(mode == 6)
                );
            } else {
                assert!(state.group_hidden.get());
                assert!(state.error.borrow().is_none());
                assert!(state.canvas.borrow().target.is_some());
            }
        });
    }
}

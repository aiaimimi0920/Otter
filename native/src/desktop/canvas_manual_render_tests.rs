//! 真实菜单命令→退休计时器→CPU 首帧；失败/重入不能借手动操作清除关联门禁。
use super::*;
use crate::desktop::{context_menu, render_commands};
use windows::{
    core::Error,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

unsafe fn finish(state: &crate::desktop::State, hwnd: HWND) {
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

unsafe fn assert_render_rejected(hwnd: HWND, state: &crate::desktop::State) {
    for command in [
        render_commands::SOFTWARE,
        render_commands::RETRY,
        render_commands::AUTOMATIC,
        render_commands::COMPATIBILITY,
    ] {
        assert!(render_commands::execute(hwnd, state, command).is_err());
    }
}

#[test]
fn busy_shape_dialog_rejects_without_menu_finish_invalidation() {
    software_tests::fixture_mode(true, |state, hwnd| unsafe {
        use windows::Win32::Graphics::Gdi::{GetUpdateRect, ValidateRect};
        let guard = crate::desktop::file_dialog::Guard::begin(&state.file_dialog).unwrap();
        let stamp = state.canvas.borrow().editor.document_stamp();
        // 两个既有自定义形状命令；不得弹窗或进入 menu_transition::finish。
        for command in [0x101C, 0x101D] {
            assert!(ValidateRect(Some(hwnd), None).as_bool());
            assert!(!GetUpdateRect(hwnd, None, false).as_bool());
            context_menu::execute(hwnd, state, command).unwrap();
            assert!(!GetUpdateRect(hwnd, None, false).as_bool());
            assert!(state.file_dialog.get());
            assert!(stamp.matches(&state.canvas.borrow().editor));
        }
        drop(guard);
        assert!(!state.file_dialog.get());
    });
}

#[test]
fn manual_cpu_menu_retires_each_hardware_backend_and_preserves_document() {
    for compatibility in [false, true] {
        software_tests::fixture_mode(true, |state, hwnd| unsafe {
            {
                let mut canvas = state.canvas.borrow_mut();
                canvas.software_renderer = false;
                canvas.compatibility_renderer = compatibility;
            }
            SendMessageW(hwnd, WM_PAINT, None, None);
            {
                let canvas = state.canvas.borrow();
                let surface = &canvas.target.as_ref().unwrap().surface;
                assert!(if compatibility {
                    matches!(surface, RenderSurface::D3d11(_))
                } else {
                    matches!(surface, RenderSurface::D3d12(_))
                });
            }
            let stamp = state.canvas.borrow().editor.document_stamp();
            let before = state.canvas.borrow().confirmed_paints;
            *state.error.borrow_mut() = Some(Error::new(E_FAIL, "keep until successful CPU frame"));
            state.group_locked.set(true); // 渲染维护不修改锁定文档。
            context_menu::execute(hwnd, state, render_commands::SOFTWARE).unwrap();
            assert!(state.error.borrow().is_some());
            assert!(stamp.matches(&state.canvas.borrow().editor));
            finish(state, hwnd);
            assert!(state.error.borrow().is_none());
            assert!(state.canvas.borrow().software_selected());
            assert!(state
                .canvas
                .borrow()
                .target
                .as_ref()
                .unwrap()
                .surface
                .is_software());
            assert_eq!(state.canvas.borrow().confirmed_paints, before + 1);
            assert!(stamp.matches(&state.canvas.borrow().editor));
            let menu = CreatePopupMenu().unwrap();
            render_commands::append(menu, state).unwrap();
            assert_ne!(
                GetMenuState(menu, render_commands::SOFTWARE as u32, MF_BYCOMMAND) & MF_CHECKED.0,
                0
            );
            DestroyMenu(menu).unwrap();
        });
    }
}

#[test]
fn software_terminal_retry_keeps_error_until_success_and_rejects_active_paint() {
    software_tests::fixture(|state, hwnd| unsafe {
        SendMessageW(hwnd, WM_PAINT, None, None);
        let stamp = state.canvas.borrow().editor.document_stamp();
        state.canvas.borrow_mut().suspend_render();
        *state.error.borrow_mut() = Some(Error::new(E_FAIL, "CPU failed"));
        state.render_active.set(true);
        {
            let _borrow = state.canvas.borrow_mut();
            assert!(context_menu::execute(hwnd, state, render_commands::RETRY).is_err());
        }
        assert!(state.canvas.borrow().render_suspended());
        state.render_active.set(false);
        {
            let _borrow = state.canvas.borrow();
            assert!(context_menu::execute(hwnd, state, render_commands::RETRY).is_err());
        }
        context_menu::execute(hwnd, state, render_commands::RETRY).unwrap();
        assert!(state.error.borrow().is_some());
        assert!(!state.canvas.borrow().render_suspended());
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert!(state.error.borrow().is_none());
        assert!(stamp.matches(&state.canvas.borrow().editor));
        assert_eq!(state.canvas.borrow().confirmed_paints, 2);
    });
}

#[test]
fn manual_request_cannot_unlock_pending_failed_or_missing_gpu_retirement_owner() {
    software_tests::fixture_mode(true, |state, hwnd| unsafe {
        state.canvas.borrow_mut().software_renderer = false;
        SendMessageW(hwnd, WM_PAINT, None, None);
        state
            .canvas
            .borrow_mut()
            .request_render(RenderChoice::Software)
            .unwrap();
        assert_render_rejected(hwnd, state);
        state.canvas.borrow_mut().suspend_render();
        let owner = state
            .canvas
            .borrow()
            .recovery
            .target
            .as_ref()
            .unwrap()
            .identity
            .clone();
        assert_render_rejected(hwnd, state);
        assert!(std::rc::Rc::ptr_eq(
            &owner,
            &state
                .canvas
                .borrow()
                .recovery
                .target
                .as_ref()
                .unwrap()
                .identity
        ));
        // 测试释放真实图预算；底层晚到完成仍不得解除窗口终态。
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let status = state
                .canvas
                .borrow_mut()
                .recovery
                .target
                .as_mut()
                .unwrap()
                .surface
                .retire_composition();
            if status == Some(crate::desktop::composition_retirement::Status::Complete) {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_render_rejected(hwnd, state);
    });
    software_tests::fixture(|state, hwnd| unsafe {
        state.canvas.borrow_mut().software_renderer = false;
        state.canvas.borrow_mut().suspend_render(); // 缺 owner 不是已确认完成的凭证。
        assert_render_rejected(hwnd, state);
        assert!(state.canvas.borrow().render_suspended());
    });
}

#[test]
fn explicitly_selected_cpu_without_a_target_can_retry_but_destroyed_host_cannot() {
    software_tests::fixture(|state, hwnd| unsafe {
        state.canvas.borrow_mut().suspend_render(); // CPU 创建前失败，没有 GPU 图。
        context_menu::execute(hwnd, state, render_commands::RETRY).unwrap();
        SendMessageW(hwnd, WM_PAINT, None, None);
        assert_eq!(state.canvas.borrow().confirmed_paints, 1);
        DestroyWindow(hwnd).unwrap();
        assert!(render_commands::execute(hwnd, state, render_commands::RETRY).is_err());
        assert_eq!(state.canvas.borrow().confirmed_paints, 1);
    });
}

#[test]
fn creation_rejection_remains_sticky_even_when_software_is_selected() {
    software_tests::fixture(|state, hwnd| unsafe {
        let _ = state
            .canvas
            .borrow_mut()
            .creation_admission
            .attempt::<()>(|| {
                Err(crate::desktop::composition_creation::Failure::attached(
                    Error::new(E_FAIL, "association fixture"),
                ))
            });
        for command in [
            render_commands::SOFTWARE,
            render_commands::RETRY,
            render_commands::AUTOMATIC,
            render_commands::COMPATIBILITY,
        ] {
            assert!(context_menu::execute(hwnd, state, command).is_err());
            assert!(state.canvas.borrow().creation_blocked());
        }
        let menu = CreatePopupMenu().unwrap();
        render_commands::append(menu, state).unwrap();
        assert_ne!(
            GetMenuState(menu, render_commands::RETRY as u32, MF_BYCOMMAND) & MF_GRAYED.0,
            0
        );
        DestroyMenu(menu).unwrap();
    });
}

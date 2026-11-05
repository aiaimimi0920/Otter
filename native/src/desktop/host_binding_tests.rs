//! 宿主身份在 WM_NCCREATE 建立；被拒候选不能进入原 State 的重入或销毁路径。
use super::*;
use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};

struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Host.Binding.Test"), Some(self.0));
        }
    }
}

unsafe fn create(
    instance: HINSTANCE,
    state: Option<&super::super::State>,
) -> windows::core::Result<Window> {
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("Otter.Host.Binding.Test"),
        w!("Otter Host 标题"),
        WS_OVERLAPPEDWINDOW,
        0,
        0,
        128,
        128,
        None,
        None,
        Some(instance),
        state.map(|state| (state as *const super::super::State).cast()),
    )
    .map(Window)
}

unsafe fn drain() -> Vec<usize> {
    let mut quit = Vec::new();
    let mut message = MSG::default();
    for _ in 0..1024 {
        if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            return quit;
        }
        if message.message == WM_QUIT {
            quit.push(message.wParam.0);
        } else {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    panic!("host binding test message budget exceeded");
}

#[test]
fn rejected_duplicate_and_unbound_candidate_do_not_touch_live_document() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Host.Binding.Test"),
                lpfnWndProc: Some(super::super::procedure::window_proc),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let state = super::super::State::new(None, false).unwrap();
        let unit = state.unit_id.clone();
        let window = create(instance, Some(&state)).unwrap();
        assert!(owns(window.0));
        let mut title = [0; 64];
        let length = GetWindowTextW(window.0, &mut title) as usize;
        assert_eq!(
            String::from_utf16_lossy(&title[..length]),
            "Otter Host 标题"
        );
        assert_eq!(LIVE.with(Cell::get), 1);
        assert!(drain().is_empty());
        {
            // 若候选消息或 NCDESTROY 误入 State，Canvas 借用冲突会触发 ABI panic/WM_QUIT(1)。
            let _canvas = state.canvas.borrow_mut();
            assert!(create(instance, Some(&state)).is_err());
            let candidate = create(instance, None).unwrap();
            assert!(!owns(candidate.0));
            let length = GetWindowTextW(candidate.0, &mut title) as usize;
            assert_eq!(
                String::from_utf16_lossy(&title[..length]),
                "Otter Host 标题"
            );
            assert!(!IsWindowVisible(candidate.0).as_bool());
            drop(candidate);
        }
        assert!(drain().is_empty());
        assert!(owns(window.0));
        assert_eq!(state.unit_id, unit);
        assert!(state.lifetime.registered.get());
        assert_eq!(LIVE.with(Cell::get), 1);
        // 原宿主继续能绘制；候选失败不触发关窗或退出。
        state
            .canvas
            .borrow_mut()
            .paint(window.0, 128, 128, 96)
            .unwrap();
        assert!(!IsWindowVisible(window.0).as_bool());
        drop(window);
        assert!(!state.lifetime.registered.get());
        assert!(state.lifetime.bound.get().is_invalid());
        assert_eq!(LIVE.with(Cell::get), 0);
        assert_eq!(drain(), vec![0]);
    }
}

#[test]
fn binding_is_exclusive_until_matching_nc_destroy_release() {
    let owner = Registration::default();
    let mut slots = [0u8; 2];
    let first = HWND(std::ptr::from_mut(&mut slots[0]).cast());
    let other = HWND(std::ptr::from_mut(&mut slots[1]).cast());
    assert!(!owner.bind(HWND::default()));
    assert!(owner.bind(first));
    assert!(!owner.bind(first));
    assert!(!owner.bind(other));
    owner.release(other);
    assert!(owner.owns(first));
    owner.release(first);
    assert!(owner.bind(other));
    owner.release(other);
}

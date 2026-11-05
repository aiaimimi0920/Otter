use super::*;
use windows::Win32::System::Threading::WaitForSingleObject;

// 仅装配真实事件 owner，不安装全局 hook、不发送键鼠，也不监听用户输入。
fn fixture() -> Watcher {
    assert!(EVENT.with(|event| event.get().is_invalid()));
    let event = unsafe { CreateEventW(None, true, false, None) }.unwrap();
    EVENT.with(|slot| slot.set(event));
    INPUT.with(|input| *input.borrow_mut() = input::Input::default());
    FAULT.with(|fault| fault.set(false));
    Watcher {
        keyboard: None,
        mouse: None,
        foreground: HWINEVENTHOOK::default(),
        minimize: HWINEVENTHOOK::default(),
        event: unsafe { OwnedHandle::from_raw_handle(event.0) },
        _thread: PhantomData,
    }
}

#[test]
fn callbacks_coalesce_into_owned_event_and_snapshot_rearms_without_polling() {
    let watcher = fixture();
    let event = HANDLE(watcher.handle().as_raw_handle());
    assert_eq!(unsafe { WaitForSingleObject(event, 0) }, WAIT_TIMEOUT);
    for _ in 0..100 {
        unsafe {
            window_event(
                HWINEVENTHOOK::default(),
                EVENT_SYSTEM_FOREGROUND,
                HWND::default(),
                0,
                0,
                0,
                0,
            );
        }
    }
    assert_eq!(unsafe { WaitForSingleObject(event, 0) }, WAIT_OBJECT_0);
    assert!(
        watcher.snapshot().unwrap().1.is_none(),
        "Window events never mint intent"
    );
    assert_eq!(unsafe { WaitForSingleObject(event, 0) }, WAIT_TIMEOUT);
    let now = unsafe { GetTickCount64() };
    update(|input| input.mouse_down(true, false, now, 1));
    assert!(watcher.snapshot().unwrap().1.is_some());
    assert_eq!(unsafe { WaitForSingleObject(event, 0) }, WAIT_TIMEOUT);
    update(|input| input.mouse_down(false, false, now, 1));
    assert!(watcher.snapshot().unwrap().1.is_none());
    assert!(
        Watcher::install().is_err(),
        "Duplicate installation must reject before touching hooks"
    );
    let retained = watcher.event.try_clone().unwrap();
    drop(watcher);
    wake();
    assert!(EVENT.with(|event| event.get().is_invalid()));
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(retained.as_raw_handle()), 0) },
        WAIT_TIMEOUT
    );
}

#[test]
fn reentrant_input_update_fails_closed_and_wakes_ui() {
    let watcher = fixture();
    INPUT.with(|input| {
        let _borrow = input.borrow_mut();
        update(|input| input.mouse_down(true, false, 1, 1));
    });
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(watcher.handle().as_raw_handle()), 0) },
        WAIT_OBJECT_0
    );
    assert!(watcher.snapshot().is_err());
}

#[test]
#[ignore = "explicit native hook installation lifecycle gate; does not synthesize input"]
fn actual_hooks_install_unregister_and_reinstall_on_owner_thread() {
    // 只短暂安装/卸载，不泵用户输入，不创建 Intent 或操作任何用户窗口。
    for _ in 0..3 {
        let watcher = Watcher::install().unwrap();
        assert!(watcher.keyboard.is_some() && watcher.mouse.is_some());
        assert!(!watcher.foreground.is_invalid() && !watcher.minimize.is_invalid());
        assert!(Watcher::install().is_err());
        let keyboard = watcher.keyboard.unwrap();
        let mouse = watcher.mouse.unwrap();
        let foreground = watcher.foreground;
        let minimize = watcher.minimize;
        drop(watcher);
        assert!(EVENT.with(|event| event.get().is_invalid()));
        unsafe {
            assert!(UnhookWindowsHookEx(keyboard).is_err());
            assert!(UnhookWindowsHookEx(mouse).is_err());
            assert!(!UnhookWinEvent(foreground).as_bool());
            assert!(!UnhookWinEvent(minimize).as_bool());
        }
    }
    println!("native-live-reclaim-hooks: install=true unregister=true reinstall=true input-synthesized=false");
}

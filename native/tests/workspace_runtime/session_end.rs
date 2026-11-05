//! 普通固定SHA产品的系统消息→取消→继续编辑→确认结束；只操作自启PID的离屏窗口。
use super::*;

unsafe fn send(expected_pid: u32, hwnd: HWND, message: u32, value: usize) -> usize {
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    assert_eq!(
        pid, expected_pid,
        "refuse unrelated process session message"
    );
    let mut result = usize::MAX;
    assert_ne!(
        SendMessageTimeoutW(
            hwnd,
            message,
            WPARAM(value),
            LPARAM(0x80000000),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            2000,
            Some(&mut result)
        )
        .0,
        0,
        "owned session message failed: {:?}",
        windows::core::Error::from_win32()
    );
    result
}

#[test]
fn ordinary_product_session_cancel_resumes_checkpoint_and_confirmed_end_keeps_last_unit() {
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    let fixture = Fixture::new();
    let image = base64::engine::general_purpose::STANDARD
        .encode(include_bytes!("../../../src-tauri/icons/32x32.png"));
    let source = serde_json::json!({
        "documentSchemaVersion":1,"documentRevision":7,
        "otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},
        "stickers":[{"id":"session-end-runtime","x":-16000,"y":-16000,"w":32,"h":32,"src":format!("data:image/png;base64,{image}")}],
        "links":[],"groups":[],"extension":{"session-end-preserved":true}
    });
    std::fs::write(fixture.workspace(), serde_json::to_vec(&source).unwrap()).unwrap();
    let mut child = fixture.launch("session-end-stderr.log");
    let hwnd = window(&mut child);
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            -15999,
            -15998,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    until(&mut child, "initial-durable-checkpoint", |_| {
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.workspace()).ok()?).ok()?;
        (value["documentRevision"].as_u64()? > 7).then_some(())
    });
    // 磁盘replace先于UI消费Receipt；按原15秒界等待真正准入，不把文件存在当作已commit。
    until(&mut child, "initial-session-admission", |child| {
        (unsafe { send(child, hwnd, WM_QUERYENDSESSION, 0) } == 1).then_some(())
    });
    unsafe {
        assert_eq!(send(child.0.id(), hwnd, WM_ENDSESSION, 0), 0);
        SetWindowPos(
            hwnd,
            None,
            -15995,
            -15994,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    let persisted = until(&mut child, "cancelled-session-resumed-checkpoint", |_| {
        let bytes = std::fs::read(fixture.workspace()).ok()?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        (value["stickers"][0]["x"].as_f64()? == -15995.0
            && value["stickers"][0]["y"].as_f64()? == -15994.0)
            .then_some(bytes)
    });
    until(&mut child, "resumed-session-admission", |child| {
        (unsafe { send(child, hwnd, WM_QUERYENDSESSION, 0) } == 1).then_some(())
    });
    unsafe {
        assert_eq!(send(child.0.id(), hwnd, WM_ENDSESSION, 1), 0);
    }
    let end = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < end, "confirmed session exit timed out");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(fixture.0.join("session-end-stderr.log")).unwrap()
    );
    assert_eq!(std::fs::read(fixture.workspace()).unwrap(), persisted);
    let value: serde_json::Value = serde_json::from_slice(&persisted).unwrap();
    assert_eq!(value["extension"]["session-end-preserved"], true);
    assert_eq!(value["stickers"].as_array().unwrap().len(), 1);
    assert_eq!(value["stickers"][0]["id"], "session-end-runtime");
}

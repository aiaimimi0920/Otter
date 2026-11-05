//! 普通启动正式菜单退出；只操作自启 PID 的离屏工作区及其临时菜单/确认框。
use super::*;
use windows::Win32::{System::Com::*, UI::Accessibility::*};
#[path = "matcher.rs"]
mod matcher;
struct Dialog {
    pid: u32,
    hwnd: Option<HWND>,
    menu: bool,
    owner: HWND,
}
unsafe extern "system" fn dialog(hwnd: HWND, parameter: LPARAM) -> BOOL {
    let search = &mut *(parameter.0 as *mut Dialog);
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == search.pid {
        let mut class = [0u16; 64];
        let size = GetClassNameW(hwnd, &mut class);
        let mut title = [0u16; 64];
        let length = GetWindowTextW(hwnd, &mut title);
        let (expected_class, expected_title) = if search.menu {
            ("STATIC", w!("Otter 菜单"))
        } else {
            ("#32770", w!("退出 Otter"))
        };
        // Win32 返回规范类名 Static；注册名不区分 ASCII 大小写，其他身份条件不放宽。
        if size > 0
            && String::from_utf16_lossy(&class[..size as usize])
                .eq_ignore_ascii_case(expected_class)
            && if search.menu {
                // GetWindowTextW 不保证跨进程读取 STATIC 文本；用 PID+精确 owner+类鉴别。
                IsWindowVisible(hwnd).as_bool()
                    && GetWindow(hwnd, GW_OWNER).ok() == Some(search.owner)
            } else {
                length > 0 && &title[..length as usize] == expected_title.as_wide()
            }
        {
            search.hwnd = Some(hwnd);
        }
    }
    BOOL(1)
}
struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
unsafe fn invoke_exit(menu: HWND) {
    CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
    let _com = Com;
    let client: IUIAutomation2 =
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).unwrap();
    client.SetConnectionTimeout(3000).unwrap();
    client.SetTransactionTimeout(3000).unwrap();
    let root = client.ElementFromHandle(menu).unwrap();
    let condition = client
        .CreatePropertyCondition(UIA_NamePropertyId, &"退出 Otter".into())
        .unwrap();
    let end = Instant::now() + Duration::from_secs(15);
    for _ in 0..16 {
        if let Ok(row) = root.FindFirst(TreeScope_Children, &condition) {
            let invoke: IUIAutomationInvokePattern =
                row.GetCurrentPatternAs(UIA_InvokePatternId).unwrap();
            invoke.Invoke().unwrap();
            return;
        }
        assert!(
            Instant::now() < end,
            "owned menu exit item discovery timed out"
        );
        let scroll: IUIAutomationScrollPattern =
            root.GetCurrentPatternAs(UIA_ScrollPatternId).unwrap();
        scroll
            .Scroll(ScrollAmount_NoAmount, ScrollAmount_LargeIncrement)
            .unwrap();
    }
    panic!("owned menu exit item exceeded bounded pages");
}
#[test]
#[ignore = "requires a short owned foreground interaction; run explicitly"]
fn ordinary_menu_exit_flushes_final_workspace_before_destroying_windows() {
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    let fixture = Fixture::new();
    let image = base64::engine::general_purpose::STANDARD
        .encode(include_bytes!("../../../src-tauri/icons/32x32.png"));
    let source = serde_json::json!({
        "documentSchemaVersion":1,"documentRevision":7,
        "otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},
        "stickers":[{"id":"workspace-exit","x":-16000,"y":-16000,"w":32,"h":32,"src":format!("data:image/png;base64,{image}")}],
        "links":[],"groups":[],"extension":{"exit-preserved":true}
    });
    std::fs::write(fixture.workspace(), serde_json::to_vec(&source).unwrap()).unwrap();
    let mut child = fixture.launch("exit-stderr.log");
    let hwnd = window(&mut child);
    unsafe {
        SetWindowPos(hwnd, None, 40, 40, 160, 160, SWP_NOZORDER | SWP_NOACTIVATE).unwrap();
        let _ = SetForegroundWindow(hwnd); // 只请求一次，拒绝时等待真实原生输入，不绕过门禁。
    }
    until(&mut child, "owned-product-foreground", |_| unsafe {
        (GetForegroundWindow() == hwnd).then_some(())
    });
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            -15991,
            -15993,
            32,
            32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        // 真正打开原生业务菜单，不能用并非本窗口合同的 WM_COMMAND 冒充菜单交付。
        assert_eq!(
            GetForegroundWindow(),
            hwnd,
            "owned menu requires real product foreground"
        );
        PostMessageW(
            Some(hwnd),
            WM_CONTEXTMENU,
            WPARAM(hwnd.0 as usize),
            LPARAM(100 | (100 << 16)),
        )
        .unwrap();
    }
    let menu = until(&mut child, "owned-native-menu", |pid| unsafe {
        let mut search = Dialog {
            pid,
            hwnd: None,
            menu: true,
            owner: hwnd,
        };
        EnumWindows(Some(dialog), LPARAM(&mut search as *mut Dialog as isize)).unwrap();
        search.hwnd
    });
    unsafe {
        invoke_exit(menu);
    }
    let modal = until(&mut child, "exit-confirmation", |pid| unsafe {
        let mut search = Dialog {
            pid,
            hwnd: None,
            menu: false,
            owner: hwnd,
        };
        EnumWindows(Some(dialog), LPARAM(&mut search as *mut Dialog as isize)).unwrap();
        search.hwnd
    });
    let before: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.workspace()).unwrap()).unwrap();
    unsafe {
        assert!(IsWindow(Some(hwnd)).as_bool());
        let mut pid = 0;
        GetWindowThreadProcessId(modal, Some(&mut pid));
        assert_eq!(
            pid,
            child.0.id(),
            "refuse to confirm any other process dialog"
        );
        PostMessageW(Some(modal), WM_COMMAND, WPARAM(IDNO.0 as usize), LPARAM(0)).unwrap();
    }
    let end = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < end,
            "native final checkpoint exit timed out"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(fixture.0.join("exit-stderr.log")).unwrap()
    );
    let final_bytes = std::fs::read(fixture.workspace()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&final_bytes).unwrap();
    assert_eq!(
        value["documentRevision"].as_u64().unwrap(),
        before["documentRevision"].as_u64().unwrap() + 1
    );
    assert_eq!(value["stickers"][0]["x"].as_f64(), Some(-15991.0));
    assert_eq!(value["stickers"][0]["y"].as_f64(), Some(-15993.0));
    assert_eq!(value["extension"]["exit-preserved"], true);
    assert_eq!(
        otter_native::session_document::decode(&final_bytes)
            .unwrap()
            .workspace
            .units()
            .len(),
        1
    );
}

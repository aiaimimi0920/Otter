//! Tray-menu exit after ordinary startup; touch only the self-started PID's workspace and temporary menu.
use super::*;
use windows::Win32::{System::Com::*, UI::Accessibility::*};
#[path = "matcher.rs"]
mod matcher;
struct Dialog {
    pid: u32,
    hwnd: Option<HWND>,
    owner: HWND,
}
unsafe extern "system" fn dialog(hwnd: HWND, parameter: LPARAM) -> BOOL {
    let search = &mut *(parameter.0 as *mut Dialog);
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == search.pid {
        let mut class = [0u16; 64];
        let size = GetClassNameW(hwnd, &mut class);
        // Win32 returns the canonical class name Static; registered names are ASCII case-insensitive, without relaxing other identity checks.
        if size > 0
            && String::from_utf16_lossy(&class[..size as usize]).eq_ignore_ascii_case("STATIC")
            // GetWindowTextW does not guarantee cross-process STATIC text reads; identify by PID, exact owner and class.
            && IsWindowVisible(hwnd).as_bool()
            && GetWindow(hwnd, GW_OWNER).ok() == Some(search.owner)
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
        .CreatePropertyCondition(UIA_NamePropertyId, &"退出".into())
        .unwrap();
    let find = || root.FindAll(TreeScope_Children, &condition).unwrap();
    let mut rows = find();
    if rows.Length().unwrap() == 0 {
        let scroll: IUIAutomationScrollPattern =
            root.GetCurrentPatternAs(UIA_ScrollPatternId).unwrap();
        assert!(
            scroll.CurrentVerticallyScrollable().unwrap().as_bool(),
            "Exit is missing from the complete tray menu"
        );
        // Exit is the last row; asynchronous scrolling must wait for the provider's new snapshot, not send repeated requests against the old page.
        scroll
            .SetScrollPercent(UIA_ScrollPatternNoScroll, 100.0)
            .unwrap();
        let end = Instant::now() + Duration::from_secs(3);
        while scroll.CurrentVerticalScrollPercent().unwrap() < 100.0 {
            assert!(
                Instant::now() < end,
                "owned tray menu scroll did not settle"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        rows = find();
    }
    assert_eq!(
        rows.Length().unwrap(),
        1,
        "expected one exact tray exit row"
    );
    let invoke: IUIAutomationInvokePattern = rows
        .GetElement(0)
        .unwrap()
        .GetCurrentPatternAs(UIA_InvokePatternId)
        .unwrap();
    invoke.Invoke().unwrap();
}
#[test]
#[ignore = "requires a short owned foreground interaction; run explicitly"]
fn ordinary_tray_menu_exit_persists_final_workspace() {
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    let fixture = Fixture::new();
    let image = base64::engine::general_purpose::STANDARD
        .encode(include_bytes!("../../assets/branding/32x32.png"));
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
        let _ = SetForegroundWindow(hwnd); // Request only once; on refusal, wait for real native input rather than bypassing the gate.
    }
    until(&mut child, "owned-product-foreground", |_| unsafe {
        (GetForegroundWindow() == hwnd).then_some(())
    });
    let before: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.workspace()).unwrap()).unwrap();
    assert_ne!(before["stickers"][0]["x"].as_f64(), Some(-15991.0));
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
        // The tray uses NOTIFYICON_VERSION_4: the low word is the event and the high word is the icon ID.
        // Deliver the notification only to the self-started foreground owner; neither simulate an Explorer click nor directly send an exit command.
        assert_eq!(
            GetForegroundWindow(),
            hwnd,
            "owned tray callback requires real product foreground"
        );
        PostMessageW(
            Some(hwnd),
            WM_APP + 91,
            WPARAM(100 | (100 << 16)),
            LPARAM((WM_CONTEXTMENU | (1 << 16)) as isize),
        )
        .unwrap();
    }
    let menu = until(&mut child, "owned-tray-menu-foreground", |pid| unsafe {
        let mut search = Dialog {
            pid,
            hwnd: None,
            owner: hwnd,
        };
        EnumWindows(Some(dialog), LPARAM(&mut search as *mut Dialog as isize)).unwrap();
        search.hwnd.filter(|menu| *menu == GetForegroundWindow())
    });
    unsafe {
        invoke_exit(menu);
    }
    // Normal exit saves the scene by default without a Save As confirmation.
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
    assert!(
        value["documentRevision"].as_u64().unwrap() > before["documentRevision"].as_u64().unwrap()
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

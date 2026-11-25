//! 普通解包产品的钉图设置、实际窗口几何与重启读回；不向桌面注入全局输入。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{IsWindowEnabled, VK_F4};

struct OwnedPopup<'a> {
    pid: u32,
    owner: HWND,
    class: &'a [u16],
    found: Option<HWND>,
}
unsafe extern "system" fn find_popup(hwnd: HWND, parameter: LPARAM) -> BOOL {
    let search = &mut *(parameter.0 as *mut OwnedPopup<'_>);
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == search.pid && GetWindow(hwnd, GW_OWNER).ok() == Some(search.owner) {
        let mut class = [0; 64];
        let size = GetClassNameW(hwnd, &mut class);
        if size > 0 && &class[..size as usize] == search.class {
            search.found = Some(hwnd);
        }
    }
    BOOL(1)
}
unsafe fn popup(pid: u32, owner: HWND, class: &[u16]) -> Option<HWND> {
    let mut search = OwnedPopup {
        pid,
        owner,
        class,
        found: None,
    };
    SetLastError(ERROR_SUCCESS);
    let result = EnumWindows(Some(find_popup), LPARAM(&mut search as *mut _ as isize));
    enumerated_window(result, search.found).unwrap()
}
unsafe fn click(hwnd: HWND, pid: u32) {
    let mut actual = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut actual));
    assert_eq!(
        actual, pid,
        "only the isolated product button may be activated"
    );
    assert!(IsWindowVisible(hwnd).as_bool() && IsWindowEnabled(hwnd).as_bool());
    PostMessageW(Some(hwnd), BM_CLICK, WPARAM(0), LPARAM(0)).unwrap();
}
unsafe fn bounds(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect).unwrap();
    rect
}

#[test]
#[ignore = "requires an owned product foreground window; run explicitly"]
fn foreground_pin_settings_width_changes_window_and_survives_restart() {
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        let fixture = Fixture::new();
        let image = base64::engine::general_purpose::STANDARD
            .encode(include_bytes!("../../assets/branding/32x32.png"));
        let source = serde_json::json!({
            "documentSchemaVersion":1,"documentRevision":7,
            "otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},
            "stickers":[{"id":"settings-pin","x":400,"y":300,"w":240,"h":240,"src":format!("data:image/png;base64,{image}")}],
            "links":[],"groups":[],"extension":{"settings-preserved":true}
        });
        std::fs::write(fixture.workspace(), serde_json::to_vec(&source).unwrap()).unwrap();
        let mut child = fixture.launch("pin-settings-stderr.log");
        let owner = window(&mut child);
        let pid = child.0.id();
        let _ = SetForegroundWindow(owner);
        until(&mut child, "pin-settings-product-foreground", |_| {
            (GetForegroundWindow() == owner).then_some(())
        });
        let toolbar = until(&mut child, "owned-pin-toolbar", |pid| {
            popup(pid, owner, w!("NeuroMini.Native.FloatingToolbar").as_wide())
        });
        if !IsWindowVisible(toolbar).as_bool() {
            PostMessageW(Some(owner), WM_KEYDOWN, WPARAM(VK_F4.0 as usize), LPARAM(1)).unwrap();
        }
        let settings = until(&mut child, "enabled-pin-settings-button", |_| {
            GetDlgItem(Some(toolbar), 0x6100).ok().filter(|button| {
                IsWindowVisible(*button).as_bool() && IsWindowEnabled(*button).as_bool()
            })
        });
        click(settings, pid);
        let panel = until(&mut child, "owned-pin-settings-panel", |pid| {
            popup(pid, toolbar, w!("NeuroMini.Native.PropertyBar").as_wide())
                .filter(|panel| IsWindowVisible(*panel).as_bool())
        });
        let viewport =
            FindWindowExW(Some(panel), None, w!("NeuroMini.Native.PropertyBar"), None).unwrap();
        // NeuroMini 的实际数值控件：字段 101 的加号为 101 * 4 + 1。
        let increase = GetDlgItem(Some(viewport), 101 * 4 + 1).unwrap();
        let before = bounds(owner);
        // 属性窗可接管前台；只接受本次已经按 PID／owner 核过的三个窗口。
        let foreground = GetForegroundWindow();
        assert!(
            [owner, toolbar, panel].contains(&foreground),
            "foreign foreground: {foreground:?}, owner={owner:?}, toolbar={toolbar:?}, panel={panel:?}"
        );
        click(increase, pid);
        let after = until(&mut child, "settings-width-applied-to-real-window", |_| {
            let next = bounds(owner);
            (next.right - next.left > before.right - before.left).then_some(next)
        });
        assert!(after.bottom - after.top > before.bottom - before.top);
        let width = f64::from(after.right - after.left);
        click(settings, pid);
        until(&mut child, "pin-settings-closed", |_| {
            (!IsWindowVisible(panel).as_bool()).then_some(())
        });
        until(&mut child, "settings-width-checkpoint", |_| {
            let data = std::fs::read(fixture.workspace()).ok()?;
            let saved: serde_json::Value = serde_json::from_slice(&data).ok()?;
            (saved["stickers"][0]["w"].as_f64() == Some(width)).then(|| {
                assert!(saved["documentRevision"].as_u64().unwrap() > 7);
                assert_eq!(saved["stickers"][0]["id"], "settings-pin");
                assert_eq!(saved["extension"]["settings-preserved"], true);
            })
        });
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        let mut restored = fixture.launch("pin-settings-restored-stderr.log");
        let restored_pin = window(&mut restored);
        let actual = bounds(restored_pin);
        assert_eq!(actual.right - actual.left, after.right - after.left);
        assert_eq!(actual.bottom - actual.top, after.bottom - after.top);
    }
}

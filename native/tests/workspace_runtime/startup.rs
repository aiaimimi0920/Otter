//! 普通无参 GUI 产品：不使用 CREATE_NO_WINDOW，不显示空工作区/面板，保留可响应的宿主。
use super::*;
use std::io::{Read, Seek, SeekFrom};

fn assert_gui_product() {
    let product =
        product::Product::resolve(std::path::Path::new(env!("CARGO_BIN_EXE_otter-native")))
            .unwrap();
    let mut file = std::fs::File::open(product.path()).unwrap();
    let mut dos = [0u8; 64];
    file.read_exact(&mut dos).unwrap();
    assert_eq!(&dos[..2], b"MZ");
    let pe = u32::from_le_bytes(dos[60..64].try_into().unwrap());
    file.seek(SeekFrom::Start(u64::from(pe))).unwrap();
    let mut header = [0u8; 94];
    file.read_exact(&mut header).unwrap();
    assert_eq!(&header[..4], b"PE\0\0");
    assert_eq!(
        u16::from_le_bytes(header[24..26].try_into().unwrap()),
        0x20b
    );
    assert_eq!(
        u16::from_le_bytes(header[92..94].try_into().unwrap()),
        2,
        "GUI subsystem required; hiding a console after startup still flashes"
    );
}

unsafe extern "system" fn visible_owned(hwnd: HWND, parameter: LPARAM) -> BOOL {
    let search = &mut *(parameter.0 as *mut Search);
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == search.pid && IsWindowVisible(hwnd).as_bool() {
        search.hwnd = Some(hwnd);
    }
    BOOL(1)
}

#[test]
fn fresh_and_restored_empty_workspace_start_without_console_or_visible_panels() {
    assert_gui_product();
    for restore in [false, true] {
        let fixture = Fixture::new();
        if restore {
            std::fs::write(fixture.workspace(), br#"{"documentSchemaVersion":1,"documentRevision":1,"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},"stickers":[],"links":[],"groups":[]}"#).unwrap();
        }
        let mut child = fixture.launch("quiet-start-stderr.log");
        owned_window(&mut child, false);
        let end = Instant::now() + Duration::from_secs(1);
        while Instant::now() < end {
            assert!(child.0.try_wait().unwrap().is_none());
            let mut search = Search {
                pid: child.0.id(),
                hwnd: None,
            };
            unsafe {
                EnumWindows(
                    Some(visible_owned),
                    LPARAM(&mut search as *mut Search as isize),
                )
                .unwrap();
            }
            assert!(
                search.hwnd.is_none(),
                "empty startup displayed a product window"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        // 等待真正恢复/检查点准入；隐藏宿主仍能完成原正常退出事务。
        let hwnd = until(&mut child, "hidden-host-session-admission", |pid| unsafe {
            // 恢复会替换初始 HWND，每次查询重新找到当前 owner；销毁中的旧 HWND 只重试发现。
            let mut search = Search { pid, hwnd: None };
            EnumWindows(Some(enumerate), LPARAM(&mut search as *mut Search as isize)).ok()?;
            let hwnd = search.hwnd?;
            let mut accepted = 0;
            let sent = SendMessageTimeoutW(
                hwnd,
                WM_QUERYENDSESSION,
                WPARAM(0),
                LPARAM(0x80000000),
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                2000,
                Some(&mut accepted),
            );
            (sent.0 != 0 && accepted == 1).then_some(hwnd)
        });
        unsafe {
            session_end::send(child.0.id(), hwnd, WM_ENDSESSION, 1);
        }
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

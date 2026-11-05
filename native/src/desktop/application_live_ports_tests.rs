//! 真实Live菜单单槽、worker保存及同ID冻结的显隐连续性。
use super::*;
use crate::workspace::ports::Visibility;
#[test]
fn live_system_menu_routes_owned_open_without_removing_the_endpoint() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, id) = live(&mut app, true);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let command = crate::desktop::application::chrome::ports::LIVE_COMMAND;
        assert_ne!(
            GetMenuState(GetSystemMenu(hwnd, false), command as u32, MF_BYCOMMAND),
            u32::MAX
        );
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(command)), None);
        assert!(!app.chrome.idle());
        assert!(app.workspace.contains(&id));
        app.chrome.close();
        assert!(app.chrome.idle());
        app.shutdown();
    }
}
#[test]
fn live_save_receipt_rejects_appearance_change_then_snapshot_and_freeze_reopen_keep_override() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, id) = live(&mut app, true);
        let value = Visibility::read(r#"{"output":false}"#).unwrap();
        app.workspace
            .set_port_visibility(&id, value.clone())
            .unwrap();
        let path = save_publish::smoke_destination().unwrap();
        app.request_save_new(path).unwrap();
        let output = save::prepared(&mut app).unwrap();
        let decoded = crate::session_document::decode(&output.candidate.bytes).unwrap();
        assert_eq!(decoded.workspace.port_visibility(&id), value);
        app.workspace
            .set_port_visibility(&id, Visibility::default())
            .unwrap();
        assert!(!output.receipt.matches(&app));
        drop(output);
        app.workspace
            .set_port_visibility(&id, value.clone())
            .unwrap();
        queue(hwnd);
        app.process_live_freeze(false).unwrap();
        settle(&mut app).unwrap();
        assert_eq!(app.workspace.port_visibility(&id), value);
        assert!(app.documents.iter().any(|doc| doc.state.unit_id == id));
        app.shutdown_live().unwrap();
        let (_, data) = app.capture_session_snapshot().unwrap();
        let candidate = data.encode(&AtomicBool::new(false)).unwrap();
        let reopened = crate::session_document::decode(&candidate.bytes).unwrap();
        assert_eq!(reopened.workspace.port_visibility(&id), value);
        app.shutdown();
    }
}

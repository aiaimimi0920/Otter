//! 同节点 Live 编辑、普通快捷键、标注保留与目录唯一性回归。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

unsafe fn edit_key(hwnd: HWND) {
    SendMessageW(hwnd, WM_PAINT, None, None);
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect).unwrap();
    let corner = LPARAM(((rect.right - 2) | ((rect.bottom - 2) << 16)) as isize);
    SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(corner));
    SendMessageW(hwnd, WM_LBUTTONUP, None, Some(corner));
    let mut prior = [0; 256];
    GetKeyboardState(&mut prior).unwrap();
    let mut control = [0; 256];
    control[VK_CONTROL.0 as usize] = 0x80;
    SetKeyboardState(&control).unwrap();
    SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x45)), None);
    SetKeyboardState(&prior).unwrap();
}
unsafe fn open(app: &mut Application, hwnd: HWND) {
    edit_key(hwnd);
    app.process_live_freeze(false).unwrap();
    assert!(app.live_freeze_job.is_some());
    settle(app).unwrap();
}
#[test]
fn live_pin_ctrl_e_keeps_session_and_id_reuses_editor_and_roundtrips_annotations() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, unit) = live(&mut app, true);
        open(&mut app, hwnd);
        assert!(app.live_sessions[0].is_alive());
        assert!(app.live_sessions[0].pin_attached());
        assert_eq!(
            app.documents
                .iter()
                .filter(|d| d.state.unit_id == unit)
                .count(),
            1
        );
        let doc = app
            .documents
            .iter()
            .find(|d| d.state.unit_id == unit)
            .unwrap();
        assert!(
            IsWindowVisible(doc.panel.0).as_bool(),
            "Ctrl+E must open ordinary edit toolbar"
        );
        let original = doc.window.0;
        {
            let mut canvas = doc.state.canvas.borrow_mut();
            canvas
                .editor
                .begin(
                    crate::editor::Tool::Brush,
                    crate::viewport::Point { x: 0.0, y: 0.0 },
                )
                .unwrap();
            canvas
                .editor
                .extend(crate::viewport::Point { x: 1.0, y: 1.0 })
                .unwrap();
            assert!(canvas.editor.commit());
        }
        let catalog = crate::desktop::link_catalog::Catalog::capture(
            &app.documents,
            &app.live_sessions,
            &app.workspace,
        );
        assert_eq!(
            catalog.endpoints.iter().filter(|e| e.unit == unit).count(),
            1
        );
        assert!(catalog.matches(&app.documents, &app.live_sessions, &app.workspace));
        doc.state.live_resume.set(true);
        app.settle_live_pins(false).unwrap();
        assert!(!app.live_sessions[0].pin_attached());
        assert!(app.documents.iter().all(|d| d.state.unit_id != unit));
        assert_eq!(
            app.live_sessions[0]
                .pin_snapshot()
                .unwrap()
                .elements()
                .len(),
            1
        );
        app.request_save_candidate().unwrap();
        let saved = save::prepared(&mut app).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&saved.candidate.bytes).unwrap();
        assert_eq!(
            json["stickers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|sticker| sticker["id"] == unit.as_str())
                .count(),
            1
        );
        open(&mut app, hwnd);
        let doc = app
            .documents
            .iter()
            .find(|d| d.state.unit_id == unit)
            .unwrap();
        assert_eq!(doc.window.0, original);
        assert_eq!(doc.state.canvas.borrow().editor.elements().len(), 1);
        assert!(doc.state.canvas.borrow_mut().editor.undo());
        assert!(doc.state.canvas.borrow().editor.elements().is_empty());
        // 文档关闭必须级联到采集 owner，不能留下仍在捕获的空 Live。
        DestroyWindow(doc.window.0).unwrap();
        app.collect_closed();
        assert!(!app.live_sessions[0].is_alive());
        app.shutdown_live().unwrap();
    }
}

#[test]
fn live_pin_recycle_last_document_stops_hidden_capture_and_keeps_resident() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (live_hwnd, unit) = live(&mut app, true);
        open(&mut app, live_hwnd);
        let resident = app
            .documents
            .iter()
            .find(|d| d.state.unit_id != unit)
            .unwrap()
            .window
            .0;
        DestroyWindow(resident).unwrap();
        app.collect_closed();
        assert_eq!(app.documents.len(), 1);
        app.sync_frozen_menu();
        let doc = &app.documents[0];
        let edited = doc.window.0;
        crate::desktop::context_menu::execute(
            edited,
            &doc.state,
            crate::desktop::reference_ui::RECYCLE,
        )
        .unwrap();
        app.process_reference(false).unwrap();
        assert!(app.save_job.is_some());
        super::recycle::settle_recycle(&mut app).unwrap();
        assert!(!IsWindow(Some(edited)).as_bool());
        assert!(!IsWindow(Some(live_hwnd)).as_bool());
        assert_eq!(app.documents.len(), 1);
        assert!(app.documents[0].is_alive());
        assert_ne!(app.documents[0].state.unit_id, unit);
        assert!(!app.live_sessions[0].is_alive());
        app.shutdown_live().unwrap();
    }
}

//! 清空走真实菜单命令、单槽 worker 和原 Receipt；仅内存夹具，不清理用户文件。
use super::*;
use crate::{desktop::reference_ui as ui, session_library::Kind};
#[test]
fn clear_commands_use_library_transaction_and_reject_cancelled_or_stale_work() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        for command in [ui::CLEAR_RECYCLE, ui::CLEAR_REFERENCE] {
            for mode in 0..3 {
                let mut app = chrome::tests::app();
                let entries = serde_json::json!([{"entryId":"entry","sourceStickerId":"old","createdAt":"legacy","snapshot":{"id":"old","x":0,"y":0,"w":32,"h":32}}]);
                app.restored_session.as_mut().unwrap().original = serde_json::to_vec(&serde_json::json!({"documentRevision":9,"stickers":[],"links":[],"recycleBin":entries,"referenceLibrary":entries})).unwrap().into();
                app.sync_frozen_menu();
                let source = app.restored_session.as_ref().unwrap().original.clone();
                let doc = &app.documents[0];
                let hwnd = doc.window.0;
                crate::desktop::context_menu::execute(hwnd, &doc.state, command).unwrap();
                app.process_reference(false).unwrap();
                assert!(app.save_job.as_ref().unwrap().1.is_library_update());
                assert!(app.take_save_candidate().is_err());
                if mode == 0 {
                    app.save_job.as_ref().unwrap().1.cancel();
                }
                if mode == 1 {
                    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
                }
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while !app.save_job.as_ref().unwrap().1.is_ready() {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                let result = app.poll_reference();
                let current = &app.restored_session.as_ref().unwrap().original;
                if mode < 2 {
                    assert!(result.is_err());
                    assert!(current.same_storage(&source));
                } else {
                    result.unwrap();
                    let kind = if command == ui::CLEAR_RECYCLE {
                        Kind::RecycleBin
                    } else {
                        Kind::ReferenceLibrary
                    };
                    assert!(crate::session_library::decode(current, kind)
                        .unwrap()
                        .entries()
                        .is_empty());
                    assert_eq!(
                        crate::session_document::decode(current).unwrap().revision,
                        9
                    );
                    assert_eq!(app.documents.len(), 1);
                    assert!(IsWindow(Some(hwnd)).as_bool());
                }
                app.shutdown();
            }
        }
    }
}

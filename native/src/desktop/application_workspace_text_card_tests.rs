//! Exercise the text snapshot through the open worker, native inline editing,
//! a real disk checkpoint and a fresh Application. No system clipboard or input.
use super::*;
use crate::desktop::{open_job, paste_input, text_input, workspace_window_fixture};
use windows::Win32::UI::Input::KeyboardAndMouse::GetFocus;

#[test]
fn text_card_worker_inline_edit_checkpoint_and_reopen_keep_one_editable_layer() {
    workspace_window_fixture::with_private_input_desktop(|| unsafe {
        run(|instance| {
            let dir = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&dir.0).unwrap()), true)
                .unwrap();
            let before = app.documents.len();
            let value = "\tfn 你好() {\r\n\r\n    let water = \"🦦\";\r\n}";
            let units = || paste_input::Snapshot::Text(value.encode_utf16().collect());
            let first = &app.documents[0];
            assert!(paste_input::deliver(first.window.0, &first.state, units(), false).is_err());
            paste_input::deliver(first.window.0, &first.state, units(), true).unwrap();
            open_job::finish(first.window.0, &first.state, true);
            app.pending_windows(false).unwrap();
            assert_eq!(app.documents.len(), before + 1);
            let doc = app.documents.last().unwrap();
            let id = doc.state.unit_id.clone();
            let pending = {
                let mut c = doc.state.canvas.borrow_mut();
                assert!(c.pinned);
                let stroke = &c.editor.elements()[0];
                assert_eq!(
                    stroke.text.as_ref().unwrap().content(),
                    value.replace("\r\n", "\n")
                );
                c.selected = Some((0, c.editor.revision()));
                text_input::Pending::selected(&c).unwrap()
            };
            // Exercise the real inline transaction, not a direct model replacement.
            text_input::inline::begin(doc.window.0, &doc.state, pending).unwrap();
            let edit = GetFocus();
            assert!(!edit.is_invalid());
            let mut class = [0_u16; 16];
            let len = GetClassNameW(edit, &mut class);
            assert_eq!(
                String::from_utf16(&class[..len as usize])
                    .unwrap()
                    .to_uppercase(),
                "EDIT"
            );
            let changed = "\tfn 编辑() {\r\n    return \"🦦\";\r\n}";
            SetWindowTextW(edit, &windows::core::HSTRING::from(changed)).unwrap();
            text_input::inline::finish(doc.window.0, &doc.state, true).unwrap();
            let (geometry, fill) = {
                let c = doc.state.canvas.borrow();
                let s = &c.editor.elements()[0];
                assert_eq!(
                    s.text.as_ref().unwrap().content(),
                    changed.replace("\r\n", "\n")
                );
                (s.text.as_ref().unwrap().geometry().unwrap(), s.fill)
            };
            settle(&mut app);
            assert!(dir.path().is_file());
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&dir.0).unwrap()), true)
                .unwrap();
            let c = reopened
                .documents
                .iter()
                .find(|d| d.state.unit_id == id)
                .unwrap()
                .state
                .canvas
                .borrow();
            let [stroke] = c.editor.elements() else {
                panic!("expected a single restored text layer")
            };
            assert_eq!(stroke.tool, crate::editor::Tool::Text);
            let text = stroke.text.as_ref().unwrap();
            assert_eq!(text.content(), changed.replace("\r\n", "\n"));
            assert_eq!(text.geometry(), Some(geometry));
            assert_eq!(stroke.fill, fill);
        });
    });
}

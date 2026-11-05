//! 实际Application标注替换后手动保存、后台Checkpoint与同profile重开，不冒充网络协作接通。
use super::*;
use crate::viewport::Point;
use serde_json::{json, Value};

fn document(path: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn expected_points(app: &Application) -> Value {
    // 独立oracle：原生图像点先到窗口DIP，再到会话局部单位；不调用被测writer/Mapping。
    // 普通工作区有20DIP视口留白，不能把图像坐标直接当成canonical坐标。
    let document = &app.documents[0];
    let view = document.state.canvas.borrow().viewport;
    let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(document.window.0) };
    let space = app.restored_session.as_ref().unwrap().space.unwrap();
    let factor = dpi as f64 / 96.0 / space.scale;
    Value::Array(
        [(2.0, 2.0), (2.0, 20.0), (20.0, 20.0)]
            .into_iter()
            .map(|(x, y)| {
                json!({"x":(x * view.scale as f64 + view.origin.x as f64) * factor,
            "y":(y * view.scale as f64 + view.origin.y as f64) * factor})
            })
            .collect(),
    )
}
fn check(value: &Value, expected: &Value) {
    let objects = value["stickers"][0]["annotationState"]["elements"]
        .as_array()
        .unwrap();
    assert_eq!(objects.len(), 2);
    for (index, kind) in ["line", "arrow"].iter().enumerate() {
        assert_eq!(objects[index]["type"], *kind);
        assert_eq!(objects[index]["id"], format!("stable-{kind}"));
        assert_eq!(&objects[index]["points"], expected);
    }
}
#[test]
fn annotation_only_replacement_survives_real_save_checkpoint_reopen_and_undo_without_baking() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            let original = app.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .clone();
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let objects: Vec<_> = ["line", "arrow"]
                .iter()
                .enumerate()
                .map(|(z, kind)| {
                    json!({
                "id":format!("stable-{kind}"),"type":kind,"zIndex":z,
                "points":[{"x":2,"y":2},{"x":2,"y":20},{"x":20,"y":20}],
                "style":{"width":2,"color":"#ff0000"}})
                })
                .collect();
            let raw =
                serde_json::value::to_raw_value(&json!({"elements":objects,"serialCounter":0}))
                    .unwrap();
            let source = crate::session_vectors::restore(Some(&raw)).unwrap().editor;
            {
                let mut canvas = app.documents[0].state.canvas.borrow_mut();
                let prepared = canvas
                    .editor
                    .prepare_annotation_replacement(&source)
                    .unwrap();
                assert!(prepared.apply(&mut canvas.editor).unwrap());
            }
            let expected = expected_points(&app);
            settle(&mut app);
            check(&document(&directory.path()), &expected);
            let manual = directory.0.join("manual-path.json");
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(manual.clone()),
            )
            .unwrap();
            let saved = document(&manual);
            check(&saved, &expected);
            let source = crate::desktop::session_image::inline(
                saved["stickers"][0]["src"].as_str().unwrap(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(
                source.pixels, original.pixels,
                "vector paths must not be baked into base"
            );
            let fixed = std::fs::read(&manual).unwrap();
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            {
                let mut canvas = reopened.documents[0].state.canvas.borrow_mut();
                assert_eq!(canvas.editor.elements().len(), 2);
                assert_eq!(canvas.editor.elements()[1].points.len(), 3);
                assert_eq!(canvas.editor.elements()[1].annotation_id(), "stable-arrow");
                assert!(canvas
                    .editor
                    .translate(1, Point { x: 2.0, y: 3.0 })
                    .unwrap());
                assert!(canvas.editor.undo());
            }
            settle(&mut reopened);
            check(&document(&directory.path()), &expected);
            assert_eq!(std::fs::read(&manual).unwrap(), fixed);
            assert!(!IsWindowVisible(reopened.documents[0].window.0).as_bool());
            drop(reopened);
        });
    }
}

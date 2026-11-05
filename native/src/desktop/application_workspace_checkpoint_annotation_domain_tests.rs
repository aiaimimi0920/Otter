//! 隔离Application手动Save/Checkpoint/同profile重开，核扩域数据与原图不烘焙。
use super::*;
use crate::projection_edit::{
    self,
    view::{Size, View},
};
use serde_json::{json, Value};
fn metrics(
    text: &crate::text_annotation::TextAnnotation,
) -> std::result::Result<crate::session_vectors::TextMetrics, crate::session_vectors::Error> {
    unsafe {
        Ok(crate::session_vectors::TextMetrics {
            size: crate::desktop::annotation_text::measure(text)
                .map_err(|_| crate::session_vectors::Error::Geometry)?,
            baseline: crate::desktop::annotation_text::measure_flip_metrics(text)
                .map_err(|_| crate::session_vectors::Error::Geometry)?
                .baseline_from_top,
        })
    }
}
fn source() -> View {
    let values = [
        json!({"id":"polygon","type":"polygon","zIndex":-1.125,"x":2,"y":2,"w":20,"h":20,"sides":64,"rotation":36000,"style":{"color":"transparent","width":8192,"cornerRadius":8192}}),
        json!({"id":"text","type":"text","zIndex":0.125,"x":4,"y":20,"text":"A\0B\r\nC","fontSize":8192,"fontFamily":"中".repeat(128),"style":{"color":"transparent","width":0}}),
        json!({"id":"serial","type":"serial","zIndex":0.125,"x":4,"y":20,"text":"7","fontSize":18,"style":{"color":"transparent","width":0,"cornerRadius":14.25}}),
    ];
    View::from_basis(
        &projection_edit::Basis::new(&"a".repeat(64), 32, 32).unwrap(),
        Size::new(32.0, 32.0).unwrap(),
        values
            .into_iter()
            .map(|v| (v["id"].as_str().unwrap().to_owned(), v))
            .collect(),
    )
    .unwrap()
}
fn document(path: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn check(value: &Value, factor: f64) {
    let objects = value["stickers"][0]["annotationState"]["elements"]
        .as_array()
        .unwrap();
    assert_eq!(objects.len(), 3);
    let object = |id| objects.iter().find(|v| v["id"] == id).unwrap();
    assert_eq!(object("polygon")["zIndex"], -1.125);
    assert_eq!(object("text")["zIndex"], 0.125);
    assert_eq!(object("serial")["zIndex"], 0.125);
    assert_eq!(object("polygon")["sides"], 64);
    assert_eq!(object("polygon")["rotation"], 36000.0);
    for (id, key, n) in [
        ("polygon", "width", 8192.0),
        ("polygon", "cornerRadius", 8192.0),
        ("serial", "cornerRadius", 14.25),
    ] {
        assert!((object(id)["style"][key].as_f64().unwrap() - n * factor).abs() < 0.001);
    }
    assert_eq!(object("text")["text"], "A\0B\r\nC");
    assert_eq!(object("text")["fontFamily"], "中".repeat(128));
    assert!((object("text")["fontSize"].as_f64().unwrap() - 8192.0 * factor).abs() < 0.001);
}
#[test]
fn wire_adapter_wide_annotations_survive_save_checkpoint_reopen_without_baking_or_identity_loss() {
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
            let wire = source();
            let bridge = {
                let mut canvas = app.documents[0].state.canvas.borrow_mut();
                let prepared =
                    projection_edit::editor::prepare(&wire, &canvas.editor, metrics).unwrap();
                let bridge = prepared.apply(&mut canvas.editor).unwrap().unwrap();
                assert_eq!(bridge.current(&canvas.editor, metrics).unwrap(), wire);
                assert!(canvas.editor.resize(0, 0.5).unwrap());
                assert!(canvas.editor.undo());
                assert_eq!(bridge.current(&canvas.editor, metrics).unwrap(), wire);
                bridge
            };
            let view = app.documents[0].state.canvas.borrow().viewport;
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(app.documents[0].window.0);
            let factor = view.scale as f64 * dpi as f64
                / 96.0
                / app.restored_session.as_ref().unwrap().space.unwrap().scale;
            settle(&mut app);
            check(&document(&directory.path()), factor);
            let manual = directory.0.join("manual-domain.json");
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(manual.clone()),
            )
            .unwrap();
            let saved = document(&manual);
            check(&saved, factor);
            let base = crate::desktop::session_image::inline(
                saved["stickers"][0]["src"].as_str().unwrap(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(base.pixels, original.pixels);
            let fixed = std::fs::read(&manual).unwrap();
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            {
                let canvas = reopened.documents[0].state.canvas.borrow();
                assert_eq!(canvas.editor.elements().len(), 3);
                assert_eq!(
                    bridge.current(&canvas.editor, metrics),
                    Err(projection_edit::editor::Error::Identity)
                );
                let text = canvas
                    .editor
                    .elements()
                    .iter()
                    .find(|s| s.annotation_id() == "text")
                    .unwrap()
                    .text
                    .as_ref()
                    .unwrap();
                assert_eq!(text.content(), "A\0B\r\nC");
                assert_eq!(text.family(), "中".repeat(128));
            }
            settle(&mut reopened);
            check(&document(&directory.path()), factor);
            assert_eq!(std::fs::read(&manual).unwrap(), fixed);
            assert!(!IsWindowVisible(reopened.documents[0].window.0).as_bool());
        });
    }
}

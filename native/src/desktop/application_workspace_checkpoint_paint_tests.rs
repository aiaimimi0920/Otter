//! 真实后台保存/Checkpoint和同profile重开：标注alpha独立于未烘焙底图与整体opacity。
use super::*;
use crate::{
    editor::Tool,
    stroke_style::{Color, PaintAlpha, StrokeStyle},
    viewport::Point,
};
use serde_json::Value;

fn document(path: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn check(value: &Value, id: &str) {
    let object = &value["stickers"][0]["annotationState"]["elements"][0];
    assert_eq!(object["id"], id);
    assert_eq!(object["style"]["color"], "rgba(255,0,0,0.25)");
    assert_eq!(object["style"]["fill"], "rgba(0,0,255,0.625)");
    assert_eq!(object["style"]["opacity"], 0.5);
}
#[test]
fn manual_save_checkpoint_reopen_and_undo_do_not_bake_or_compound_color_alpha() {
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
            let alpha = PaintAlpha::new(0.25, 0.625).unwrap();
            let id = {
                let mut canvas = app.documents[0].state.canvas.borrow_mut();
                canvas.editor.set_style(
                    StrokeStyle::new(4.0, Color(255, 0, 0))
                        .unwrap()
                        .with_fill(Some(Color(0, 0, 255)))
                        .with_opacity(0.5)
                        .unwrap()
                        .with_paint_alpha(alpha),
                );
                canvas
                    .editor
                    .begin(Tool::Rectangle, Point { x: 2.0, y: 2.0 })
                    .unwrap();
                canvas.editor.extend(Point { x: 20.0, y: 20.0 }).unwrap();
                assert!(canvas.editor.commit());
                canvas.editor.elements()[0].annotation_id().to_owned()
            };
            settle(&mut app);
            check(&document(&directory.path()), &id);
            let manual = directory.0.join("manual-paint.json");
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(manual.clone()),
            )
            .unwrap();
            let saved = document(&manual);
            check(&saved, &id);
            let source = crate::desktop::session_image::inline(
                saved["stickers"][0]["src"].as_str().unwrap(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(
                (source.width, source.height),
                (original.width, original.height)
            );
            assert_eq!(
                source.pixels, original.pixels,
                "canonical source must not bake vector alpha"
            );
            let fixed = std::fs::read(&manual).unwrap();
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            {
                let mut canvas = reopened.documents[0].state.canvas.borrow_mut();
                assert_eq!(canvas.editor.elements().len(), 1);
                assert_eq!(canvas.editor.elements()[0].annotation_id(), id);
                assert_eq!(canvas.editor.elements()[0].paint_alpha, alpha);
                assert_eq!(canvas.editor.elements()[0].opacity, 0.5);
                let style = canvas.editor.elements()[0].style();
                assert!(canvas.editor.restyle(0, style.with_width(8.0).unwrap()));
                assert!(canvas.editor.undo());
                assert_eq!(canvas.editor.elements()[0].paint_alpha, alpha);
            }
            settle(&mut reopened);
            check(&document(&directory.path()), &id);
            assert_eq!(
                std::fs::read(&manual).unwrap(),
                fixed,
                "checkpoint must not overwrite manual file"
            );
            assert!(!IsWindowVisible(reopened.documents[0].window.0).as_bool());
            drop(reopened);
        });
    }
}

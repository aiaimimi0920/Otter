//! 正式输出身份独立于Editor：原保存/Checkpoint追踪提交，不持有预览或额外像素副本。
use super::*;
use crate::desktop::{image::Image, loom_output_batch::Batch};
use crate::surface_javascript_wire::result::{Resolved, ResolvedOutput};
use serde_json::{json, Value};

fn image(bgra: [u8; 4]) -> Image {
    Image {
        width: 1,
        height: 1,
        pixels: bgra.to_vec().into(),
    }
}
fn commit(app: &Application, color: Option<[u8; 4]>, mode: Option<&str>) {
    let mut outputs = std::collections::BTreeMap::new();
    if let Some(color) = color {
        outputs.insert("output_image".into(), ResolvedOutput::Image(image(color)));
    }
    if let Some(mode) = mode {
        outputs.insert("mode".into(), ResolvedOutput::Value(json!(mode)));
    }
    let mut results = app.documents[0].state.execution_results.borrow_mut();
    let stamp = results.output_stamp();
    results
        .apply_local_result(&stamp, Resolved { outputs })
        .unwrap();
}
fn checkpoint(directory: &Directory) -> Value {
    serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap()
}
fn check_image(document: &Value, expected: [u8; 4]) {
    let encoded = document["stickers"][0]["outputs"]["output_image"]
        .as_str()
        .unwrap();
    let decoded = crate::desktop::session_image::inline(encoded, &AtomicBool::new(false)).unwrap();
    assert_eq!((decoded.width, decoded.height), (1, 1));
    assert_eq!(decoded.pixels.as_ref(), expected);
}
#[test]
fn formal_outputs_dirty_manual_and_automatic_save_while_preview_does_not_and_clear_survives_reopen()
{
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let editor = app.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .document_stamp();
            let original = app.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .pixels
                .clone();
            commit(&app, Some([0, 0, 255, 255]), Some("red"));
            settle(&mut app);
            let manual = directory.0.join("manual.json");
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(manual.clone()),
            )
            .unwrap();
            let manual_bytes = std::fs::read(&manual).unwrap();
            settle(&mut app);
            let red = checkpoint(&directory);
            check_image(&red, [0, 0, 255, 255]);
            assert_eq!(app.save_status(), save_state::Status::Unchanged);
            let saved = save_state::Stamp::capture(&app).unwrap();
            let request = crate::graph_requests::Request {
                id: crate::workspace_id::UnitId::parse("preview-request").unwrap(),
                generation: 1,
            };
            {
                let mut results = app.documents[0].state.execution_results.borrow_mut();
                results.begin(request.clone());
                assert!(results
                    .preview(
                        &request,
                        1,
                        Batch::local_image(image([255, 0, 0, 255]).into()).unwrap()
                    )
                    .unwrap());
            }
            assert!(saved.matches_automatic(&app));
            assert_eq!(app.save_status(), save_state::Status::Unchanged);
            assert!(app.documents[0]
                .state
                .execution_results
                .borrow_mut()
                .cancel(&request));
            settle(&mut app);
            assert_eq!(
                checkpoint(&directory),
                red,
                "preview/cancel must not publish a checkpoint"
            );
            commit(&app, Some([255, 0, 0, 255]), Some("blue"));
            assert!(editor.matches(&app.documents[0].state.canvas.borrow().editor));
            assert!(
                !saved.matches_automatic(&app),
                "formal ports changed without any Editor change"
            );
            assert_eq!(app.save_status(), save_state::Status::Changed);
            settle(&mut app);
            let blue = checkpoint(&directory);
            assert_eq!(
                blue["documentRevision"].as_u64(),
                Some(red["documentRevision"].as_u64().unwrap() + 1)
            );
            check_image(&blue, [255, 0, 0, 255]);
            assert_eq!(blue["stickers"][0]["outputs"]["mode"], "blue");
            commit(&app, None, Some("value-only"));
            settle(&mut app);
            assert_eq!(
                checkpoint(&directory)["stickers"][0]["outputs"],
                json!({"mode":"value-only"})
            );
            commit(&app, None, None);
            settle(&mut app);
            let empty = checkpoint(&directory);
            assert_eq!(empty["stickers"][0]["outputs"], json!({}));
            assert_eq!(
                empty["documentRevision"].as_u64(),
                Some(blue["documentRevision"].as_u64().unwrap() + 2)
            );
            assert!(original.same_storage(
                &app.documents[0]
                    .state
                    .canvas
                    .borrow()
                    .image
                    .as_ref()
                    .unwrap()
                    .pixels
            ));
            assert_eq!(std::fs::read(&manual).unwrap(), manual_bytes);
            drop(app);
            let mut restored = new_app(instance);
            restored
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert!(restored.documents[0]
                .state
                .execution_results
                .borrow()
                .ports()
                .is_empty());
            settle(&mut restored);
            assert_eq!(
                checkpoint(&directory),
                empty,
                "reopen must not resurrect old formal ports"
            );
            drop(restored);
        });
    }
}
#[test]
fn formal_result_replacement_rejects_inflight_old_checkpoint_and_publishes_only_current_ports() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            let baseline = std::fs::read(directory.path()).unwrap();
            commit(&app, Some([0, 0, 255, 255]), Some("red"));
            app.start_checkpoint().unwrap();
            ready(&app);
            commit(&app, Some([255, 0, 0, 255]), Some("blue"));
            app.poll_checkpoint().unwrap();
            assert_eq!(
                std::fs::read(directory.path()).unwrap(),
                baseline,
                "stale red candidate was published"
            );
            settle(&mut app);
            let blue = checkpoint(&directory);
            check_image(&blue, [255, 0, 0, 255]);
            assert_eq!(blue["stickers"][0]["outputs"]["mode"], "blue");
            commit(&app, Some([0, 255, 0, 255]), Some("cancelled-green"));
            app.start_checkpoint().unwrap();
            ready(&app);
            app.exit_requested.set(true);
            app.poll_checkpoint().unwrap();
            assert_eq!(
                checkpoint(&directory),
                blue,
                "cancelled checkpoint must not publish"
            );
            app.exit_requested.set(false);
            drop(app);
        });
    }
}

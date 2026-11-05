//! 真实Application图输入→签名child→Canvas→正式输出→保存恢复，源截图保持不变。
use super::*;
pub(super) const DRAW: &str = r#"
NeuroSurface.define({mount({root,snapshot}) {
 const input=snapshot.state.inputs.input_image,im=NeuroSurface.image(input.resourceId);
 if(im.width!==input.width||im.height!==input.height)throw Error('dimensions');
 const c=document.createElement('canvas');c.id='processed';c.width=im.width;c.height=im.height;
 root.append(c);c.getContext('2d').drawImage(im,0,0);
 NeuroSurface.commitResult({output_image:{kind:'canvas',nodeId:'processed'}});
}});
"#;
#[test]
fn art_draws_current_crop_to_formal_image_and_session_roundtrip_without_mutating_source() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = colored_app();
        let source = app.documents[0]
            .state
            .canvas
            .borrow()
            .image
            .as_ref()
            .unwrap()
            .pixels
            .clone();
        app.documents[0]
            .state
            .canvas
            .borrow_mut()
            .editor
            .crop_to(crate::crop_geometry::Rect {
                x: 1.,
                y: 0.,
                width: 2.,
                height: 2.,
            })
            .unwrap();
        let stamp = app.documents[0]
            .state
            .canvas
            .borrow()
            .editor
            .document_stamp();
        let peer = begin_with(&mut app, DRAW);
        settle_local(&mut app, (1, 1));
        peer.finish();
        let target = app.documents[1].state.unit_id.clone();
        let expected = [
            0, 255, 0, 255, 255, 0, 0, 255, 0, 128, 128, 128, 255, 0, 255, 255,
        ];
        let image = app.documents[1]
            .state
            .execution_results
            .borrow()
            .image_source("output_image")
            .unwrap();
        assert_eq!((image.image.width, image.image.height), (2, 2));
        assert_eq!(image.image.pixels.as_ref(), expected);
        let canvas = app.documents[0].state.canvas.borrow();
        assert!(source.same_storage(&canvas.image.as_ref().unwrap().pixels));
        assert!(stamp.matches(&canvas.editor));
        drop(canvas);
        let (_, snapshot) = app.capture_session_snapshot().unwrap();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let encoded = snapshot.encode(&cancel).unwrap();
        let restored =
            crate::desktop::session_candidate::prepare(encoded.bytes.to_vec(), None, &cancel)
                .unwrap();
        let results = restored.prepare_results(&cancel).unwrap();
        assert_eq!(
            results
                .iter()
                .find(|r| r.unit() == &target)
                .unwrap()
                .image_source("output_image")
                .unwrap()
                .image
                .pixels
                .as_ref(),
            expected
        );
        app.shutdown();
    }
}
#[test]
fn art_canvas_queued_pixels_are_rejected_after_source_or_workspace_retirement() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for workspace in [false, true] {
            let mut app = colored_app();
            let peer = begin_with(&mut app, DRAW);
            let deadline = Instant::now() + Duration::from_secs(5);
            while app.local_arts.test_counts() != (1, 1) {
                app.local_arts.poll(
                    &app.documents,
                    &mut app.workspace,
                    &app.live_sessions,
                    false,
                );
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            peer.finish();
            if workspace {
                app.local_arts.reset();
            } else {
                app.documents[0]
                    .state
                    .canvas
                    .borrow_mut()
                    .editor
                    .crop_to(crate::crop_geometry::Rect {
                        x: 0.,
                        y: 0.,
                        width: 1.,
                        height: 1.,
                    })
                    .unwrap();
            }
            let (changed, error) =
                app.local_arts
                    .apply_results(&app.documents, &app.workspace, &app.live_sessions);
            assert!(changed.is_empty() && error.is_none());
            assert!(app.documents[1]
                .state
                .execution_results
                .borrow()
                .image_source("output_image")
                .is_none());
            settle_local(&mut app, (0, 0));
            app.shutdown();
        }
    }
}

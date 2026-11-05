//! 实际Application→签名child读取上游PNG；预期像素手工构造，不用资源适配器生成oracle。
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
#[path = "application_local_art_canvas_tests.rs"]
mod canvas_tests;
const READ: &str = r#"
NeuroSurface.define({mount({root,snapshot,resources}) {
 const input=snapshot.state.inputs.input_image;
 if (!input || input.kind!=='image' || input.mimeType!=='image/png') throw Error('missing image input');
 const uri=NeuroSurface.resource(input.resourceId);
 if (!uri || uri!==resources[input.resourceId] || uri!==NeuroSurface.resource(NeuroSurface.snapshot().state.inputs.input_image.resourceId)) throw Error('resource mismatch');
 const text=document.createElement('span');text.textContent=`${input.width}x${input.height}`;root.appendChild(text);
 NeuroSurface.commitResult({observed:{kind:'value',value:{width:input.width,height:input.height,uri}}});
}});
"#;
unsafe fn colored_app() -> Application {
    let instance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
        .unwrap()
        .into();
    let image = crate::desktop::image::Image {
        width: 4,
        height: 2,
        pixels: vec![
            0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 128,
            128, 128, 255, 0, 255, 255, 0, 255, 255, 255,
        ]
        .into(),
    };
    let document =
        DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false).unwrap();
    Application::new(instance, document)
}
fn begin(app: &mut Application) -> peer::Peer {
    begin_with(app, READ)
}
fn begin_with(app: &mut Application, source: &'static str) -> peer::Peer {
    let peer = peer::Peer::with_source(source, true);
    peer.authorize(&mut app.authorization_owner);
    app.art_issuers = peer.authority();
    let source = app.documents[0].state.unit_id.to_string();
    app.open_local_art(
        &peer.origin,
        "vendor/application".into(),
        &source,
        peer.config(),
    )
    .unwrap();
    peer
}
#[test]
fn child_reads_actual_committed_crop_without_changing_original_or_canonical() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = colored_app();
        let original = app.documents[0]
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
                x: 1.0,
                y: 0.0,
                width: 2.0,
                height: 2.0,
            })
            .unwrap();
        let source_stamp = app.documents[0]
            .state
            .canvas
            .borrow()
            .editor
            .document_stamp();
        let peer = begin(&mut app);
        settle_local(&mut app, (1, 1));
        peer.finish();
        let value = app.documents[1]
            .state
            .execution_results
            .borrow()
            .save_value_outputs()
            .unwrap()["observed"]
            .clone();
        assert_eq!(
            (value["width"].as_u64(), value["height"].as_u64()),
            (Some(2), Some(2))
        );
        let bytes = STANDARD
            .decode(
                value["uri"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("data:image/png;base64,")
                    .unwrap(),
            )
            .unwrap();
        let image = crate::desktop::image::Image::from_encoded(&bytes).unwrap();
        assert_eq!((image.width, image.height), (2, 2));
        assert_eq!(
            image.pixels.as_ref(),
            &[0, 255, 0, 255, 255, 0, 0, 255, 0, 128, 128, 128, 255, 0, 255, 255]
        );
        let canvas = app.documents[0].state.canvas.borrow();
        assert!(original.same_storage(&canvas.image.as_ref().unwrap().pixels));
        assert!(source_stamp.matches(&canvas.editor));
        assert!(app.documents[0]
            .state
            .execution_results
            .borrow()
            .save_value_outputs()
            .is_none());
        drop(canvas);
        app.shutdown();
    }
}
#[test]
fn source_edit_or_reset_rejects_queued_child_result_but_unrelated_node_does_not() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for change in [0, 1, 2, 3] {
            let mut app = colored_app();
            let peer = begin(&mut app);
            let deadline = Instant::now() + Duration::from_secs(10);
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
            if change == 1 {
                app.documents[0]
                    .state
                    .canvas
                    .borrow_mut()
                    .editor
                    .crop_to(crate::crop_geometry::Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 2.0,
                        height: 1.0,
                    })
                    .unwrap();
            } else if change == 2 {
                app.local_arts.reset();
            } else if change == 3 {
                app.documents[0]
                    .state
                    .canvas
                    .borrow_mut()
                    .editor
                    .begin(
                        crate::editor::Tool::Rectangle,
                        crate::viewport::Point { x: 0.0, y: 0.0 },
                    )
                    .unwrap();
            } else {
                app.workspace
                    .add_units([crate::workspace_id::UnitId::parse("unrelated-image").unwrap()])
                    .unwrap();
            }
            let (changed, error) =
                app.local_arts
                    .apply_results(&app.documents, &app.workspace, &app.live_sessions);
            assert!(error.is_none());
            assert_eq!(changed.is_empty(), change != 0);
            assert_eq!(
                app.documents[1]
                    .state
                    .execution_results
                    .borrow()
                    .save_value_outputs()
                    .is_none(),
                change != 0
            );
            if change != 0 {
                settle_local(&mut app, (0, 0));
            }
            app.shutdown();
        }
    }
}
#[test]
fn formal_output_is_used_instead_of_preview_and_changed_output_rejects_queued_result() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for change_output in [false, true] {
            let mut app = colored_app();
            let source = app.documents[0].state.unit_id.to_string();
            let first = peer::Peer::with_source(
                r#"NeuroSurface.define({mount({root}) {
                const c=document.createElement('canvas');c.id='formal';c.width=2;c.height=1;root.appendChild(c);
                c.getContext('2d').fillStyle='red';c.getContext('2d').fillRect(0,0,2,1);
                NeuroSurface.commitResult({output_image:{kind:'canvas',nodeId:'formal'}});
            }});"#,
                true,
            );
            first.authorize(&mut app.authorization_owner);
            app.art_issuers = first.authority();
            app.open_local_art(
                &first.origin,
                "vendor/application".into(),
                &source,
                first.config(),
            )
            .unwrap();
            settle_local(&mut app, (1, 1));
            first.finish();
            let upstream = app.documents[1].state.unit_id.clone();
            let peer = peer::Peer::with_art_source("vendor/consumer", READ, true);
            peer.authorize(&mut app.authorization_owner);
            app.art_issuers = peer.authority();
            app.open_local_art(
                &peer.origin,
                "vendor/consumer".into(),
                upstream.as_str(),
                peer.config(),
            )
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
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
            if change_output {
                *app.documents[1].state.execution_results.borrow_mut() =
                    crate::desktop::loom_result_state::Results::new(upstream);
            }
            let (changed, error) =
                app.local_arts
                    .apply_results(&app.documents, &app.workspace, &app.live_sessions);
            assert!(error.is_none());
            assert_eq!(changed.is_empty(), change_output);
            if !change_output {
                let value = app.documents[2]
                    .state
                    .execution_results
                    .borrow()
                    .save_value_outputs()
                    .unwrap()["observed"]
                    .clone();
                let bytes = STANDARD
                    .decode(
                        value["uri"]
                            .as_str()
                            .unwrap()
                            .strip_prefix("data:image/png;base64,")
                            .unwrap(),
                    )
                    .unwrap();
                let image = crate::desktop::image::Image::from_encoded(&bytes).unwrap();
                assert_eq!((image.width, image.height), (2, 1));
                assert_eq!(image.pixels.as_ref(), &[0, 0, 255, 255, 0, 0, 255, 255]);
            }
            app.shutdown();
        }
    }
}
#[test]
fn image_larger_than_inline_gate_draws_at_native_resolution_without_changing_source() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let instance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
            .unwrap()
            .into();
        let mut noise = Vec::with_capacity(512 * 512 * 4);
        let mut seed = 0x1234abcd_u32;
        for _ in 0..512 * 512 {
            for _ in 0..3 {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                noise.push((seed >> 24) as u8);
            }
            noise.push(255);
        }
        let image = crate::desktop::image::Image {
            width: 512,
            height: 512,
            pixels: noise.into(),
        };
        let original = image.pixels.clone();
        let document =
            DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false)
                .unwrap();
        let mut app = Application::new(instance, document);
        let peer = begin_with(&mut app, canvas_tests::DRAW);
        settle_local(&mut app, (1, 1));
        peer.finish();
        let result = app.documents[1]
            .state
            .execution_results
            .borrow()
            .image_source("output_image")
            .unwrap();
        assert_eq!((result.image.width, result.image.height), (512, 512));
        assert_eq!(result.image.pixels.as_ref(), original.as_ref());
        // 高熵PNG确实大于JSON资源门；不是压缩极小的纯色图冒充大图传输。
        assert!(
            result
                .image
                .png(&std::sync::atomic::AtomicBool::new(false))
                .unwrap()
                .len()
                > crate::surface_javascript::MAX_RESOURCES_BYTES
        );
        let canvas = app.documents[0].state.canvas.borrow();
        assert!(original.same_storage(&canvas.image.as_ref().unwrap().pixels));
        assert!(!canvas.editor.has_edit_state());
        drop(canvas);
        app.shutdown();
    }
}

//! Live→独立 Local Art 经真实签名包和 child 输入资源；只用自有合成帧/回环 TLS。
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
const READ: &str = r#"
NeuroSurface.define({mount({snapshot}) {
 const input=snapshot.state.inputs.input_image;
 if (!input || input.kind!=='image') throw Error('missing Live image');
 NeuroSurface.commitResult({observed:{kind:'value',value:{width:input.width,height:input.height,uri:NeuroSurface.resource(input.resourceId)}}});
}});
"#;
#[test]
fn local_child_reads_live_pixels_via_signed_chain_and_rejects_result_after_source_close() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for close in [false, true] {
            let mut app = app();
            let (hwnd, unit) =
                crate::desktop::application::live::freeze::tests::live(&mut app, true);
            let peer = peer::Peer::with_source(READ, true);
            peer.authorize(&mut app.authorization_owner);
            app.art_issuers = peer.authority();
            app.open_local_art_child(
                &peer.origin,
                "vendor/application".into(),
                unit.as_str(),
                peer.config(),
                |_| true,
            )
            .unwrap();
            assert_eq!(app.documents.len(), 2);
            assert_eq!(app.workspace.links()[0].from_unit, unit);
            assert_eq!(app.workspace.links()[0].to_port, "input_image");
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
            if close {
                DestroyWindow(hwnd).unwrap();
            }
            app.process_local_arts(false);
            let outputs = app.documents[1]
                .state
                .execution_results
                .borrow()
                .save_value_outputs();
            if close {
                assert!(outputs.is_none());
            } else {
                let value = &outputs.unwrap()["observed"];
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
                assert_eq!(image.pixels.as_ref(), &[0, 0, 255, 255].repeat(4));
                assert!(app.live_sessions[0].is_alive());
            }
            app.shutdown();
        }
    }
}

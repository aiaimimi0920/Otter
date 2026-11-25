//! Real hidden Application: signed-child committed results enter existing ports/save/restore without overwriting the initiating screenshot.
use super::*;
use crate::art_execution_grant::package::ports::{Port, Ports};
const RESULT_SOURCE: &str = r#"
NeuroSurface.define({mount({root}) {
 const c=document.createElement('canvas');c.id='result';c.width=2;c.height=1;root.appendChild(c);
 const x=c.getContext('2d');x.fillStyle='red';x.fillRect(0,0,2,1);
 NeuroSurface.commitResult({output:{kind:'canvas',nodeId:'result'},data:{kind:'value',value:null}});
}});
"#;
#[test]
fn local_art_formal_result_enters_session_save_restore_but_image_host_is_not_a_target() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for art_target in [true, false] {
            let mut app = image_app();
            if art_target {
                restore_local_art(&mut app);
            }
            let node = app.documents[0].state.unit_id.to_string();
            let pixels = app.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .map(|image| image.pixels.clone());
            // The currently published package includes signed ports; only restored Local Art hosts may receive them.
            let peer = peer::Peer::with_ports(
                RESULT_SOURCE,
                Ports {
                    inputs: vec![],
                    outputs: vec![
                        Port {
                            name: "output".into(),
                            kind: "image".into(),
                        },
                        Port {
                            name: "data".into(),
                            kind: "value".into(),
                        },
                    ],
                },
            );
            peer.authorize(&mut app.authorization_owner);
            app.art_issuers = peer.authority();
            app.start_local_art(
                &peer.origin,
                "vendor/application".into(),
                &node,
                peer.config(),
            )
            .unwrap();
            settle_local(&mut app, if art_target { (1, 1) } else { (0, 0) });
            peer.finish();
            if art_target {
                let results = app.documents[0].state.execution_results.borrow();
                assert!(results.save_value_outputs().unwrap()["data"].is_null());
                assert_eq!(
                    results
                        .image_source("output")
                        .unwrap()
                        .image
                        .pixels
                        .as_ref(),
                    &[0, 0, 255, 255, 0, 0, 255, 255]
                );
                drop(results);
                let (_, snapshot) = app.capture_session_snapshot().unwrap();
                let cancel = std::sync::atomic::AtomicBool::new(false);
                let encoded = snapshot.encode(&cancel).unwrap();
                let restored = crate::desktop::session_candidate::prepare(
                    encoded.bytes.to_vec(),
                    None,
                    &cancel,
                )
                .unwrap();
                let results = restored.prepare_results(&cancel).unwrap();
                assert!(results[0].save_value_outputs().unwrap()["data"].is_null());
                assert_eq!(
                    results[0]
                        .image_source("output")
                        .unwrap()
                        .image
                        .pixels
                        .as_ref(),
                    &[0, 0, 255, 255, 0, 0, 255, 255]
                );
            } else {
                assert_eq!(
                    app.authorization.borrow().status,
                    crate::ui_text::text("integration-art-host-type-changed")
                );
                assert!(app.documents[0]
                    .state
                    .execution_results
                    .borrow()
                    .save_value_outputs()
                    .is_none());
                assert!(pixels.unwrap().same_storage(
                    &app.documents[0]
                        .state
                        .canvas
                        .borrow()
                        .image
                        .as_ref()
                        .unwrap()
                        .pixels
                ));
            }
            app.shutdown();
        }
    }
}

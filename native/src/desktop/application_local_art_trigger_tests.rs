//! 隐藏Application真实签名child→Results→Save；自动回执不等child返回或磁盘保存。
use super::*;
use crate::art_live_trigger::{tests as f, Receipt};
const SOURCE: &str = r#"
NeuroSurface.define({mount({root}) {
 const c=document.createElement('canvas');c.id='result';c.width=2;c.height=1;root.appendChild(c);
},onLiveTrigger(event) {
 const c=document.getElementById('result');const x=c.getContext('2d');x.fillStyle='red';x.fillRect(0,0,2,1);
 NeuroSurface.commitResult({output:{kind:'canvas',nodeId:'result'},data:{kind:'value',value:event.delivery.observation.value}});
}});
"#;
unsafe fn art_app() -> Application {
    let mut app = image_app();
    app.request_restore_input(crate::desktop::session_job::Input::Bytes(
        br#"{"documentSchemaVersion":1,"documentRevision":1,"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},"stickers":[{"id":"trigger-art","type":"art","artId":"vendor/application","x":0,"y":0,"w":200,"h":120}],"links":[]}"#.to_vec()),None,None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.process_restore(false).unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    app
}
#[test]
fn automatic_result_ack_waits_for_actual_results_transaction_and_cancel_preserves_canonical() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for cancel in [false, true] {
            let mut app = art_app();
            let peer = peer::Peer::with_source(SOURCE, true);
            peer.authorize(&mut app.authorization_owner);
            app.art_issuers = peer.authority();
            let node = app.documents[0].state.unit_id.to_string();
            app.start_local_art(
                &peer.origin,
                "vendor/application".into(),
                &node,
                peer.config(),
            )
            .unwrap();
            settle_local(&mut app, (1, 1));
            peer.finish();
            let proofs =
                app.local_arts
                    .viewer_proofs(&app.documents, &app.workspace, &app.live_sessions);
            assert_eq!(proofs.len(), 1);
            let (gate, ticket) = f::ticket(proofs[0].binding().clone());
            app.local_arts.accept_triggers(vec![ticket.clone()]);
            let deadline = Instant::now() + Duration::from_secs(5);
            while !app.local_arts.test_result_ready() {
                assert!(app
                    .local_arts
                    .poll(
                        &app.documents,
                        &mut app.workspace,
                        &app.live_sessions,
                        false
                    )
                    .is_none());
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(ticket.receipt().is_none());
            assert!(app.documents[0]
                .state
                .execution_results
                .borrow()
                .save_value_outputs()
                .is_none());
            if cancel {
                gate.stop();
            }
            app.process_local_arts(false);
            let results = app.documents[0].state.execution_results.borrow();
            if cancel {
                assert!(ticket.receipt().is_none());
                assert!(results.save_value_outputs().is_none());
            } else {
                assert_eq!(
                    ticket.receipt(),
                    Some(Receipt::Applied { formal_revision: 1 })
                );
                assert_eq!(
                    results.save_value_outputs().unwrap()["data"],
                    serde_json::json!({"text":"eight"})
                );
                assert_eq!(
                    results
                        .image_source("output")
                        .unwrap()
                        .image
                        .pixels
                        .as_ref(),
                    [0, 0, 255, 255, 0, 0, 255, 255]
                );
            }
            drop(results);
            if !cancel {
                let (_, snapshot) = app.capture_session_snapshot().unwrap();
                let stop = std::sync::atomic::AtomicBool::new(false);
                let encoded = snapshot.encode(&stop).unwrap();
                let candidate =
                    crate::desktop::session_candidate::prepare(encoded.bytes.to_vec(), None, &stop)
                        .unwrap();
                let restored = candidate.prepare_results(&stop).unwrap();
                assert_eq!(
                    restored[0].save_value_outputs().unwrap()["data"],
                    serde_json::json!({"text":"eight"})
                );
                assert_eq!(
                    restored[0]
                        .image_source("output")
                        .unwrap()
                        .image
                        .pixels
                        .as_ref(),
                    [0, 0, 255, 255, 0, 0, 255, 255]
                );
            }
            app.shutdown();
            assert!(!app.local_arts.busy());
        }
    }
}

#[test]
fn callback_without_result_acknowledges_only_after_hidden_presentation() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        let mut app = art_app();
        let peer=peer::Peer::with_source("let label;NeuroSurface.define({mount({root}){label=document.createElement('span');label.textContent='initial';root.replaceChildren(label)},onLiveTrigger(){label.textContent='done'}});",false);
        peer.authorize(&mut app.authorization_owner);
        app.art_issuers = peer.authority();
        let node = app.documents[0].state.unit_id.to_string();
        app.start_local_art(
            &peer.origin,
            "vendor/application".into(),
            &node,
            peer.config(),
        )
        .unwrap();
        settle_local(&mut app, (1, 1));
        peer.finish();
        let proofs =
            app.local_arts
                .viewer_proofs(&app.documents, &app.workspace, &app.live_sessions);
        let (_gate, ticket) = f::ticket(proofs[0].binding().clone());
        app.local_arts.accept_triggers(vec![ticket.clone()]);
        assert!(ticket.receipt().is_none());
        let deadline = Instant::now() + Duration::from_secs(5);
        while ticket.receipt().is_none() {
            app.process_local_arts(false);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(ticket.receipt(), Some(Receipt::NoResult));
        assert!(app.documents[0]
            .state
            .execution_results
            .borrow()
            .save_value_outputs()
            .is_none());
        app.shutdown();
        assert!(!app.local_arts.busy());
    }
}

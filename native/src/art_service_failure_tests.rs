//! 真实signed child/TLS：暂态错误拒绝Promise但保留本地交互；取消只作用原call，旧结果不发布。
use super::*;
use crate::surface_javascript::fixture as js;
const SOURCE: &str = r#"
let label;
NeuroSurface.define({async mount({root}) {
 label=document.createElement('span');root.append(label);
 try {await NeuroSurface.service('capability:vendor:service.run',{n:1},5000);label.textContent='wrong';}
 catch(error){label.textContent=error.message;}
 const local=document.createElement('button');local.textContent='local';root.append(local);
 local.addEventListener('click',()=>{label.textContent='local-ready';});
 const retry=document.createElement('button');retry.textContent='retry';root.append(retry);
 retry.addEventListener('click',async()=>{
  const value=await NeuroSurface.service('capability:vendor:service.run',{n:2},5000);
  label.textContent=value===42?'retried-once':'wrong';
 });
}});
"#;
#[test]
fn signed_transient_failure_cancels_original_and_only_explicit_retry_starts_new_call() {
    let _serial = js::SERIAL.lock().unwrap();
    for status in [429, 503, 0] {
        let mut first = None;
        let (prepared, client, _device, lease, peer) =
            setup_source_status(3, SOURCE, move |index, head, body, reply| {
                match index {
                    0 => {
                        assert!(head.starts_with("POST /v1/art/local/service/submit "));
                        assert_eq!(body["input"], json!({"n":1}));
                        first = Some(body["owner"].clone());
                        return status;
                    }
                    1 => {
                        assert!(head.starts_with("POST /v1/art/local/service/cancel "));
                        assert_eq!(Some(&body["owner"]), first.as_ref());
                        assert!(body.get("input").is_none());
                        assert!(body.get("timeoutMs").is_none());
                        // ambiguous submit could have completed: its late output must never enter JS.
                        reply["status"] = json!("succeeded");
                        reply["output"] = json!("late-old-output");
                    }
                    _ => {
                        assert!(head.starts_with("POST /v1/art/local/service/submit "));
                        assert_ne!(Some(&body["owner"]), first.as_ref());
                        assert_eq!(body["input"], json!({"n":2}));
                        reply["status"] = json!("succeeded");
                        reply["output"] = json!(42);
                    }
                }
                200
            });
        let binding = prepared.binding().clone();
        let (mut local, frame) = prepared
            .mount(view(client.origin()), js::snapshot(1, 0), true, &|| false)
            .unwrap();
        assert_eq!(
            js::text(&frame.scene),
            match status {
                429 => "service_busy",
                503 => "service_unavailable",
                _ => "service_transport",
            }
        );
        let local_button = frame.scene.root().children()[1].id().to_owned();
        let retry_button = frame.scene.root().children()[2].id().to_owned();
        assert_eq!(
            js::text(
                &local
                    .click(&binding, true, &local_button, &|| false)
                    .unwrap()
                    .scene
            ),
            "local-ready"
        );
        assert_eq!(
            js::text(
                &local
                    .click(&binding, true, &retry_button, &|| false)
                    .unwrap()
                    .scene
            ),
            "retried-once"
        );
        assert!(!lease.is_revoked());
        peer.join().unwrap();
    }
}
#[test]
fn signed_refused_connection_preserves_local_art_without_offline_or_new_authority() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, _device, lease, peer) =
        setup_source(0, SOURCE, |_, _, _, _| unreachable!());
    peer.join().unwrap(); // no listener remains, so submit and bounded cleanup both refuse connection.
    let binding = prepared.binding().clone();
    let (mut local, frame) = prepared
        .mount(view(client.origin()), js::snapshot(1, 0), true, &|| false)
        .unwrap();
    assert_eq!(js::text(&frame.scene), "service_transport");
    assert!(!lease.is_local());
    let target = frame.scene.root().children()[1].id().to_owned();
    assert_eq!(
        js::text(
            &local
                .click(&binding, true, &target, &|| false)
                .unwrap()
                .scene
        ),
        "local-ready"
    );
}
#[test]
fn signed_poll_failure_and_missing_cleanup_task_never_replay_submission() {
    let _serial = js::SERIAL.lock().unwrap();
    let mut owner = None;
    let (prepared, client, _device, _lease, peer) =
        setup_source_status(3, SOURCE, move |index, head, body, _| {
            if index == 0 {
                owner = Some(body["owner"].clone());
                return 200;
            }
            assert_eq!(Some(&body["owner"]), owner.as_ref());
            assert!(head.starts_with(if index == 1 {
                "POST /v1/art/local/service/poll "
            } else {
                "POST /v1/art/local/service/cancel "
            }));
            if index == 1 {
                503
            } else {
                404
            }
        });
    let (_, frame) = prepared
        .mount(view(client.origin()), js::snapshot(1, 0), true, &|| false)
        .unwrap();
    assert_eq!(js::text(&frame.scene), "service_unavailable");
    peer.join().unwrap();
}
#[test]
fn signed_cleanup_timeout_is_bounded_and_does_not_extend_service_or_permit() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, _device, _lease, peer) =
        setup_source_status(2, SOURCE, |index, head, _, _| {
            if index == 0 {
                return 503;
            }
            assert!(head.starts_with("POST /v1/art/local/service/cancel "));
            std::thread::sleep(Duration::from_millis(1200));
            0
        });
    let (_, frame) = prepared
        .mount(view(client.origin()), js::snapshot(1, 0), true, &|| false)
        .unwrap();
    assert_eq!(js::text(&frame.scene), "service_unavailable");
    peer.join().unwrap();
}
#[test]
fn signed_authority_or_protocol_rejection_cannot_be_caught_to_keep_running() {
    let _serial = js::SERIAL.lock().unwrap();
    for status in [401, 403, 500, 200] {
        let (prepared, client, _device, _lease, peer) =
            setup_source_status(1, SOURCE, move |_, _, _, reply| {
                reply["owner"]["binding"]["generation"] = json!(99);
                status
            });
        assert!(prepared
            .mount(view(client.origin()), js::snapshot(1, 0), true, &|| false)
            .is_err());
        peer.join().unwrap();
    }
}
#[test]
fn signed_cleanup_authority_rejection_and_revocation_suppress_caught_failure_frame() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, _device, _lease, peer) = setup_source_status(
        2,
        SOURCE,
        |index, _, _, _| if index == 0 { 503 } else { 403 },
    );
    assert!(prepared
        .mount(view(client.origin()), js::snapshot(1, 0), true, &|| false)
        .is_err());
    peer.join().unwrap();
    let ready = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sent = ready.clone();
    let (prepared, client, _device, lease, peer) =
        setup_source_status(1, SOURCE, move |_, _, _, _| {
            sent.store(true, std::sync::atomic::Ordering::SeqCst);
            503
        });
    assert!(prepared
        .mount(view(client.origin()), js::snapshot(1, 0), true, &|| {
            if ready.load(std::sync::atomic::Ordering::SeqCst) {
                lease.revoke();
            }
            lease.is_revoked()
        })
        .is_err());
    peer.join().unwrap();
}

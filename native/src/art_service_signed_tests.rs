//! 已验签下载→真实受限 JS child→固定 origin TLS 服务；不是实际 Loom daemon 联合验收。
use super::*;
use crate::surface_javascript::fixture as js;
const ASYNC: &str = r#"
let label;
NeuroSurface.define({async mount({root}) {
 label=document.createElement('span'); label.textContent='pending'; root.appendChild(label);
 const result=await NeuroSurface.service('capability:vendor:service.run',{n:1},5000);
 label.textContent=result===null?'null-complete':'wrong';
 const button=document.createElement('button'); button.textContent='run';root.appendChild(button);
 button.addEventListener('click',async()=>{
  try {await NeuroSurface.service('capability:vendor:service.run',{n:2},5000);}
  catch(error){label.textContent=error.message;}
 });
}});
"#;
#[test]
fn art_service_signed_download_child_poll_null_click_failure_and_revocation() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, device, lease, peer) =
        setup_source(4, ASYNC, |index, head, body, response| {
            assert_eq!(body["owner"]["binding"]["workspaceId"], "workspace:1");
            assert_eq!(body["owner"]["grantId"], "grant:download");
            if index == 0 {
                assert!(head.starts_with("POST /v1/art/local/service/submit "));
                assert_eq!(body["input"], json!({"n":1}));
                std::thread::sleep(Duration::from_millis(350));
            } else if index == 1 {
                response["status"] = json!("running");
                response["progress"] = json!({"percent":50});
            } else if index == 2 {
                assert!(head.starts_with("POST /v1/art/local/service/poll "));
                response["status"] = json!("succeeded");
                response["output"] = Value::Null;
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/submit "));
                assert_eq!(body["input"], json!({"n":2}));
                response["status"] = json!("failed");
                response["error"] = json!("service_denied");
            }
        });
    let current = prepared.binding().clone();
    let (mut local, frame) = prepared
        .mount(view(client.origin()), js::snapshot(1, 1), true, &|| false)
        .unwrap();
    assert_eq!(js::text(&frame.scene), "null-complete");
    let target = frame.scene.root().children()[1].id().to_owned();
    let frame = local.click(&current, true, &target, &|| false).unwrap();
    assert_eq!(js::text(&frame.scene), "service_denied");
    peer.join().unwrap();
    drop(device);
    assert!(lease.is_revoked());
    assert!(local.refresh(&current, true, &|| false).is_err());
}
#[test]
fn art_service_signed_scope_and_offline_reject_before_service_network() {
    let _serial = js::SERIAL.lock().unwrap();
    for online in [false, true] {
        let source = if online {
            "NeuroSurface.define({async mount(){await NeuroSurface.service('capability:other:denied.run',{},5000)}});"
        } else {
            ASYNC
        };
        let (prepared, client, _device, _lease, peer) =
            setup_source(0, source, |_, _, _, _| unreachable!());
        peer.join().unwrap();
        assert!(prepared
            .mount(view(client.origin()), js::snapshot(1, 0), online, &|| false)
            .is_err());
    }
}
#[test]
fn art_service_signed_revocation_after_http_never_publishes_a_frame() {
    let _serial = js::SERIAL.lock().unwrap();
    let sent = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ready = sent.clone();
    let (prepared, client, _device, lease, peer) =
        setup_source(1, ASYNC, move |_, _, _, response| {
            sent.store(true, std::sync::atomic::Ordering::SeqCst);
            response["status"] = json!("succeeded");
            response["output"] = Value::Null;
        });
    let result = prepared.mount(view(client.origin()), js::snapshot(1, 0), true, &|| {
        if ready.load(std::sync::atomic::Ordering::SeqCst) {
            lease.revoke();
        }
        lease.is_revoked()
    });
    peer.join().unwrap();
    assert!(result.is_err());
}

#[test]
fn art_service_signed_host_stop_cancels_original_call_without_publishing() {
    let _serial = js::SERIAL.lock().unwrap();
    let sent = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ready = sent.clone();
    let mut owner = None;
    let (prepared, client, _device, lease, peer) =
        setup_source(2, ASYNC, move |index, head, body, response| {
            if index == 0 {
                assert!(head.starts_with("POST /v1/art/local/service/submit "));
                owner = Some(body["owner"].clone());
                sent.store(true, std::sync::atomic::Ordering::SeqCst);
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/cancel "));
                assert_eq!(Some(&body["owner"]), owner.as_ref());
                assert!(body.get("input").is_none());
                assert!(body.get("timeoutMs").is_none());
                response["status"] = json!("cancelled");
            }
        });
    let result = prepared.mount(view(client.origin()), js::snapshot(1, 0), true, &|| {
        ready.load(std::sync::atomic::Ordering::SeqCst)
    });
    peer.join().unwrap();
    assert!(result.is_err());
    assert!(!lease.is_revoked());
}

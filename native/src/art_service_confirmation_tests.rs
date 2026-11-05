//! 真实signed child/TLS与宿主票据；程序化批准仅为夹具，不冒充真实可见用户确认。
use super::*;
use crate::surface_javascript::{fixture as js, signed::runtime::Runtime};
use sha2::{Digest, Sha256};
use std::time::Instant;
const SOURCE: &str = r#"NeuroSurface.define({async mount({root}) {
 const label=document.createElement('span');root.appendChild(label);
 try { label.textContent=String(await NeuroSurface.serviceWithConfirmation('capability:vendor:service.run',{n:7},{timeoutMs:5000})); }
 catch(error) {label.textContent=error.message;}
}});"#;
fn challenge(body: &Value) -> Value {
    let call = &body["call"];
    let hashes = body["images"]
        .as_array()
        .unwrap()
        .iter()
        .map(|image| {
            format!(
                "{:x}",
                Sha256::digest(image["png"].as_str().unwrap().as_bytes())
            )
        })
        .collect::<Vec<_>>();
    let tuple = json!([
        call["input"],
        call["timeoutMs"],
        body["imageOutput"],
        true,
        hashes
    ]);
    let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(&tuple).unwrap()));
    json!({"nonce":"challenge:fixture","fingerprint":fingerprint,"pluginId":"vendor/service","commandId":"vendor/service.run",
        "packageDigest":"a".repeat(64),"scopeId":"scope:fixture","expiresAtMs":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64+5000})
}
fn envelope(response: &mut Value, confirmation: Value) {
    *response = json!({"protocolVersion":"loom.art.local.service.confirmation.v1","reply":response.clone(),"image":null,"confirmation":confirmation});
}
fn ticket(
    runtime: &Runtime,
) -> Arc<crate::live_relay_http::art_prepare::service::confirmation::Ticket> {
    let due = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(ticket) = runtime.take_confirmation(runtime.binding()) {
            return ticket;
        }
        assert!(Instant::now() < due, "missing confirmation");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn art_service_confirmation_signed_waits_for_host_and_never_exposes_token_to_script() {
    let _serial = js::SERIAL.lock().unwrap();
    for approve in [false, true] {
        let mut expected = Value::Null;
        let (prepared, client, _device, _lease, peer) =
            setup_source(2, SOURCE, move |index, head, body, response| {
                if index == 0 {
                    assert!(head.starts_with("POST /v1/art/local/service/submit-confirmable "));
                    assert_eq!(body["call"]["input"], json!({"n":7}));
                    assert!(body["call"].get("userGestureToken").is_none());
                    expected = challenge(body);
                    envelope(response, expected.clone());
                } else {
                    assert!(head.starts_with("POST /v1/art/local/service/confirm "));
                    assert_eq!(body["decision"]["challenge"], expected);
                    assert_eq!(body["decision"]["approve"], approve);
                    response["status"] = json!(if approve { "succeeded" } else { "failed" });
                    if approve {
                        response["output"] = json!(42);
                    } else {
                        response["error"] = json!("local_art_confirmation_denied");
                    }
                    envelope(response, Value::Null);
                }
            });
        let mut runtime =
            Runtime::start(prepared, view(client.origin()), js::snapshot(1, 0), true).unwrap();
        let binding = runtime.binding().clone();
        let pending = ticket(&runtime);
        assert!(pending.text.contains("vendor/service"));
        assert!(pending.text.contains("参数（由 Art 提供）"));
        assert!(runtime.pending());
        assert!(runtime.take_reply(&binding).is_none());
        assert!(runtime.take_confirmation(&binding).is_none());
        assert!(pending.resolve(approve));
        assert!(!pending.resolve(!approve));
        let due = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(frame) = runtime.take_reply(&binding) {
                assert_eq!(
                    js::text(&frame.unwrap().scene),
                    if approve {
                        "42"
                    } else {
                        "local_art_confirmation_denied"
                    }
                );
                break;
            }
            assert!(Instant::now() < due);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!pending.live());
        runtime.request_stop();
        drop(runtime);
        peer.join().unwrap();
    }
}
#[test]
fn art_service_confirmation_stop_invalidates_ticket_and_cancels_original_call() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, _device, _lease, peer) =
        setup_source(2, SOURCE, |index, head, body, response| {
            if index == 0 {
                envelope(response, challenge(body));
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/cancel "));
                response["status"] = json!("cancelled");
            }
        });
    let runtime =
        Runtime::start(prepared, view(client.origin()), js::snapshot(1, 0), true).unwrap();
    let pending = ticket(&runtime);
    runtime.request_stop();
    assert!(!pending.resolve(true));
    drop(runtime);
    peer.join().unwrap();
}
#[test]
fn art_service_confirmation_rejects_forged_input_before_asking_user() {
    let (mut prepared, client, _device, _lease, peer) =
        setup_source(1, SOURCE, |_, _, body, response| {
            let mut bad = challenge(body);
            bad["fingerprint"] = json!("b".repeat(64));
            envelope(response, bad);
        });
    let mut call = Call::new(&mut prepared, &client, SERVICE, json!({"n":7}), 5000).unwrap();
    call.request_confirmation().unwrap();
    assert_eq!(call.submit(), Err(ServiceError::Identity));
    peer.join().unwrap();
}

#[test]
fn art_service_confirmation_remote_scope_failure_retires_unanswered_host_ticket() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, _device, _lease, peer) =
        setup_source(2, SOURCE, |index, head, body, response| {
            if index == 0 {
                envelope(response, challenge(body));
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/poll-confirmable "));
                assert!(body["decision"].is_null());
                response["status"] = json!("failed");
                response["error"] = json!("local_art_command_changed");
                envelope(response, Value::Null);
            }
        });
    let mut runtime =
        Runtime::start(prepared, view(client.origin()), js::snapshot(1, 0), true).unwrap();
    let binding = runtime.binding().clone();
    let pending = ticket(&runtime);
    let due = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(frame) = runtime.take_reply(&binding) {
            assert_eq!(js::text(&frame.unwrap().scene), "local_art_command_changed");
            break;
        }
        assert!(Instant::now() < due);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!pending.live());
    assert!(!pending.resolve(true));
    runtime.request_stop();
    drop(runtime);
    peer.join().unwrap();
}

#[test]
fn art_service_confirmation_unobserved_host_denies_instead_of_auto_approving() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, _device, _lease, peer) =
        setup_source(2, SOURCE, |index, head, body, response| {
            if index == 0 {
                envelope(response, challenge(body));
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/confirm "));
                assert_eq!(body["decision"]["approve"], false);
                response["status"] = json!("failed");
                response["error"] = json!("local_art_confirmation_denied");
                envelope(response, Value::Null);
            }
        });
    let (local, frame) = prepared
        .mount(view(client.origin()), js::snapshot(1, 0), true, &|| false)
        .unwrap();
    assert_eq!(js::text(&frame.scene), "local_art_confirmation_denied");
    drop(local);
    peer.join().unwrap();
}

#[test]
fn art_service_confirmation_canvas_preview_and_image_result_share_the_exact_payload() {
    let _serial = js::SERIAL.lock().unwrap();
    const CANVAS: &str = r#"NeuroSurface.define({async mount({root}) {
      const c=document.createElement('canvas');c.width=1;c.height=1;c.getContext('2d').fillStyle='red';c.getContext('2d').fillRect(0,0,1,1);
      const result=await NeuroSurface.serviceWithConfirmation('capability:vendor:service.run',{n:7},{timeoutMs:5000,canvases:[c],imageOutput:true});
      const output=document.createElement('canvas');output.width=2;output.height=2;root.appendChild(output);output.getContext('2d').drawImage(result.image,0,0);
    }});"#;
    let source_image = crate::surface_javascript_wire::images::tests::resource();
    let result_image = json!({"width":2,"height":2,"digest":format!("{:x}",Sha256::digest(&*source_image.png)),"png":STANDARD.encode(&*source_image.png)});
    let (prepared, client, _device, _lease, peer) =
        setup_source(3, CANVAS, move |index, head, body, response| {
            if index == 0 {
                assert_eq!(body["images"].as_array().unwrap().len(), 1);
                assert_eq!(body["imageOutput"], true);
                envelope(response, challenge(body));
            } else {
                response["sequence"] = json!(2);
                response["status"] = json!("succeeded");
                response["output"] = json!({"answer":42});
                if index == 1 {
                    assert!(head.starts_with("POST /v1/art/local/service/confirm "));
                    assert_eq!(body["decision"]["approve"], true);
                    envelope(response, Value::Null);
                    response["image"] = result_image.clone();
                } else {
                    assert!(head.starts_with("POST /v1/art/local/service/cancel "));
                }
            }
        });
    let mut runtime =
        Runtime::start(prepared, view(client.origin()), js::snapshot(1, 0), true).unwrap();
    let binding = runtime.binding().clone();
    let pending = ticket(&runtime);
    assert_eq!(pending.images.len(), 1);
    assert!(pending.images[0].label.starts_with("画布 "));
    assert_eq!(pending.images[0].size, [1, 1]);
    assert_eq!(pending.images[0].image.pixels(), [0, 0, 255, 255]);
    assert!(pending.resolve(true));
    let due = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(frame) = runtime.take_reply(&binding) {
            assert_eq!(
                frame
                    .unwrap()
                    .images
                    .values()
                    .next()
                    .unwrap()
                    .pixels
                    .as_ref(),
                [0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 128, 128, 128]
            );
            break;
        }
        assert!(Instant::now() < due);
        std::thread::sleep(Duration::from_millis(5));
    }
    runtime.request_stop();
    drop(runtime);
    peer.join().unwrap();
}

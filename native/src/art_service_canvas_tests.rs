//! 实际signed child到原图片TLS出口；预期像素手工声明，不用新转换生成oracle。
use super::*;
use crate::surface_javascript::fixture as js;
const SOURCE_CANVAS: &str = r#"
NeuroSurface.define({async mount({root}){
 const c=document.createElement('canvas');c.width=2;c.height=1;c.style.width='400px';
 const g=c.getContext('2d');g.fillStyle='red';g.fillRect(0,0,1,1);
 g.fillStyle='rgba(0,0,255,0.5)';g.fillRect(1,0,1,1);
 let rejected=false;try{NeuroSurface.serviceCanvases('capability:vendor:service.run',null,[root],5000);}catch{rejected=true;}
 if(!rejected)throw new Error('invalid Canvas accepted');
 const p=NeuroSurface.serviceCanvases('capability:vendor:service.run',{canvas:true},[c],5000);
 c.width=1;g.clearRect(0,0,1,1);const result=await p;root.textContent=result.text;
}});
"#;
#[test]
fn canvas_service_signed_child_tls_preserves_call_time_intrinsic_pixels_and_original_owner() {
    let _serial = js::SERIAL.lock().unwrap();
    let mut owner = None;
    let (prepared, client, _device, _lease, peer) =
        setup_source(2, SOURCE_CANVAS, move |i, head, body, response| {
            if i == 0 {
                assert!(head.starts_with("POST /v1/art/local/service/submit-images "));
                assert_eq!(body["call"]["input"], json!({"canvas":true}));
                let images = body["images"].as_array().unwrap();
                assert_eq!(images.len(), 1);
                let bytes = STANDARD.decode(images[0]["png"].as_str().unwrap()).unwrap();
                let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
                    .read_info()
                    .unwrap();
                assert_eq!([reader.info().width, reader.info().height], [2, 1]);
                let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
                reader.next_frame(&mut pixels).unwrap();
                assert_eq!(pixels, [255, 0, 0, 255, 0, 0, 255, 128]);
                owner = Some(body["call"]["owner"].clone());
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/poll "));
                assert_eq!(Some(&body["owner"]), owner.as_ref());
                assert!(body.get("images").is_none());
                response["status"] = json!("succeeded");
                response["output"] = json!({"text":"canvas-read"});
            }
        });
    let (local, frame) = prepared
        .mount(view(client.origin()), js::snapshot(1, 1), true, &|| false)
        .unwrap();
    assert_eq!(js::text(&frame.scene), "canvas-read");
    drop(local);
    peer.join().unwrap();
}
#[test]
fn canvas_service_stop_cancels_same_owner_without_replay_or_publishing_frame() {
    let _serial = js::SERIAL.lock().unwrap();
    let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sent = stopped.clone();
    let mut owner = None;
    let (prepared, client, _device, lease, peer) =
        setup_source(2, SOURCE_CANVAS, move |i, head, body, response| {
            if i == 0 {
                owner = Some(body["call"]["owner"].clone());
                sent.store(true, std::sync::atomic::Ordering::SeqCst);
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/cancel "));
                assert_eq!(Some(&body["owner"]), owner.as_ref());
                assert!(body.get("images").is_none());
                response["status"] = json!("cancelled");
            }
        });
    assert!(prepared
        .mount(view(client.origin()), js::snapshot(1, 1), true, &|| stopped
            .load(std::sync::atomic::Ordering::SeqCst))
        .is_err());
    peer.join().unwrap();
    assert!(!lease.is_revoked());
}
#[test]
fn canvas_service_missing_images_endpoint_fails_without_silent_json_fallback() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, _device, _lease, peer) =
        setup_source_status(1, SOURCE_CANVAS, |_, head, _, _| {
            assert!(head.starts_with("POST /v1/art/local/service/submit-images "));
            404
        });
    assert!(prepared
        .mount(view(client.origin()), js::snapshot(1, 1), true, &|| false)
        .is_err());
    peer.join().unwrap();
}

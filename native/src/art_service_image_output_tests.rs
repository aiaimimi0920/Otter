//! 真signed child/隔离TLS图像回复；像素oracle独立声明，非由新适配器生成预期。
use super::*;
use crate::surface_javascript::fixture as js;
const SOURCE_OUTPUT: &str = r#"
NeuroSurface.define({async mount({root}) {
 const input=document.createElement('canvas');input.width=1;input.height=1;
 input.getContext('2d').fillRect(0,0,1,1);
 const result=await NeuroSurface.serviceImage('capability:vendor:service.run',{returnImage:true},{canvases:[input],timeoutMs:5000});
 if(result.output.answer!==42 || result.image.width!==2 || result.image.height!==2)throw new Error('metadata');
 let forged=false;try{input.getContext('2d').drawImage({width:2,height:2},0,0);}catch{forged=true;}if(!forged)throw new Error('forged');
 const c=document.createElement('canvas');c.width=2;c.height=2;root.appendChild(c);
 c.getContext('2d').drawImage(result.image,0,0);
 const check=await NeuroSurface.serviceImages('capability:vendor:service.run',null,[result.image],5000);
 if(check!==null)throw new Error('chain');
}});
"#;
fn wrap(response: &mut Value, image: Option<Value>) {
    *response = json!({"protocolVersion":"loom.art.local.service.image-output.v1","reply":response.clone(),"image":image});
}
fn image() -> Value {
    let image = crate::surface_javascript_wire::images::tests::resource();
    json!({"width":2,"height":2,"digest":format!("{:x}",Sha256::digest(&*image.png)),"png":STANDARD.encode(&*image.png)})
}
#[test]
fn image_output_signed_child_tls_installs_draws_chains_and_releases_original_call() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, _device, _lease, peer) =
        setup_source(4, SOURCE_OUTPUT, |i, head, body, response| match i {
            0 => {
                assert!(head.starts_with("POST /v1/art/local/service/submit-image-output "));
                assert_eq!(body["images"].as_array().unwrap().len(), 1);
                wrap(response, None);
            }
            1 => {
                assert!(head.starts_with("POST /v1/art/local/service/poll-image-output "));
                response["status"] = json!("succeeded");
                response["output"] = json!({"answer":42});
                wrap(response, Some(image()));
            }
            2 => {
                assert!(head.starts_with("POST /v1/art/local/service/cancel "));
                response["sequence"] = json!(2);
                response["status"] = json!("succeeded");
                response["output"] = json!({"answer":42});
            }
            _ => {
                assert!(head.starts_with("POST /v1/art/local/service/submit-images "));
                assert_eq!(body["images"][0]["png"], image()["png"]);
                response["status"] = json!("succeeded");
                response["output"] = Value::Null;
            }
        });
    let (local, frame) = prepared
        .mount(view(client.origin()), js::snapshot(1, 1), true, &|| false)
        .unwrap();
    assert_eq!(
        frame.images.values().next().unwrap().pixels.as_ref(),
        [0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 128, 128, 128]
    );
    drop(local);
    peer.join().unwrap();
}
#[test]
fn image_output_digest_mismatch_fails_without_install_or_json_fallback() {
    let _serial = js::SERIAL.lock().unwrap();
    let (prepared, client, _device, _lease, peer) =
        setup_source(1, SOURCE_OUTPUT, |_, head, _, response| {
            assert!(head.starts_with("POST /v1/art/local/service/submit-image-output "));
            response["status"] = json!("succeeded");
            response["output"] = json!({"answer":42});
            let mut wrong = image();
            wrong["digest"] = json!("0".repeat(64));
            // Wire/identity failures stop execution without another network request.
            wrap(response, Some(wrong));
        });
    assert!(prepared
        .mount(view(client.origin()), js::snapshot(1, 1), true, &|| false)
        .is_err());
    peer.join().unwrap();
}
#[test]
fn image_output_png_above_json_limit_preserves_intrinsic_pixels_without_js_base64() {
    let _serial = js::SERIAL.lock().unwrap();
    let mut state = 23u32;
    let rgba: Vec<u8> = (0..240 * 120 * 4)
        .map(|i| {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            if i % 4 == 3 {
                255
            } else {
                (state >> 24) as u8
            }
        })
        .collect();
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 240, 120);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&rgba)
            .unwrap();
    }
    assert!(png.len() > 65536 && png.len() < 2 * 1024 * 1024);
    let expected: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect();
    const LARGE: &str = r#"NeuroSurface.define({async mount({root}){const r=await NeuroSurface.serviceImage('capability:vendor:service.run',null,{timeoutMs:5000});const c=document.createElement('canvas');c.width=r.image.width;c.height=r.image.height;root.appendChild(c);c.getContext('2d').drawImage(r.image,0,0);}});"#;
    let (prepared, client, _device, _lease, peer) = setup_source(
        2,
        LARGE,
        move |i, head, _, response| {
            response["sequence"] = json!(1);
            response["status"] = json!("succeeded");
            response["output"] = Value::Null;
            if i == 0 {
                wrap(
                    response,
                    Some(
                        json!({"width":240,"height":120,"digest":format!("{:x}",Sha256::digest(&png)),"png":STANDARD.encode(&png)}),
                    ),
                );
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/cancel "));
            }
        },
    );
    let (local, frame) = prepared
        .mount(view(client.origin()), js::snapshot(1, 1), true, &|| false)
        .unwrap();
    let image = frame.images.values().next().unwrap();
    assert_eq!((image.width, image.height), (240, 120));
    assert_eq!(image.pixels.as_ref(), expected);
    drop(local);
    peer.join().unwrap();
}
#[test]
fn image_output_stop_before_terminal_never_installs_and_cancels_original_call() {
    let _serial = js::SERIAL.lock().unwrap();
    let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sent = stopped.clone();
    let (prepared, client, _device, lease, peer) =
        setup_source(2, SOURCE_OUTPUT, move |i, head, _, response| {
            if i == 0 {
                wrap(response, None);
                sent.store(true, std::sync::atomic::Ordering::Release);
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/cancel "));
                response["status"] = json!("cancelled");
            }
        });
    assert!(prepared
        .mount(view(client.origin()), js::snapshot(1, 1), true, &|| stopped
            .load(std::sync::atomic::Ordering::Acquire))
        .is_err());
    peer.join().unwrap();
    assert!(!lease.is_revoked());
}

//! 当前安装图→真实signed child→TLS新请求→原JSON结果；预期PNG来自独立png库。
use super::*;
use crate::surface_javascript::{fixture as js, signed::runtime::Content};
const IMAGE_SERVICE: &str = r#"
NeuroSurface.define({async mount({root}) {
 const image=NeuroSurface.image('input:1');
 const result=await NeuroSurface.serviceImages('capability:vendor:service.run',{operation:'read'},[image],5000);
 root.textContent=result.text;
}});
"#;
fn image() -> crate::surface_javascript::ImageResource {
    crate::surface_javascript_wire::images::tests::resource()
}
fn content() -> Content {
    Content {
        snapshot: js::snapshot(1, 1),
        resources: json!({}),
        images: vec![image()],
    }
}
#[test]
fn art_service_images_signed_child_transfers_exact_original_png_and_polls_original_owner() {
    let _serial = js::SERIAL.lock().unwrap();
    let mut installed = image();
    let mut png = installed.png.to_vec();
    // 保留独立2x2有效PNG，合法IEND后尾部让正文越过普通1MiB门；WIC仍须实际解码成功。
    png.resize(1024 * 1024 + 1, 0);
    installed.png = png.into();
    let expected = installed.png.clone();
    let mut owner = None;
    let (prepared, client, _device, _lease, peer) =
        setup_source(2, IMAGE_SERVICE, move |index, head, body, response| {
            if index == 0 {
                assert!(head.starts_with("POST /v1/art/local/service/submit-images "));
                assert_eq!(body["call"]["input"], json!({"operation":"read"}));
                assert_eq!(body["images"].as_array().unwrap().len(), 1);
                assert_eq!(
                    STANDARD
                        .decode(body["images"][0]["png"].as_str().unwrap())
                        .unwrap()
                        .as_slice(),
                    expected.as_ref()
                );
                owner = Some(body["call"]["owner"].clone());
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/poll "));
                assert_eq!(Some(&body["owner"]), owner.as_ref());
                assert!(body.get("images").is_none());
                response["status"] = json!("succeeded");
                response["output"] = json!({"text":"image-read"});
            }
        });
    let mut input = content();
    input.images = vec![installed];
    let (_local, frame) = prepared
        .mount_content_observed(view(client.origin()), input, true, None, None, &|| false)
        .unwrap();
    assert_eq!(js::text(&frame.scene), "image-read");
    peer.join().unwrap();
}
#[test]
fn art_service_images_rejects_unknown_foreign_duplicate_and_oversized_sources_before_network() {
    let _serial = js::SERIAL.lock().unwrap();
    for source in [
        "NeuroSurface.define({async mount(){await NeuroSurface.serviceImages('s',{},[{}]);}});",
        "NeuroSurface.define({async mount(){const i=NeuroSurface.image('input:1');await NeuroSurface.serviceImages('s',{},[i,i]);}});",
        "NeuroSurface.define({async mount(){const a=[];Object.defineProperty(a,0,{get(){throw 1}});a.length=1;await NeuroSurface.serviceImages('s',{},a);}});",
    ] {
        let (prepared,client,_device,_lease,peer)=setup_source(0,source,|_,_,_,_|unreachable!());
        peer.join().unwrap();
        assert!(prepared.mount_content_observed(view(client.origin()),content(),true,None,None,&||false).is_err());
    }
}
#[test]
fn art_service_images_stop_cancels_same_call_without_upload_replay_or_frame() {
    let _serial = js::SERIAL.lock().unwrap();
    let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sent = stopped.clone();
    let mut owner = None;
    let (prepared, client, _device, lease, peer) =
        setup_source(2, IMAGE_SERVICE, move |index, head, body, response| {
            if index == 0 {
                owner = Some(body["call"]["owner"].clone());
                sent.store(true, std::sync::atomic::Ordering::SeqCst);
            } else {
                assert!(head.starts_with("POST /v1/art/local/service/cancel "));
                assert_eq!(Some(&body["owner"]), owner.as_ref());
                assert!(body.get("images").is_none());
                response["status"] = json!("cancelled");
            }
        });
    let result = prepared.mount_content_observed(
        view(client.origin()),
        content(),
        true,
        None,
        None,
        &|| stopped.load(std::sync::atomic::Ordering::SeqCst),
    );
    peer.join().unwrap();
    assert!(result.is_err());
    assert!(!lease.is_revoked());
}

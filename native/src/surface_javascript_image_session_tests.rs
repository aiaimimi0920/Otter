//! 真实受限child中的图片安装、正式结果及取消回收；不替代Application产品链测试。
use super::*;
use crate::surface_javascript_wire::images;
fn input() -> services::Mount {
    services::Mount {
        binding: fixture::binding(),
        source: r#"NeuroSurface.define({mount({root}){
          const im=NeuroSurface.image('input:1'),c=document.createElement('canvas');
          c.id='out';c.width=im.width;c.height=im.height;root.append(c);c.getContext('2d').drawImage(im,0,0);
          NeuroSurface.commitResult({output:{kind:'canvas',nodeId:'out'}});
        }});"#.into(),
        snapshot: fixture::snapshot(1,1), resources:serde_json::json!({}),
        images:vec![images::tests::resource()], viewport:None,dispatcher:None,allow_results:true,
    }
}
#[test]
fn image_session_transfers_raw_png_and_returns_formal_pixels_then_reclaims_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount_serviced(input(), &|| false).unwrap();
    assert_eq!(session.sequence, 2);
    let result = frame.result.unwrap();
    let wire::result::ResolvedOutput::Image(image) = &result.outputs["output"] else {
        panic!("expected formal image")
    };
    assert_eq!(
        image.pixels.as_ref(),
        &[0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 128, 128, 128,]
    );
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn image_session_rejects_bad_png_and_missing_result_permission_without_leaking_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let mut bad = input();
    bad.images[0].width = 1;
    bad.images[0].height = 4;
    assert!(Session::mount_serviced(bad, &|| false).is_err());
    assert_eq!(CAPACITY.used(), 0);
    let mut denied = input();
    denied.allow_results = false;
    assert!(Session::mount_serviced(denied, &|| false).is_err());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn image_session_cancel_after_spawn_never_delivers_frame_and_reclaims_job() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let stopped = std::cell::Cell::new(false);
    let input = input();
    let result = Session::mount_options(
        input.binding,
        input.source,
        input.snapshot,
        input.resources,
        services::Options {
            images: input.images,
            viewport: None,
            dispatcher: None,
            allow_results: true,
        },
        &|| stopped.get(),
        |endpoint, deadline| {
            let child=worker::start(endpoint,deadline,&||false,worker::Policy {
                suffix:" --exact surface_javascript_worker::tests::javascript_worker_entry --ignored --nocapture",
                maximum:CHILD_LIMIT,pool:&CAPACITY,environment:worker::Environment::SystemRoot,
            })?;
            stopped.set(true);
            Ok(child)
        },
    );
    assert!(result.is_err());
    assert_eq!(CAPACITY.used(), 0);
}

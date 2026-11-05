//! 同预算真实 child 的定位、裁剪、动态变更与回收；产品门必须绑定外部预期摘要。
use super::*;
use crate::surface_viewport::Viewport;
use sha2::{Digest, Sha256};
const SOURCE: &str = include_str!("../tests/fixtures/javascript-spatial.js");
pub(crate) fn report(scene: &Scene) -> Value {
    serde_json::from_str(scene.root().children()[1].props()["text"].as_str().unwrap()).unwrap()
}
fn exercise(mut session: Session, frame: Frame) {
    let view = Viewport::new([400, 240], 192).unwrap();
    assert_eq!(frame.viewport, Some(view));
    let value = report(&frame.scene);
    assert_eq!(value["stage"]["width"], 100);
    assert_eq!(value["fill"]["left"], 2);
    assert_eq!(value["fill"]["width"], 96);
    assert_eq!(value["fill"]["height"], 56);
    assert_eq!(value["flow"]["top"], 6);
    assert_eq!(value["overlay"]["left"], 72);
    assert_eq!(value["overlay"]["top"], 12);
    assert_eq!(value["client"], serde_json::json!([96, 56]));
    let target = frame.scene.root().children()[0].children()[2]
        .id()
        .to_owned();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let frame = session.click(&target, &|| false).unwrap();
    assert_eq!(report(&frame.scene)["overlay"]["left"], 82);
    assert_eq!(
        frame.scene.root().children()[0].layout()["overflowX"],
        "visible"
    );
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    let value = report(&frame.scene);
    assert_eq!(value["fill"]["width"], 136);
    assert_eq!(value["overlay"]["left"], 83);
    assert_eq!(value["overlay"]["top"], 23);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-spatial: same-child=true anchors=true padding-box=true synchronous-bounds=true dynamic-overflow=true closed=true peak-job-bytes={}", probe.peak_job_bytes().unwrap());
}
#[test]
fn javascript_spatial_real_child_mount_click_update_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([400, 240], 192).unwrap()),
        &|| false,
    )
    .unwrap();
    exercise(session, frame);
}
#[test]
fn javascript_spatial_rejects_unimplemented_containing_blocks_order_scroll_and_auto_height() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for body in [
        "root.style.top='1px'",
        "root.style.overflowX='hidden'",
        "root.innerHTML='<span></span>';root.firstChild.style.overflow='visible'",
        "root.innerHTML='<div></div>';root.firstChild.style.cssText='position:absolute;inset:0'",
        "root.style.position='relative';root.innerHTML='<div><div></div></div>';root.firstChild.firstChild.style.position='relative'",
        "root.style.position='relative';root.innerHTML='<div></div><div></div>';root.firstChild.style.position='relative'",
        "root.innerHTML='<div><div></div></div><div><div></div></div>';root.firstChild.firstChild.style.position='relative';root.lastChild.firstChild.style.position='relative'",
        "root.style.cssText='position:relative;display:grid';root.innerHTML='<div></div>';root.firstChild.style.cssText='position:absolute;inset:0'",
        "root.style.position='relative';root.innerHTML='<div></div>';root.firstChild.style.cssText='position:absolute;left:0;top:0'",
        "root.style.position='relative';root.innerHTML='<div></div>';root.firstChild.style.cssText='position:absolute;left:0;right:0'",
        "root.style.position='relative';root.innerHTML='<div></div>';root.firstChild.style.cssText='position:relative;top:50%'",
        "root.style.position='relative';root.innerHTML='<div></div>';root.firstChild.style.cssText='position:relative;bottom:10%'",
        "root.style.position='relative';root.innerHTML='<div></div>';root.firstChild.style.cssText='position:absolute;inset:0;flex-grow:1'",
        "root.innerHTML='<button><span>inline</span></button>';root.firstChild.firstChild.style.position='relative'",
    ] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{{body}}}}});");
        assert!(Session::mount_in(fixture::binding(), source, fixture::snapshot(3, 42), fixture::resources(),
            Some(Viewport::new([400, 240], 96).unwrap()), &|| false).is_err(), "{body}");
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
fn javascript_spatial_real_child_atomic_rejections_and_intrinsics_survive_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for source in [
        include_str!("../tests/fixtures/javascript-spatial-rejection.js"),
        include_str!("../tests/fixtures/javascript-spatial-integrity.js"),
    ] {
        let (mut session, _) = Session::mount(
            fixture::binding(),
            source.into(),
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        let probe = session.child.as_ref().unwrap().probe().unwrap();
        session
            .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
            .unwrap();
        session.dispose(&|| false).unwrap();
        assert!(probe.exited());
    }
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_spatial_relative_offsets_preserve_flow_and_percent_anchors_use_padding_box() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"NeuroSurface.define({mount({root}){
      const s=new CSSStyleSheet();s.replaceSync('*{box-sizing:border-box}:root{position:relative;width:100px;height:80px;padding:5px;border-width:2px;border-style:solid}div{position:relative;width:20px;height:10px;min-height:10px}');document.adoptedStyleSheets=[s];
      root.innerHTML='<div></div><div></div><div></div><p></p>';
      root.children[0].style.cssText='right:10%;bottom:2px';root.children[1].style.left='5px';
      root.children[2].style.cssText='position:absolute;left:50%;right:0;top:25%;bottom:0';root.lastChild.style.display='none';
      root.lastChild.textContent=JSON.stringify([root.children[0].getBoundingClientRect(),root.children[1].getBoundingClientRect(),root.children[2].getBoundingClientRect()]);
    }});"#;
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([200, 160], 96).unwrap()),
        &|| false,
    )
    .unwrap();
    let value: Value = serde_json::from_str(
        frame.scene.root().children()[3].props()["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert!((value[0]["left"].as_f64().unwrap() - (-1.6)).abs() < 0.01);
    assert_eq!(value[0]["top"], 5);
    assert_eq!(value[1]["left"], 12);
    assert_eq!(value[1]["top"], 23); // 第一项偏移不改变后项正常占位，含原 column gap=6。
    assert_eq!(value[2]["left"], 50);
    assert_eq!(value[2]["top"], 21);
    session.dispose(&|| false).unwrap();
}
#[test]
fn javascript_spatial_single_anchor_decimal_dimensions_are_not_intrinsic_fallbacks() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"NeuroSurface.define({mount({root}){
      root.style.position='relative';root.innerHTML='<div></div><p></p>';
      const node=root.firstChild;node.style.cssText='box-sizing:border-box;position:absolute;left:0;top:0;width:.5px;height:10.px';
      root.lastChild.style.display='none';root.lastChild.textContent=JSON.stringify(node.getBoundingClientRect());
    }});"#;
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([200, 160], 96).unwrap()),
        &|| false,
    )
    .unwrap();
    let value: Value = serde_json::from_str(
        frame.scene.root().children()[1].props()["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value["width"], 0.5);
    assert_eq!(value["height"], 10);
    session.dispose(&|| false).unwrap();
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_spatial_actual_product_worker_uses_same_position_and_clip_projection() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    let (session, frame) = Session::mount_with_viewport(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([400, 240], 192).unwrap()),
        &|| false,
        |endpoint, deadline| {
            worker::start_at(
                endpoint,
                deadline,
                &|| false,
                worker::Policy {
                    suffix: " --native-javascript-worker",
                    maximum: CHILD_LIMIT,
                    pool: &CAPACITY,
                    environment: worker::Environment::SystemRoot,
                },
                &path,
            )
        },
    )
    .unwrap();
    exercise(session, frame);
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

//! 固定原预算下的同步读/变更/视口/失败与回收；产品门另绑定外部预期摘要。
use super::*;
use crate::surface_viewport::Viewport;
use sha2::{Digest, Sha256};
const SOURCE: &str = include_str!("../tests/fixtures/javascript-geometry.js");
fn reentrant_source(mode: i32) -> &'static str {
    if mode < 0 {
        include_str!("../tests/fixtures/javascript-geometry-reentrancy.js")
    } else {
        include_str!("../tests/fixtures/javascript-geometry-reentrancy-variants.js")
    }
}
fn exercise_reentrant(mut session: Session, frame: Frame, mode: i32) {
    let width = if mode < 2 { "60px" } else { "50px" };
    let color = if mode < 2 { "green" } else { "blue" };
    assert_eq!(frame.scene.root().layout()["width"], width);
    assert_eq!(frame.scene.root().style()["color"], color);
    assert_eq!(fixture::text(&frame.scene), "ready");
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let frame = session
        .update(fixture::snapshot(4, mode), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(frame.scene.root().layout()["width"], width);
    assert_eq!(frame.scene.root().style()["color"], color);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-geometry: reentrant-mode={mode} original-stack=true same-child=true closed=true");
}
pub(crate) fn report(scene: &Scene) -> Value {
    serde_json::from_str(scene.root().children()[2].props()["text"].as_str().unwrap()).unwrap()
}
fn exercise(mut session: Session, frame: Frame) {
    let view = Viewport::new([600, 400], 192).unwrap();
    assert_eq!(frame.viewport, Some(view));
    let value = report(&frame.scene);
    assert_eq!(value["first"]["width"], 40.);
    assert_eq!(value["second"]["left"], 48.);
    assert_eq!(value["second"]["width"], 252.);
    assert_eq!(value["client"], serde_json::json!([36, 56]));
    assert_eq!(value["offset"], serde_json::json!([40, 60]));
    assert_eq!(value["hidden"]["width"], 0);
    assert_eq!(value["ratio"], 2.);
    let target = frame.scene.root().children()[0].id().to_owned();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let frame = session.click(&target, &|| false).unwrap();
    assert_eq!(report(&frame.scene)["first"]["width"], 70.);
    session
        .set_viewport(Viewport::new([480, 320], 96).unwrap())
        .unwrap();
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(report(&frame.scene)["first"]["width"], 90.);
    assert_eq!(report(&frame.scene)["second"]["width"], 382.);
    assert_eq!(report(&frame.scene)["ratio"], 1.);
    session.suspend(&|| false).unwrap();
    assert!(session.click(&target, &|| false).is_err());
    let frame = session.resume(&|| false).unwrap();
    assert_eq!(report(&frame.scene)["first"]["width"], 90.);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-geometry: synchronous-mount=true mutation=true viewport=true dpi=true same-child=true closed=true peak-bytes={}", probe.peak_job_bytes().unwrap());
}
#[test]
fn javascript_geometry_real_child_mount_mutation_viewport_update_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([600, 400], 192).unwrap()),
        &|| false,
    )
    .unwrap();
    exercise(session, frame);
}
#[test]
fn javascript_geometry_real_child_conversion_reentrancy_and_atomic_rejection_reap_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for mode in -1..6 {
        let (session, frame) = Session::mount_in(
            fixture::binding(),
            reentrant_source(mode).into(),
            fixture::snapshot(3, mode),
            fixture::resources(),
            Some(Viewport::new([600, 400], 96).unwrap()),
            &|| false,
        )
        .unwrap();
        exercise_reentrant(session, frame, mode);
    }
}
#[test]
fn javascript_geometry_missing_viewport_and_native_layout_failure_reap_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    assert!(Session::mount(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false
    )
    .is_err());
    let source = "NeuroSurface.define({mount({root}){root.style.display='grid';root.style.gridTemplateRows='20%';try{root.getBoundingClientRect()}catch{}}});";
    assert!(Session::mount_in(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([600, 400], 96).unwrap()),
        &|| false
    )
    .is_err());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_geometry_caught_read_and_layout_budget_errors_still_fail_operation() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for body in [
        "for(let i=0;i<257;i++)try{root.clientWidth}catch{}",
        "root.style.boxSizing='border-box';for(let i=0;i<9;i++){root.style.width=(100+i)+'px';try{root.clientWidth}catch{}}",
    ] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{{body}}}}});");
        assert!(Session::mount_in(fixture::binding(), source, fixture::snapshot(3, 42), fixture::resources(),
            Some(Viewport::new([600, 400], 96).unwrap()), &|| false).is_err());
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
fn javascript_geometry_intrinsic_pollution_cannot_change_private_boxes_or_rounding() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"let host;NeuroSurface.define({mount({root}){host=root;root.style.cssText='box-sizing:border-box;width:40.25px;height:30.25px';root.innerHTML='<button><span>safe</span></button>';
      Object.getPrototypeOf(new Map().entries()).next=()=>{throw Error('iterator next')};
      Map.prototype.entries=Map.prototype[Symbol.iterator]=()=>{throw Error('map iterator')};
      Math.round=()=>999;JSON.stringify=()=>'';Map.prototype.get=()=>undefined;Object.prototype.toJSON=()=>({bad:true});
      if(root.clientWidth!==40||root.offsetHeight!==30||root.getBoundingClientRect().width!==40.25)throw Error('polluted');},
      update(){if(host.getBoundingClientRect().height!==30.25||devicePixelRatio!==1)throw Error('cross frame');}});"#;
    let (mut session, frame) = Session::mount_in(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(Viewport::new([600, 400], 96).unwrap()),
        &|| false,
    )
    .unwrap();
    assert!(frame.viewport.is_some());
    session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
// 仅复用已有产品worker启动接缝；所有场景仍经过原Session/Job/容量与期限门。
pub(super) fn mount_product(
    path: &std::path::Path,
    source: &str,
    mode: i32,
    view: Viewport,
) -> (Session, Frame) {
    Session::mount_with_viewport(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, mode),
        fixture::resources(),
        Some(view),
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
                path,
            )
        },
    )
    .unwrap()
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_geometry_actual_product_worker_uses_same_native_measure() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    let (session, frame) =
        mount_product(&path, SOURCE, 42, Viewport::new([600, 400], 192).unwrap());
    exercise(session, frame);
    for mode in -1..6 {
        let (session, frame) = mount_product(
            &path,
            reentrant_source(mode),
            mode,
            Viewport::new([600, 400], 96).unwrap(),
        );
        exercise_reentrant(session, frame, mode);
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

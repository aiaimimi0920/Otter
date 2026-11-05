//! 真实持久 child 的 Grid 声明、上下文与回收；实际产品另绑定封存 EXE 摘要。
use super::*;
use sha2::{Digest, Sha256};
const SOURCE: &str = include_str!("../tests/fixtures/javascript-grid.js");
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let root = frame.scene.root();
    assert_eq!(root.kind(), crate::surface_scene::Kind::Grid);
    assert_eq!(root.layout()["rowGap"], "4px");
    let target = root.children()[0].id().to_owned();
    let footer = root.children()[2].id().to_owned();
    let frame = session.click(&target, &|| false).unwrap();
    assert_eq!(frame.events.len(), 1);
    let event: Value = serde_json::from_slice(frame.events[0].bytes()).unwrap();
    assert_eq!(event["payload"]["grid"], true);
    assert_eq!(event["baseRevision"], 3);
    assert_eq!(
        frame.scene.root().layout()["gridTemplateColumns"],
        "60px 1fr"
    );
    assert!(!frame.scene.root().children()[1].visible());
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert!(frame.events.is_empty());
    assert_eq!(frame.scene.root().children()[0].id(), target);
    assert_eq!(frame.scene.root().children()[2].id(), footer);
    assert_eq!(
        frame.scene.root().children()[2].layout()["gridColumn"],
        "2 / auto"
    );
    assert_eq!(
        frame.scene.root().layout()["gridTemplateColumns"],
        "80px 1fr"
    );
    assert!(frame.scene.root().children()[1].visible());
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    println!("native-javascript-grid: cascade=true click=true update=true same-child=true reaped=true peak-job-bytes={peak}");
}
#[test]
fn javascript_grid_real_child_cascade_click_update_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    exercise(session, frame);
}
#[test]
fn javascript_grid_real_child_invalid_declarations_leave_sheet_and_inline_atomic() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../tests/fixtures/javascript-grid-rejection.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    assert_eq!(frame.scene.root().layout()["rowGap"], "8px");
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_grid_real_child_intrinsics_stay_private_across_collect_update_dispose() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../tests/fixtures/javascript-grid-integrity.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert_eq!(frame.scene.root().layout()["columnGap"], "5px");
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(frame.scene.root().layout()["columnGap"], 0);
    assert_eq!(frame.scene.root().layout()["justifyItems"], "end");
    assert_eq!(
        frame.scene.root().children()[0].layout()["gridColumn"],
        "1 / -1"
    );
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
}
#[test]
fn javascript_grid_real_child_rejects_unconsumed_visible_contexts_and_inline_boxes() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for operation in [
        "root.innerHTML='<span>x</span>';root.firstChild.style.display='grid'",
        "root.innerHTML='<button>x</button>';root.firstChild.style.cssText='display:grid;place-items:center'",
        "root.style.gridTemplateColumns='1fr'",
        "root.style.cssText='display:flex;grid-template-columns:1fr'",
        "root.style.cssText='display:grid;flex-direction:row'",
        "root.style.cssText='display:flex;place-content:center'",
        "root.style.cssText='display:flex;align-items:baseline'",
        "root.style.cssText='display:flex;justify-content:stretch'",
        "root.style.gridColumn='2'",
        "root.style.display='grid';root.innerHTML='<button>x</button>';root.firstChild.style.flexGrow='1'",
        "root.style.display='flex';root.innerHTML='<button>x</button>';root.firstChild.style.alignSelf='center'",
        "root.style.display='grid';root.innerHTML='<button><span>x</span></button>';root.firstChild.firstChild.style.gridColumn='2'",
        "root.style.display='grid';root.innerHTML='<button>x</button>';root.firstChild.style.cssText='grid-row-start:span 2;grid-row-end:span 3'",
    ] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{{operation}}}}});");
        assert!(Session::mount(fixture::binding(), source, fixture::snapshot(3, 42), fixture::resources(), &|| false).is_err(), "{operation}");
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
fn javascript_grid_real_child_hidden_retains_known_declarations_but_show_rechecks_context() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"let host;NeuroSurface.define({mount({root}){host=root;root.style.cssText='display:none;grid-template-columns:1fr';root.innerHTML='<button>x</button>';root.firstChild.style.gridColumn='2';},update(){host.style.display='flex';}});"#;
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert!(!frame.scene.root().visible());
    assert!(session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .is_err());
    drop(session);
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_grid_real_child_hidden_grid_can_show_without_losing_retained_placement() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"let host;NeuroSurface.define({mount({root}){host=root;root.style.cssText='display:none;grid-template-columns:40px 1fr';root.innerHTML='<button>x</button>';root.firstChild.style.gridColumn='2';},update(){host.style.display='grid';}});"#;
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let target = frame.scene.root().children()[0].id().to_owned();
    assert!(!frame.scene.root().visible());
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert!(frame.scene.root().visible());
    assert_eq!(frame.scene.root().kind(), crate::surface_scene::Kind::Grid);
    assert_eq!(frame.scene.root().children()[0].id(), target);
    assert_eq!(
        frame.scene.root().children()[0].layout()["gridColumn"],
        "2 / auto"
    );
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_grid_actual_product_worker_uses_same_css_grid_and_updates() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    let (session, frame) = Session::mount_with(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
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

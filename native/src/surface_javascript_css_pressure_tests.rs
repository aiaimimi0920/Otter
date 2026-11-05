//! 原规模压力经过真实持久child/实际产品，校验完整帧、同一文档和Job回收；不改预算。
use super::*;
use sha2::{Digest, Sha256};
const SOURCE: &str = include_str!("../tests/fixtures/javascript-css-pressure.js");

fn check(frame: &Frame) {
    assert_eq!(frame.scene.root().children().len(), 256);
    for node in frame.scene.root().children() {
        assert_eq!(node.style()["opacity"], 0.5);
    }
    assert!(frame.events.is_empty());
}

fn exercise(mut session: Session, frame: Frame) {
    check(&frame);
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let ids: Vec<_> = frame
        .scene
        .root()
        .children()
        .iter()
        .map(|node| node.id().to_owned())
        .collect();
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    check(&frame);
    for (node, expected) in frame.scene.root().children().iter().zip(ids) {
        assert_eq!(node.id(), expected);
    }
    assert!(!probe.exited());
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
    println!("native-css-pressure: nodes=256 rules=16 same-document=true same-child=true reaped=true peak-job-bytes={peak}");
}

#[test]
#[ignore = "explicit original-scale pressure gate; always run separately so a failure cannot poison functional regression locks"]
fn javascript_css_projection_pressure_real_child_same_document_and_cleanup() {
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
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_css_projection_pressure_actual_product_worker_same_budget() {
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

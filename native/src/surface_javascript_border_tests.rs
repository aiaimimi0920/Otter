//! 同预算真实 child/产品 worker 的四边占位、同步客户盒、稳定 identity 与回收。
use super::*;
use sha2::{Digest, Sha256};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-border-sides.js");
pub(crate) fn view() -> crate::surface_viewport::Viewport {
    crate::surface_viewport::Viewport::new([200, 80], 96).unwrap()
}
fn check(frame: &Frame, expected: [u32; 6]) {
    let nodes = frame.scene.root().children();
    let report: Value = serde_json::from_str(nodes[1].props()["text"].as_str().unwrap()).unwrap();
    assert_eq!(report, serde_json::json!(expected));
    assert!(frame.events.is_empty());
}
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let id = frame.scene.root().children()[0].id().to_owned();
    check(&frame, [100, 64, 90, 56, 11, 5]);
    for (revision, expected) in [
        (4, [100, 64, 94, 56, 7, 5]),
        (5, [100, 64, 100, 64, 3, 3]),
        (6, [100, 64, 90, 56, 11, 5]),
    ] {
        let next = session
            .update(
                fixture::snapshot(revision, 45),
                fixture::resources(),
                &|| false,
            )
            .unwrap();
        assert_eq!(next.scene.root().children()[0].id(), id);
        check(&next, expected);
    }
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_border_sides_real_child_geometry_update_restore_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(view()),
        &|| false,
    )
    .unwrap();
    exercise(session, frame);
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_border_sides_actual_product_worker_geometry_update_restore_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    let (session, frame) = geometry_tests::mount_product(&path, SOURCE, 42, view());
    exercise(session, frame);
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

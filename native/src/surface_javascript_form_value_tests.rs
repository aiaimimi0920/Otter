//! 值校验边界和污染通过同一child/实际产品；本地失败不产生emit或改变权限。
use super::*;
const SOURCE: &str = include_str!("../tests/fixtures/javascript-form-value-boundaries.js");
const CASES: usize = 28;

fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let button = frame.scene.root().children()[2].id().to_owned();
    let field = frame.scene.root().children()[1].id().to_owned();
    assert_eq!(fixture::text(&frame.scene), "ready");
    for index in 1..=CASES {
        let started = std::time::Instant::now();
        let result = session.click(&button, &|| false);
        eprintln!(
            "native-form-case: index={index} wallUs={} succeeded={}",
            started.elapsed().as_micros(),
            result.is_ok()
        );
        let next = result.unwrap_or_else(|error| panic!("form case {index}: {error}"));
        assert_eq!(fixture::text(&next.scene), format!("case:{index}"));
        assert_eq!(next.scene.root().children()[1].id(), field);
        assert_eq!(next.scene.root().children()[1].props()["value"], "seed");
        assert!(next.events.is_empty());
    }
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
}

#[test]
fn javascript_form_value_boundaries_atomic_rejection_and_intrinsics_use_real_child() {
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
fn javascript_form_value_actual_product_worker_uses_same_budget_and_boundaries() {
    use sha2::{Digest, Sha256};
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    let (session, frame) = geometry_tests::mount_product(
        &path,
        SOURCE,
        42,
        crate::surface_viewport::Viewport::new([320, 200], 96).unwrap(),
    );
    exercise(session, frame);
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

//! 四边 padding 贯穿真实 child、DirectWrite 布局与同步观察；沿用原产品 worker 接缝。
use super::*;
use crate::surface_viewport::Viewport;
use sha2::{Digest, Sha256};
const SOURCES: [&str; 3] = [
    include_str!("../tests/fixtures/javascript-padding.js"),
    include_str!("../tests/fixtures/javascript-padding-geometry.js"),
    include_str!("../tests/fixtures/javascript-padding-integrity.js"),
];
fn view(updated: bool) -> Viewport {
    if updated {
        Viewport::new([300, 120], 144)
    } else {
        Viewport::new([200, 80], 96)
    }
    .unwrap()
}
fn check(frame: &Frame, case: usize, updated: bool) {
    let viewport = view(updated);
    let context = viewport.context();
    let mut measure = crate::surface_native_measure::NativeMeasure::new(
        &frame.scene,
        crate::surface_viewport::style(),
        context,
        viewport.logical(),
    )
    .unwrap();
    let prepared = measure
        .prepare(&frame.scene, viewport.logical(), context)
        .unwrap();
    if case == 0 {
        assert_eq!(
            prepared.tree().nodes()[1].origin,
            if updated { [19., 9.] } else { [15., 5.] }
        );
        assert_eq!(
            prepared.tree().nodes()[2].origin,
            if updated { [55., 9.] } else { [51., 5.] }
        );
        let text = &prepared.text().items()[0];
        assert_eq!(
            text.allocated(),
            if updated { [20., 34.] } else { [22., 34.] }
        );
        assert_eq!(text.origin()[0], if updated { 28. } else { 22. });
    } else if case == 1 {
        let report = frame.scene.root().children()[1].props()["text"]
            .as_str()
            .unwrap();
        let values: Vec<f32> = serde_json::from_str(report).unwrap();
        assert_eq!(
            values,
            if updated {
                vec![17., 7., 168., 56., 252., 84.]
            } else {
                vec![13., 3., 176., 62., 176., 62.]
            }
        );
        assert_eq!(frame.viewport, Some(viewport));
        assert!(frame.resize_observed);
    } else {
        let inset = crate::surface_box::inset(frame.scene.root(), 200., context);
        assert_eq!(inset.start(), if updated { [12., 1.] } else { [4., 1.] });
        assert_eq!(inset.total(), if updated { [14., 4.] } else { [6., 4.] });
    }
}
fn exercise(mut session: Session, frame: Frame, case: usize) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let id = frame.scene.root().id().to_owned();
    check(&frame, case, false);
    session.set_viewport(view(true)).unwrap();
    let next = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(next.scene.root().id(), id);
    check(&next, case, true);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-padding: case={case} same-child=true four-edges=true geometry=true reaped=true peak-job-bytes={peak}");
}
#[test]
fn javascript_padding_real_child_cascade_geometry_observer_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for (case, source) in SOURCES.iter().enumerate() {
        let (session, frame) = Session::mount_in(
            fixture::binding(),
            (*source).into(),
            fixture::snapshot(3, 42),
            fixture::resources(),
            Some(view(false)),
            &|| false,
        )
        .unwrap();
        exercise(session, frame, case);
    }
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_padding_actual_product_worker_uses_same_four_edge_layout() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    for (case, source) in SOURCES.iter().enumerate() {
        let (session, frame) = geometry_tests::mount_product(&path, source, 42, view(false));
        exercise(session, frame, case);
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

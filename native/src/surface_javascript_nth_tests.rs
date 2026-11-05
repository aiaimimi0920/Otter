//! 通用 CSS 位置匹配穿过同一 child/实际产品；原权限、单槽和资源预算不变。
use super::*;
use sha2::{Digest, Sha256};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-nth-child.js");
const GEOMETRY: &str = include_str!("../tests/fixtures/javascript-nth-child-geometry.js");
const INTEGRITY: &str = include_str!("../tests/fixtures/javascript-nth-child-integrity.js");
fn check(frame: &Frame, green: &str, blue: &str) {
    assert_eq!(frame.scene.root().children().len(), 8);
    for (index, node) in frame.scene.root().children().iter().enumerate() {
        assert_eq!(
            node.style()["background"],
            if index % 4 == 3 { green } else { blue }
        );
    }
    assert!(frame.events.is_empty());
}
fn exercise(mut session: Session, frame: Frame, mode: usize) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let first = frame.scene.root().children()[0].id().to_owned();
    if mode == 0 {
        check(&frame, "#00ff00", "#0000ff");
        let frame = session.click(&first, &|| false).unwrap();
        check(&frame, "#00ff00", "#0000ff");
        assert_eq!(frame.scene.root().children()[7].id(), first);
    } else if mode == 1 {
        assert_eq!(frame.viewport, Some(view()));
        assert_eq!(
            frame.scene.root().children()[3].props()["text"],
            "[[20,30,20],[20,20,30],[30,20],[30,20,20]]"
        );
    } else {
        assert_eq!(frame.scene.root().children()[0].style()["color"], "blue");
        assert_eq!(frame.scene.root().children()[1].style()["color"], "green");
    }
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    if mode == 0 {
        assert_eq!(frame.scene.root().children()[0].id(), first);
        check(&frame, "#00ff00", "#0000ff");
    } else if mode == 1 {
        assert_eq!(
            frame.scene.root().children()[3].props()["text"],
            "[[20,30,20],[20,20,30],[30,20],[30,20,20],[20,20,30]]"
        );
    } else {
        assert_eq!(frame.scene.root().children()[3].id(), first);
        for (index, node) in frame.scene.root().children().iter().enumerate() {
            assert_eq!(
                node.style()["color"],
                if index % 2 == 1 { "green" } else { "blue" }
            );
        }
    }
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-nth-child: mode={mode} same-child=true reaped=true peak-job-bytes={peak}");
}
fn view() -> crate::surface_viewport::Viewport {
    crate::surface_viewport::Viewport::new([200, 80], 96).unwrap()
}
#[test]
fn javascript_nth_child_real_child_reorder_geometry_intrinsics_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for (mode, source) in [SOURCE, GEOMETRY, INTEGRITY].iter().enumerate() {
        let (session, frame) = Session::mount_in(
            fixture::binding(),
            (*source).into(),
            fixture::snapshot(3, 42),
            fixture::resources(),
            Some(view()),
            &|| false,
        )
        .unwrap();
        exercise(session, frame, mode);
    }
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_nth_child_actual_product_worker_reorders_same_document() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    for (mode, source) in [SOURCE, GEOMETRY, INTEGRITY].iter().enumerate() {
        let (session, frame) = Session::mount_with_viewport(
            fixture::binding(),
            (*source).into(),
            fixture::snapshot(3, 42),
            fixture::resources(),
            Some(view()),
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
        exercise(session, frame, mode);
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

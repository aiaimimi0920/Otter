//! 字体通用夹具通过真实 child/原生同步几何；实际产品门另绑定外部 EXE 摘要。
use super::*;
use sha2::{Digest, Sha256};
const SOURCE: &str = include_str!("../tests/fixtures/javascript-fonts.js");
const GEOMETRY: &str = include_str!("../tests/fixtures/javascript-fonts-geometry.js");
fn view() -> crate::surface_viewport::Viewport {
    crate::surface_viewport::Viewport::new([600, 400], 96).unwrap()
}
fn check(frame: &Frame, geometry: bool, updated: bool) {
    if geometry {
        let values: Vec<f32> = serde_json::from_str(
            frame.scene.root().children()[1].props()["text"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let engine = crate::surface_text::Engine::new().unwrap();
        let width = |family| {
            engine
                .layout(
                    "WWii 0123",
                    crate::surface_text::Style {
                        family,
                        ..crate::surface_viewport::style()
                    },
                    600.,
                    400.,
                )
                .unwrap()
                .size()[0]
        };
        for (actual, expected) in
            values
                .iter()
                .zip([width("Consolas"), width("Segoe UI"), width("Consolas")])
        {
            assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
        }
        assert_eq!(values.len(), 3);
        assert_eq!(frame.viewport, Some(view()));
    } else {
        assert_eq!(
            frame.scene.root().children()[0].style()["fontFamily"],
            if updated {
                "\"Segoe UI\",sans-serif"
            } else {
                "\"MissingOtterFamily\",\"Consolas\",monospace"
            }
        );
        assert_eq!(
            frame.scene.root().children()[1].style()["fontFamily"],
            if updated {
                "\"MissingOtterFamily\",monospace"
            } else {
                "\"Segoe UI\",sans-serif"
            }
        );
    }
}
fn exercise(mut session: Session, frame: Frame, geometry: bool) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let id = frame.scene.root().children()[0].id().to_owned();
    check(&frame, geometry, false);
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(frame.scene.root().children()[0].id(), id);
    check(&frame, geometry, true);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
    println!("native-javascript-font-family: geometry={geometry} same-child-update=true atomic=true reaped=true peak-job-bytes={peak}");
}
#[test]
fn javascript_font_family_real_child_same_document_geometry_update_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for (source, geometry) in [(SOURCE, false), (GEOMETRY, true)] {
        let (session, frame) = Session::mount_in(
            fixture::binding(),
            source.into(),
            fixture::snapshot(3, 42),
            fixture::resources(),
            Some(view()),
            &|| false,
        )
        .unwrap();
        exercise(session, frame, geometry);
    }
}
#[test]
fn javascript_font_family_real_child_intrinsic_pollution_survives_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../tests/fixtures/javascript-fonts-integrity.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        frame.scene.root().style()["fontFamily"],
        "\"MissingOtterFamily\",\"Microsoft YaHei\",monospace"
    );
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(
        frame.scene.root().style()["fontFamily"],
        "\"Consolas\",monospace"
    );
    session.dispose(&|| false).unwrap();
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_font_family_actual_product_worker_uses_native_selection_and_geometry() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    for (source, geometry) in [(SOURCE, false), (GEOMETRY, true)] {
        let (session, frame) = Session::mount_with_viewport(
            fixture::binding(),
            source.into(),
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
        exercise(session, frame, geometry);
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

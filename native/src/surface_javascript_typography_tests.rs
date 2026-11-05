//! 通用 typography 通过真实 child、原文字 owner 和同步几何；不执行真实插件业务。
use super::*;
use sha2::{Digest, Sha256};
const SOURCES: [&str; 3] = [
    include_str!("../tests/fixtures/javascript-typography.js"),
    include_str!("../tests/fixtures/javascript-typography-geometry.js"),
    include_str!("../tests/fixtures/javascript-typography-integrity.js"),
];
fn view() -> crate::surface_viewport::Viewport {
    crate::surface_viewport::Viewport::new([600, 400], 96).unwrap()
}
fn check(frame: &Frame, case: usize, updated: bool) {
    let engine = crate::surface_text::Engine::new().unwrap();
    let base = crate::surface_viewport::style();
    let styles =
        crate::surface_text_styles::Styles::build(&frame.scene, base, view().context(), &engine)
            .unwrap();
    if case == 0 {
        for (index, node) in frame.scene.root().children().iter().enumerate() {
            let style = styles.get(node).unwrap();
            let (size, height, weight, family) = match (updated, index) {
                (false, 0) => (20., Some(30.), 700, "Segoe UI"),
                (false, _) => (12., None, 600, "Consolas"),
                (true, 0) => (24., Some(30.), 400, "Segoe UI"),
                (true, _) => (14., Some(28.), 700, "Segoe UI"),
            };
            assert_eq!(
                (style.size, style.line_height, style.weight, style.family),
                (size, height, weight, family)
            );
            let expected = engine
                .layout(
                    "WWii 0123",
                    crate::surface_text::Style {
                        size,
                        line_height: height,
                        weight,
                        family,
                        ..base
                    },
                    600.,
                    400.,
                )
                .unwrap();
            let actual = styles.layout_text(&engine, node, 600., 400.).unwrap();
            assert_eq!(actual.size(), expected.size());
            assert_eq!(actual.baseline(), expected.baseline());
        }
    } else if case == 1 {
        let report = frame.scene.root().children()[1].props()["text"]
            .as_str()
            .unwrap();
        let actual: Vec<f32> = serde_json::from_str(report).unwrap();
        let normal = engine
            .layout(
                "First\nSecond",
                crate::surface_text::Style {
                    size: 20.,
                    whitespace: crate::surface_text::Whitespace::Pre,
                    ..base
                },
                600.,
                400.,
            )
            .unwrap()
            .size()[1];
        assert_eq!(actual.len(), 5);
        for (actual, expected) in actual.iter().zip([48., 60., normal, 60., 48.]) {
            assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
        }
        assert_eq!(frame.viewport, Some(view()));
    } else {
        let style = styles.get(frame.scene.root()).unwrap();
        assert_eq!(style.size, if updated { 12. } else { 18. });
        assert_eq!(style.weight, if updated { 400 } else { 700 });
        assert_eq!(style.line_height, if updated { None } else { Some(22.5) });
    }
}
fn exercise(mut session: Session, frame: Frame, case: usize) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let id = frame.scene.root().id().to_owned();
    check(&frame, case, false);
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
    println!("native-javascript-typography: case={case} same-child=true metrics=true atomic=true reaped=true peak-job-bytes={peak}");
}
#[test]
fn javascript_typography_real_child_shorthand_inheritance_geometry_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for (case, source) in SOURCES.iter().enumerate() {
        let (session, frame) = Session::mount_in(
            fixture::binding(),
            (*source).into(),
            fixture::snapshot(3, 42),
            fixture::resources(),
            Some(view()),
            &|| false,
        )
        .unwrap();
        exercise(session, frame, case);
    }
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_typography_actual_product_worker_uses_same_font_and_line_height() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    for (case, source) in SOURCES.iter().enumerate() {
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
        exercise(session, frame, case);
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

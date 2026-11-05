//! 同生产预算真实 child/产品的省略、同步几何、更新、拒绝与回收。
use super::*;
use sha2::{Digest, Sha256};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-text-overflow.js");
pub(crate) fn view() -> crate::surface_viewport::Viewport {
    crate::surface_viewport::Viewport::new([400, 80], 96).unwrap()
}
fn check(frame: &Frame, width: u32, overflow: &str, inline: &str) {
    let nodes = frame.scene.root().children();
    assert_eq!(nodes[0].style()["textOverflow"], overflow);
    let report: Value = serde_json::from_str(nodes[1].props()["text"].as_str().unwrap()).unwrap();
    assert_eq!(report[0], width);
    assert_eq!(report[1], 32);
    assert_eq!(report[2], nodes[0].props()["text"]);
    // 首帧来自 author sheet，公开 inline getter 仍为空；不能伪造 computed reflection。
    assert_eq!(report[3], inline);
    assert!(frame.events.is_empty());
}
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let id = frame.scene.root().children()[0].id().to_owned();
    check(&frame, 84, "ellipsis", "");
    for (revision, width, overflow) in [(4, 84, "clip"), (5, 360, "ellipsis")] {
        let next = session
            .update(
                fixture::snapshot(revision, 45),
                fixture::resources(),
                &|| false,
            )
            .unwrap();
        assert_eq!(next.scene.root().children()[0].id(), id);
        check(&next, width, overflow, overflow);
    }
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_text_overflow_real_child_geometry_update_and_cleanup() {
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
fn javascript_text_overflow_real_child_rejects_unsupported_contexts_and_inline_runs() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for (tag, css, children) in [
        ("div", "text-overflow:clip", ""),
        ("button", "text-overflow:clip", ""),
        ("canvas", "text-overflow:clip", ""),
        ("input", "text-overflow:clip", ""),
        ("span", "text-overflow:ellipsis", ""),
        ("span", "text-overflow:ellipsis;overflow:hidden", ""),
        (
            "span",
            "text-overflow:ellipsis;overflow:hidden;white-space:normal",
            "",
        ),
        (
            "span",
            "text-overflow:ellipsis;overflow:hidden;white-space:pre",
            "",
        ),
        (
            "span",
            "text-overflow:ellipsis;overflow:hidden;white-space:pre-wrap",
            "",
        ),
        (
            "span",
            "text-overflow:ellipsis;overflow:hidden;white-space:nowrap;text-align:right",
            "",
        ),
        (
            "span",
            "text-overflow:ellipsis;overflow:hidden;white-space:nowrap;text-align:center",
            "",
        ),
        ("span", "overflow-x:hidden;overflow-y:visible", ""),
        ("span", "overflow:visible", ""),
        (
            "span",
            "overflow:hidden;white-space:nowrap;text-overflow:ellipsis",
            "<span style='text-overflow:clip'>run</span>",
        ),
    ] {
        let source=format!("NeuroSurface.define({{mount({{root}}){{root.innerHTML=\"<{tag}>{children}</{tag}>\";root.firstChild.style.cssText='{css}'}}}});");
        assert!(
            Session::mount_in(
                fixture::binding(),
                source,
                fixture::snapshot(3, 42),
                fixture::resources(),
                Some(view()),
                &|| false
            )
            .is_err(),
            "{tag} {css}"
        );
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_text_overflow_actual_product_worker_geometry_update_and_cleanup() {
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

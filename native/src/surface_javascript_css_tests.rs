//! 真实 child 的样式增量与预算；封存产品用外部 EXE 摘要显式验证，默认能力不放行。
use super::*;
use sha2::{Digest, Sha256};
const SOURCE: &str = include_str!("../tests/fixtures/javascript-css.js");
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let first = &frame.scene.root().children()[0];
    let target = first.id().to_owned();
    assert_eq!(first.layout()["width"], "32px");
    assert_eq!(first.style()["background"], "#00ff00");
    let frame = session.click(&target, &|| false).unwrap();
    assert_eq!(frame.events.len(), 1);
    assert_eq!(
        frame.scene.root().children()[0].style()["background"],
        "#0000ff"
    );
    assert!(!frame.scene.root().children()[1].visible());
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(frame.scene.root().children()[0].id(), target);
    assert_eq!(frame.scene.root().children()[0].layout()["width"], "60px");
    assert_eq!(
        frame.scene.root().children()[0].style()["background"],
        "#ffff00"
    );
    assert!(frame.scene.root().children()[1].visible() && frame.events.is_empty());
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    println!("native-javascript-css: cascade=true inline=true reclassify=true hidden=true replace=true reaped=true peak-job-bytes={peak}");
}
#[test]
fn javascript_css_real_child_cascade_inline_click_update_and_cleanup() {
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
fn javascript_css_real_child_invalid_sheet_and_inline_replacements_remain_atomic() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../tests/fixtures/javascript-css-rejection.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    assert_eq!(frame.scene.root().children()[0].style()["color"], "blue");
    session.dispose(&|| false).unwrap();
}
#[test]
fn javascript_css_real_child_private_css_survives_collect_update_and_dispose() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../tests/fixtures/javascript-css-integrity.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert_eq!(frame.scene.root().style()["opacity"], 0.5);
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(frame.scene.root().style()["background"], "blue");
    assert_eq!(frame.scene.root().style()["opacity"], 1);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
}
#[test]
fn javascript_css_real_child_rejects_unimplemented_box_inline_and_resource_budgets() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for operation in [
        "root.style.width='40px'", // 缺少显式 border-box，不能静默采用 native 语义。
        "root.style.fontSize='100px';root.style.lineHeight='1px'",
        "root.style.borderWidth='2px';root.style.borderStyle='solid'", // 缺少 border-box。
        "root.innerHTML='<button><span>run</span></button>';root.firstChild.firstChild.style.color='red'",
        "for(let i=0;i<17;i++)new CSSStyleSheet()",
        "new CSSStyleSheet().replaceSync('div{color:red}'.repeat(129))",
        "new CSSStyleSheet().replaceSync(' '.repeat(65537))",
        "const s=new CSSStyleSheet();s.replaceSync('div{color:red}'.repeat(128));document.adoptedStyleSheets=[s,s,s]",
    ] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{{operation}}}}});");
        assert!(Session::mount(fixture::binding(), source, fixture::snapshot(3, 42), fixture::resources(), &|| false).is_err(), "{operation}");
    }
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_css_actual_product_worker_cascade_and_updates() {
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

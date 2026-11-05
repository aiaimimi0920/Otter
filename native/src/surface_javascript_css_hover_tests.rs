//! Author :hover 贯穿真实 child/产品；不把合成事件或公开树属性当可信输入。
use super::*;
use sha2::{Digest, Sha256};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-css-hover.js");
fn view() -> crate::surface_viewport::Viewport {
    crate::surface_viewport::Viewport::new([200, 100], 96).unwrap()
}
fn mount(source: String) -> (Session, Frame) {
    Session::mount_in(
        fixture::binding(),
        source,
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(view()),
        &|| false,
    )
    .unwrap()
}
fn check(frame: &Frame, left: &str, right: &str, width: &str, moved: bool) {
    let groups = frame.scene.root().children();
    assert_eq!(groups[1].style()["background"], left);
    assert_eq!(groups[2].style()["background"], right);
    let a = &groups[if moved { 2 } else { 1 }].children()[0];
    assert_eq!(a.layout()["width"], width);
    assert!(frame.events.is_empty());
}
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let a = frame.scene.root().children()[1].children()[0]
        .id()
        .to_owned();
    let b = frame.scene.root().children()[1].children()[1]
        .id()
        .to_owned();
    check(&frame, "#0000ff", "#0000ff", "20px", false);
    assert_eq!(fixture::text(&frame.scene), "synthetic");
    let frame = session
        .local_input(hover_tests::input(Some(&a)), &|| false)
        .unwrap();
    check(&frame, "#00ff00", "#0000ff", "30px", false);
    assert_eq!(fixture::text(&frame.scene), "enter|30|24");
    assert_eq!(frame.scene.root().style()["color"], "#ffffff");
    assert_eq!(
        frame.scene.root().children()[1].children()[1].layout()["height"],
        "24px"
    );
    let frame = session
        .local_input(hover_tests::input(Some(&b)), &|| false)
        .unwrap();
    check(&frame, "#00ff00", "#0000ff", "20px", false);
    assert_eq!(
        frame.scene.root().children()[1].children()[1].layout()["width"],
        "30px"
    );
    session
        .local_input(hover_tests::input(Some(&a)), &|| false)
        .unwrap();
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    check(&frame, "#0000ff", "#00ff00", "30px", true);
    assert_eq!(frame.scene.root().children()[2].children()[0].id(), a);
    let frame = session
        .update(fixture::snapshot(5, 46), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(frame.scene.root().style()["color"], "#000000");
    for group in &frame.scene.root().children()[1..] {
        assert_eq!(group.style()["background"], "#0000ff");
    }
    let frame = session
        .update(fixture::snapshot(6, 47), fixture::resources(), &|| false)
        .unwrap();
    check(&frame, "#0000ff", "#00ff00", "30px", true);
    session.suspend(&|| false).unwrap();
    let frame = session.resume(&|| false).unwrap();
    check(&frame, "#0000ff", "#0000ff", "20px", true);
    assert_eq!(fixture::text(&frame.scene), "suspended");
    session
        .local_input(hover_tests::input(Some(&a)), &|| false)
        .unwrap();
    let frame = session
        .local_input(hover_tests::input(None), &|| false)
        .unwrap();
    check(&frame, "#0000ff", "#0000ff", "20px", true);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_css_hover_real_child_ancestor_specificity_geometry_mutation_and_lifetime() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = mount(SOURCE.into());
    exercise(session, frame);
}
fn polluted() -> String {
    format!(
        "{SOURCE}\n{}",
        r#"
      Object.defineProperty(HTMLElement.prototype,'parentElement',{get(){throw Error('public parent')}});
      Object.defineProperty(Node.prototype,'parentNode',{get(){throw Error('public parent')}});
      Object.defineProperty(HTMLElement.prototype,'contains',{value(){throw Error('public contains')}});
      Map.prototype.get=WeakMap.prototype.get=Array.prototype.push=()=>{throw Error('intrinsic')};
    "#
    )
}
#[test]
fn javascript_css_hover_real_child_uses_private_parents_and_captured_intrinsics() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = mount(polluted());
    exercise(session, frame);
}
#[test]
fn javascript_css_hover_real_child_invalid_selectors_are_atomic_and_do_not_alias() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"NeuroSurface.define({mount({root}){
      const sheet=new CSSStyleSheet();sheet.replaceSync('div{color:blue}div:hover{color:red}');
      document.adoptedStyleSheets=[sheet];root.innerHTML='<div></div>';
      for(const bad of [':hovering',':hover()',':hovered',':hover-x',':active',':focus-visible']){
        let rejected=false;try{sheet.replaceSync('div'+bad+'{color:green}')}catch{rejected=true}
        if(!rejected)throw Error('invalid hover selector accepted');
      }
    }});"#;
    let (mut session, frame) = mount(source.into());
    let id = frame.scene.root().children()[0].id().to_owned();
    assert_eq!(frame.scene.root().children()[0].style()["color"], "blue");
    let frame = session
        .local_input(hover_tests::input(Some(&id)), &|| false)
        .unwrap();
    assert_eq!(frame.scene.root().children()[0].style()["color"], "red");
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_css_hover_actual_product_worker_styles_geometry_and_cleanup() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    for source in [SOURCE.to_owned(), polluted()] {
        let (session, frame) = geometry_tests::mount_product(&path, &source, 42, view());
        exercise(session, frame);
    }
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

//! 焦点接口先在真实受限 child 求值；实际 HWND 默认失焦提交另由桌面测试保护。
use super::*;
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-native-focus.js");
#[test]
fn javascript_native_focus_real_child_mounts_supported_methods() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    exercise(&mut session, frame);
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
fn exercise(session: &mut Session, frame: Frame) {
    assert_eq!(fixture::text(&frame.scene), "ready");
    assert!(frame.focus.is_empty() && frame.events.is_empty());
    let field = frame.scene.root().children()[1].id().to_owned();
    let focus = frame.scene.root().children()[2].id().to_owned();
    let blur = frame.scene.root().children()[3].id().to_owned();
    for (button, expected) in [(focus, true), (blur, false)] {
        let frame = session.click(&button, &|| false).unwrap();
        assert_eq!(frame.focus.len(), 1);
        assert_eq!(frame.focus[0].target, field);
        assert_eq!(frame.focus[0].focus, expected);
        assert!(frame.events.is_empty());
    }
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert!(frame.focus.is_empty());
    assert_eq!(
        frame.scene.root().children()[1].props()["value"],
        "server:45"
    );
    assert!(session.suspend(&|| false).unwrap().focus.is_empty());
    assert!(session.resume(&|| false).unwrap().focus.is_empty());
}
#[test]
fn javascript_native_focus_private_requests_survive_public_reflection_and_intrinsic_pollution() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../tests/fixtures/javascript-focus-integrity.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let field = frame.scene.root().children()[1].id().to_owned();
    let button = frame.scene.root().children()[2].id().to_owned();
    let frame = session.click(&button, &|| false).unwrap();
    assert_eq!(frame.focus.len(), 2);
    assert!(frame.focus.iter().all(|request| request.target == field));
    assert!(frame.focus[0].focus && !frame.focus[1].focus);
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_native_focus_bounded_requests_and_lifecycle_cleanup_reap_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for count in [32, 33] {
        let source = format!("let field;NeuroSurface.define({{mount({{root}}){{root.innerHTML='<p>bounded</p><input><button>run</button>';field=root.children[1];root.children[2].addEventListener('click',()=>{{for(let i=0;i<{count};i++)field.focus();}});}},suspend(){{field.focus();}},dispose(){{field.blur();}}}});");
        let (mut session, frame) = Session::mount(
            fixture::binding(),
            source,
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        let probe = session.child.as_ref().unwrap().probe().unwrap();
        let next = session.click(frame.scene.root().children()[2].id(), &|| false);
        if count == 32 {
            assert_eq!(next.unwrap().focus.len(), 32);
            assert!(session.suspend(&|| false).unwrap().focus.is_empty());
            session.dispose(&|| false).unwrap();
        } else {
            assert!(next.is_err());
        }
        assert!(probe.exited());
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
fn javascript_native_focus_wire_rejects_unknown_fields_bad_ids_and_overflow() {
    for value in [
        serde_json::json!({"target":"field","focus":true,"hwnd":42}),
        serde_json::json!({"target":"field","focus":"yes"}),
    ] {
        assert!(serde_json::from_value::<wire::focus::Request>(value).is_err());
    }
    for target in ["", "field/other", "field\0", &"x".repeat(257)] {
        assert!(wire::focus::validate(&[wire::focus::Request {
            target: target.into(),
            focus: true
        }])
        .is_err());
    }
    let requests = (0..33)
        .map(|_| wire::focus::Request {
            target: "field".into(),
            focus: true,
        })
        .collect::<Vec<_>>();
    assert!(wire::focus::validate(&requests).is_err());
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_native_focus_actual_product_worker_uses_same_bounded_requests() {
    use sha2::{Digest, Sha256};
    let _serial = fixture::SERIAL.lock().unwrap();
    let path = std::path::PathBuf::from(std::env::var_os("OTTER_JAVASCRIPT_PRODUCT_EXE").unwrap());
    let expected = std::env::var("OTTER_JAVASCRIPT_PRODUCT_SHA256").unwrap();
    assert!(path.is_absolute());
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
    let (mut session, frame) = geometry_tests::mount_product(
        &path,
        SOURCE,
        42,
        crate::surface_viewport::Viewport::new([320, 200], 96).unwrap(),
    );
    exercise(&mut session, frame);
    session.dispose(&|| false).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap())),
        expected
    );
}

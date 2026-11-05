//! 原生文本输入进入真实 QuickJS child；后续与 HWND/Application 共用夹具。
use super::*;
use wire::{InputPhase, TextInput};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-native-input.js");
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let field = frame.scene.root().children()[1].id().to_owned();
    let button = frame.scene.root().children()[2].id().to_owned();
    assert_eq!(
        frame.scene.root().children()[1].kind(),
        crate::surface_scene::Kind::Input
    );
    assert_eq!(frame.scene.root().children()[1].props()["value"], "初始🙂");
    for (phase, expected) in [
        (InputPhase::Input, "input|键入🙂|1"),
        (InputPhase::Change, "change|键入🙂|2"),
    ] {
        let next = session
            .local_input(
                TextInput {
                    target: field.clone(),
                    phase,
                    value: "键入🙂".into(),
                },
                &|| false,
            )
            .unwrap();
        assert_eq!(fixture::text(&next.scene), expected);
        assert_eq!(next.scene.root().children()[1].id(), field);
        assert_eq!(
            next.events.len(),
            if matches!(phase, InputPhase::Change) {
                2
            } else {
                0
            }
        );
    }
    let next = session.click(&button, &|| false).unwrap();
    assert_eq!(fixture::text(&next.scene), "click|键入🙂|2");
    let next = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(
        next.scene.root().children()[1].props()["value"],
        "server:45"
    );
    session.suspend(&|| false).unwrap();
    assert!(session
        .local_input(
            TextInput {
                target: field.clone(),
                phase: InputPhase::Input,
                value: "paused".into()
            },
            &|| false
        )
        .is_err());
    session.resume(&|| false).unwrap();
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_native_input_real_child_projects_text_control() {
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
fn javascript_native_input_invalid_target_disabled_or_callback_budget_reaps_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for body in [
        "field.disabled=true;",
        "field.remove();",
        "field.addEventListener('input',()=>{throw Error('rejected')});",
        "field.addEventListener('input',()=>{while(true){}});",
    ] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{const field=document.createElement('input');root.appendChild(field);{body}}}}});");
        let (mut session, frame) = Session::mount(
            fixture::binding(),
            source,
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        let probe = session.child.as_ref().unwrap().probe().unwrap();
        let target = frame
            .scene
            .root()
            .children()
            .first()
            .map(|n| n.id())
            .unwrap_or("dom:2")
            .to_owned();
        let start = Instant::now();
        assert!(session
            .local_input(
                TextInput {
                    target,
                    phase: InputPhase::Input,
                    value: "input".into()
                },
                &|| false
            )
            .is_err());
        assert!(start.elapsed() < Duration::from_secs(4));
        assert!(probe.exited());
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_native_input_actual_product_worker_uses_same_value_and_dispatch() {
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
#[test]
fn javascript_native_input_private_value_survives_intrinsic_pollution_and_no_handler() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for source in [
        include_str!("../tests/fixtures/javascript-input-integrity.js"),
        "NeuroSurface.define({mount({root}){root.innerHTML='<p>no-handler</p><input>';}});",
    ] {
        let (mut session, frame) = Session::mount(
            fixture::binding(),
            source.into(),
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        let target = frame.scene.root().children()[1].id().to_owned();
        let next = session
            .local_input(
                TextInput {
                    target,
                    phase: InputPhase::Input,
                    value: "安全🙂".into(),
                },
                &|| false,
            )
            .unwrap();
        assert_eq!(next.scene.root().children()[1].props()["value"], "安全🙂");
        assert!(next.events.is_empty());
        session.dispose(&|| false).unwrap();
        assert_eq!(CAPACITY.used(), 0);
    }
}

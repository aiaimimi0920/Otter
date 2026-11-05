//! 相同 Canvas Pointer 夹具贯穿真实 child 和 SHA 绑定产品 worker。
use super::*;
use wire::{LocalInput, PointerInput, PointerPhase};
#[test]
fn javascript_pointer_up_hiding_self_or_ancestor_including_stylesheet_microtask_suppresses_click() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for body in ["c.style.display='none'", "root.style.display='none'", "Promise.resolve().then(()=>{const sheet=new CSSStyleSheet();sheet.replaceSync('canvas { display:none }');document.adoptedStyleSheets=[sheet]})"] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{const c=document.createElement('canvas');c.width=c.height=4;root.append(c);c.addEventListener('pointerup',()=>{{{body}}});c.addEventListener('click',()=>{{throw Error('hidden click')}});}}}});");
        let (mut session, frame) = Session::mount(fixture::binding(), source, fixture::snapshot(3, 42), fixture::resources(), &|| false).unwrap();
        session.local_input(input(frame.scene.root().children()[0].id(), PointerPhase::Pointerup, true), &|| false).unwrap();
        session.dispose(&|| false).unwrap(); assert_eq!(CAPACITY.used(), 0);
    }
}
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-native-pointer.js");
pub(crate) fn input(target: &str, phase: PointerPhase, click: bool) -> LocalInput {
    LocalInput::Pointer(PointerInput {
        target: target.into(),
        phase,
        client: [10.5, -2.],
        modifiers: 15,
        click,
    })
}
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let target = frame.scene.root().children()[1].id();
    let mut expected = String::new();
    for (phase, label, click) in [
        (PointerPhase::Pointerdown, "pointerdown", false),
        (PointerPhase::Pointermove, "pointermove", false),
        (PointerPhase::Pointerup, "pointerup>click", true),
        (PointerPhase::Pointerdown, "pointerdown", false),
        (PointerPhase::Pointercancel, "pointercancel", false),
    ] {
        let next = session
            .local_input(input(target, phase, click), &|| false)
            .unwrap();
        if !expected.is_empty() {
            expected.push('>');
        }
        expected.push_str(label);
        assert_eq!(fixture::text(&next.scene), expected);
        assert!(next.default_allowed.is_none() && next.events.is_empty());
        assert_eq!(next.scene.root().children()[1].id(), target);
        assert_eq!(next.images.len(), 1);
    }
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_pointer_real_child_canvas_sequence_click_cancel_same_identity_and_cleanup() {
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
fn javascript_pointer_up_listener_detach_suppresses_click_and_cancel_can_reach_detached_identity() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"NeuroSurface.define({mount({root}){const p=document.createElement('p'),c=document.createElement('canvas');c.width=c.height=4;root.append(p,c);
      c.addEventListener('pointerup',()=>c.remove()); c.addEventListener('click',()=>{throw Error('detached click')});
      c.addEventListener('pointercancel',()=>p.textContent='canceled');}});"#;
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let target = frame.scene.root().children()[1].id();
    let next = session
        .local_input(input(target, PointerPhase::Pointerup, true), &|| false)
        .unwrap();
    assert_eq!(next.scene.root().children().len(), 1);
    let next = session
        .local_input(input(target, PointerPhase::Pointercancel, false), &|| false)
        .unwrap();
    assert_eq!(fixture::text(&next.scene), "canceled");
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_pointer_actual_product_worker_canvas_sequence_and_cleanup() {
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
fn javascript_pointer_native_fields_ignore_public_constructors_and_private_slot_pollution() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"NeuroSurface.define({mount({root}){
      const c=document.createElement('canvas');c.width=c.height=4;root.append(c);const Type=PointerEvent;
      c.addEventListener('pointerdown',e=>{if(!(e instanceof Type)||!e.isTrusted||e.clientX!==10.5||e.clientY!==-2||
        !e.ctrlKey||!e.shiftKey||!e.altKey||!e.metaKey||e.button!==0||e.buttons!==1)throw Error('native fields');});
      Object.defineProperty(Object.prototype,'mouse',{configurable:true,get(){throw Error('mouse get')},set(){throw Error('mouse set')}});
      Object.defineProperty(Object.prototype,'pointer',{configurable:true,get(){throw Error('pointer get')},set(){throw Error('pointer set')}});
      try{globalThis.PointerEvent=()=>{throw Error('constructor')}}catch{}
      if(globalThis.PointerEvent!==Type)throw Error('global constructor changed');
      WeakMap.prototype.get=Map.prototype.get=Array.prototype.push=Promise.prototype.then=()=>{throw Error('intrinsic')};
      Object.defineProperty(HTMLCanvasElement.prototype,'then',{get(){throw Error('node then')}});
    }});"#;
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    session
        .local_input(
            input(
                frame.scene.root().children()[0].id(),
                PointerPhase::Pointerdown,
                false,
            ),
            &|| false,
        )
        .unwrap();
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_pointer_listener_failure_reaps_child_without_replay() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for body in ["throw Error('pointer rejected')", "while(true){}"] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{const c=document.createElement('canvas');c.width=c.height=4;root.append(c);c.addEventListener('pointerdown',()=>{{{body}}});}}}});");
        let (mut session, frame) = Session::mount(
            fixture::binding(),
            source,
            fixture::snapshot(3, 42),
            fixture::resources(),
            &|| false,
        )
        .unwrap();
        let probe = session.child.as_ref().unwrap().probe().unwrap();
        assert!(session
            .local_input(
                input(
                    frame.scene.root().children()[0].id(),
                    PointerPhase::Pointerdown,
                    false
                ),
                &|| false
            )
            .is_err());
        assert!(probe.exited());
        assert_eq!(CAPACITY.used(), 0);
    }
}

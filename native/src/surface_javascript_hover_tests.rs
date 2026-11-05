//! 同一通用悬停夹具贯穿真实 child；产品路径须额外绑定实际 EXE 摘要。
use super::*;
use wire::{HoverInput, LocalInput};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-native-hover.js");
pub(crate) fn input(target: Option<&str>) -> LocalInput {
    LocalInput::Hover(HoverInput {
        target: target.map(str::to_owned),
        client: [10.5, -2.],
        modifiers: 15,
    })
}
pub(crate) const ENTER: &str = "pointerover:a:null>pointerover:w:null>pointerenter:w:null>pointerenter:a:null>pointermove:a:null>pointermove:w:null";
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let a = frame.scene.root().children()[1].children()[0]
        .id()
        .to_owned();
    let b = frame.scene.root().children()[1].children()[1]
        .id()
        .to_owned();
    let mut expected = String::new();
    for (target, suffix) in [
        (Some(a.as_str()), ENTER),
        (Some(a.as_str()), "pointermove:a:null>pointermove:w:null"),
        (Some(b.as_str()), "pointerout:a:b>pointerout:w:b>pointerleave:a:b>pointerover:b:a>pointerover:w:a>pointerenter:b:a>pointermove:b:null>pointermove:w:null"),
        (None, "pointerout:b:null>pointerout:w:null>pointerleave:b:null>pointerleave:w:null"),
    ] {
        let next = session.local_input(input(target), &|| false).unwrap();
        assert!(next.animation_pending && next.events.is_empty() && next.default_allowed.is_none());
        let next = session.animation_frame(&|| false).unwrap();
        if !expected.is_empty() { expected.push('>'); } expected.push_str(suffix);
        assert_eq!(fixture::text(&next.scene), expected);
        assert_eq!(next.scene.root().children()[1].children()[0].id(), a);
        assert_eq!(next.images.len(), 2);
    }
    let next = session.local_input(input(None), &|| false).unwrap();
    assert!(!next.animation_pending);
    assert_eq!(fixture::text(&next.scene), expected);
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_hover_real_child_sibling_order_related_identity_nonbubble_leave_and_raf() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (session, frame) = Session::mount_in(
        fixture::binding(),
        SOURCE.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        Some(crate::surface_viewport::Viewport::new([320, 200], 96).unwrap()),
        &|| false,
    )
    .unwrap();
    exercise(session, frame);
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_hover_actual_product_worker_sibling_order_and_cleanup() {
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
fn javascript_hover_detached_previous_target_leave_and_text_projection_are_safe() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"NeuroSurface.define({mount({root}){const p=document.createElement('p'),w=document.createElement('div'),t=document.createTextNode('text');root.append(p,w);w.append(t);
      w.addEventListener('pointermove',e=>{if(e.clientX!==10.5||e.clientY!==-2||!e.ctrlKey||!e.shiftKey||!e.altKey||!e.metaKey)throw Error('DIP/modifier');w.remove()});
      w.addEventListener('pointerleave',()=>p.textContent='left');}});"#;
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let text = frame.scene.root().children()[1].children()[0]
        .id()
        .to_owned();
    session.local_input(input(Some(&text)), &|| false).unwrap();
    let next = session.local_input(input(None), &|| false).unwrap();
    assert_eq!(fixture::text(&next.scene), "left");
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_hover_listener_failure_and_loop_reap_child_without_replay() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for body in ["throw Error('hover rejected')", "while(true){}"] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{const n=document.createElement('div');root.append(n);n.addEventListener('pointermove',()=>{{{body}}});}}}});");
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
            .local_input(input(Some(frame.scene.root().children()[0].id())), &|| {
                false
            })
            .is_err());
        assert!(probe.exited());
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
fn javascript_hover_native_related_target_and_modifiers_ignore_public_prototype_pollution() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"NeuroSurface.define({mount({root}){
      const a=document.createElement('div'),b=document.createElement('div');root.append(a,b);
      const getter=Object.getOwnPropertyDescriptor(MouseEvent.prototype,'relatedTarget').get;
      let rejected=0;try{getter.call(new Event('x'))}catch{rejected++}try{new MouseEvent('x',{relatedTarget:{}})}catch{rejected++}
      if(rejected!==2||new MouseEvent('x',{relatedTarget:a}).relatedTarget!==a)throw Error('relatedTarget brand');
      b.addEventListener('pointerover',e=>{if(e.relatedTarget!==a||e.clientX!==10.5||e.clientY!==-2||!e.ctrlKey||!e.shiftKey||!e.altKey||!e.metaKey)throw Error('native relatedTarget');});
      Object.defineProperty(Object.prototype,'relatedTarget',{configurable:true,get(){throw Error('related get')},set(){throw Error('related set')}});
      Object.defineProperty(HTMLElement.prototype,'then',{get(){throw Error('node then')}});
      Map.prototype.get=WeakMap.prototype.get=Array.prototype.push=Promise.prototype.then=()=>{throw Error('intrinsic')};
    }});"#;
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    for child in frame.scene.root().children() {
        session
            .local_input(input(Some(child.id())), &|| false)
            .unwrap();
    }
    session.local_input(input(None), &|| false).unwrap();
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}

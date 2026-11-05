//! 真实 child / 实际产品共用 Enter 夹具；取消结果只回宿主，不伪造默认 change。
use super::*;
use wire::{KeyPhase, KeyboardInput};
pub(crate) const SOURCE: &str = include_str!("../tests/fixtures/javascript-native-enter.js");
pub(crate) fn enter(target: &str, flags: u32) -> KeyboardInput {
    KeyboardInput {
        target: target.into(),
        flags,
        phase: if flags & 1 == 0 {
            KeyPhase::Keydown
        } else {
            KeyPhase::Keyup
        },
    }
}
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let target = frame.scene.root().children()[1].id();
    for (flags, expected, allowed) in [
        (0, "down:cancel:first", false),
        (1, "down:cancel:first>up", true),
        (2, "down:cancel:first>up>down:allow:repeat", true),
        (
            0,
            "down:cancel:first>up>down:allow:repeat>down:allow:first",
            true,
        ),
    ] {
        let next = session
            .local_input(enter(target, flags), &|| false)
            .unwrap();
        assert_eq!(next.default_allowed, Some(allowed));
        assert_eq!(fixture::text(&next.scene), expected);
        assert!(next.events.is_empty());
        assert_eq!(next.scene.root().children()[1].props()["value"], "initial");
    }
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_native_enter_real_child_returns_cancel_before_any_default() {
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
fn javascript_native_enter_keyboard_fields_synthetic_passive_and_private_cancel() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"NeuroSurface.define({mount({root}){
      const field=document.createElement('input');root.appendChild(field);
      const synthetic=new KeyboardEvent('keydown',{key:'A',code:'KeyA',location:1,ctrlKey:true,bubbles:true,cancelable:true});
      if(!(synthetic instanceof Event)||synthetic.isTrusted||synthetic.key!=='A'||synthetic.code!=='KeyA'||!synthetic.getModifierState('Control')||synthetic.getModifierState('Shift'))throw Error('synthetic');
      let got=false; field.addEventListener('keydown',e=>{ if(!e.isTrusted){got=true;return;}
        if(e.key!=='Enter'||e.code!=='NumpadEnter'||e.location!==3||!e.ctrlKey||!e.shiftKey||!e.altKey||!e.metaKey||!e.repeat||e.isComposing)throw Error('native fields');
        Object.defineProperty(e,'defaultPrevented',{value:false}); e.returnValue=false;
      });
      field.dispatchEvent(synthetic);if(!got)throw Error('dispatch');
      const cancel=Event.prototype.preventDefault;
      field.addEventListener('keyup',e=>cancel.call(e),{passive:true});
      Map.prototype.get=WeakMap.prototype.get=Array.prototype.push=()=>{throw Error('pollution')};
      KeyboardEvent.prototype.preventDefault=()=>{throw Error('public cancel')};
      Object.defineProperty(HTMLInputElement.prototype,'value',{get(){throw Error('public value')}});
    }});"#;
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let target = frame.scene.root().children()[0].id();
    assert_eq!(
        session
            .local_input(enter(target, 126), &|| false)
            .unwrap()
            .default_allowed,
        Some(false)
    );
    assert_eq!(
        session
            .local_input(enter(target, 65), &|| false)
            .unwrap()
            .default_allowed,
        Some(true)
    );
    session.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_native_enter_callback_failure_and_budget_reap_child() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for body in ["throw Error('rejected')", "while(true){}"] {
        let source=format!("NeuroSurface.define({{mount({{root}}){{const f=document.createElement('input');root.appendChild(f);f.addEventListener('keydown',()=>{{{body}}});}}}});");
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
            .local_input(enter(frame.scene.root().children()[0].id(), 0), &|| {
                false
            })
            .is_err());
        assert!(probe.exited());
        assert_eq!(CAPACITY.used(), 0);
    }
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_native_enter_actual_product_worker_uses_same_cancel_result() {
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

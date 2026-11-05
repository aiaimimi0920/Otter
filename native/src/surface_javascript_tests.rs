use super::fixture::{binding, resources, snapshot, text, SOURCE};
use super::*;
use serde_json::json;
#[test]
fn javascript_real_child_preserves_state_updates_emits_and_disposes() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, frame) = Session::mount(
        binding(),
        SOURCE.into(),
        snapshot(3, 42),
        resources(),
        &|| false,
    )
    .unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert_eq!(text(&frame.scene), "原生🙂:42:0");
    assert!(frame.events.is_empty());
    let button = frame.scene.root().children()[1].id().to_owned();
    let frame = session.click(&button, &|| false).unwrap();
    assert_eq!(text(&frame.scene), "原生🙂:42:1");
    assert_eq!(frame.events.len(), 1);
    let event: Value = serde_json::from_slice(frame.events[0].bytes()).unwrap();
    assert_eq!(event["generation"], 7);
    assert_eq!(event["baseRevision"], 3);
    assert_eq!(event["attachmentId"], "attachment:javascript");
    assert_eq!(event["payload"]["count"], 1);
    assert_eq!(event["payload"]["label"], "原生🙂");
    let frame = session
        .update(snapshot(4, 43), resources(), &|| false)
        .unwrap();
    assert_eq!(text(&frame.scene), "原生🙂:43:1");
    let frame = session.click(&button, &|| false).unwrap();
    let event: Value = serde_json::from_slice(frame.events[0].bytes()).unwrap();
    assert_eq!(event["baseRevision"], 4);
    assert_eq!(event["payload"]["count"], 2);
    assert_eq!(event["payload"]["quote"], 43);
    assert!(session
        .update(snapshot(4, 999), resources(), &|| false)
        .is_err());
    assert_eq!(text(&session.suspend(&|| false).unwrap().scene), "paused");
    assert!(session.click(&button, &|| false).is_err());
    assert_eq!(
        text(&session.resume(&|| false).unwrap().scene),
        "原生🙂:43:2"
    );
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
}
#[test]
fn javascript_child_abort_and_drop_reclaim_capacity_and_process() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, _) = Session::mount(
        binding(),
        SOURCE.into(),
        snapshot(1, 1),
        resources(),
        &|| false,
    )
    .unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert!(session
        .update(snapshot(2, 2), resources(), &|| true)
        .is_err());
    assert!(probe.exited());
    assert!(session.resume(&|| false).is_err());
    let (session, _) = Session::mount(
        binding(),
        SOURCE.into(),
        snapshot(1, 1),
        resources(),
        &|| false,
    )
    .unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    drop(session);
    assert!(probe.exited());
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_cpu_heap_unresolved_promise_and_import_fail_closed() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for source in [
        "while(true){}",
        "import fs from 'std';",
        "import fs from 'file:///C:/Windows/win.ini';",
        "await import('https://example.com/script.js');",
        "await new Promise(() => {});",
        "NeuroSurface.define({mount(){ const a=[]; while(true) a.push(new Uint8Array(1048576)); }});",
        "NeuroSurface.define({mount(){ function again(){Promise.resolve().then(again)} again(); }});",
        "NeuroSurface.define({mount({root}){root.innerHTML='<script>unsupported</script>';}});",
        "NeuroSurface.define({mount({root}){const s=document.createElement('span'); s.appendChild(document.createElement('button'));root.appendChild(s);}});",
    ] {
        let now = Instant::now();
        assert!(Session::mount(binding(), source.into(), snapshot(1, 1), resources(), &|| false).is_err(), "{source}");
        assert!(now.elapsed() < Duration::from_secs(4));
    }
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_parent_does_not_trust_emitted_binding_or_action() {
    let (authority, revision) = super::snapshot(&snapshot(3, 42)).unwrap();
    let mut value = wire::Emitted {
        node_id: "refresh".into(),
        event: "click".into(),
        action: "refresh".into(),
        class: "discrete".into(),
        payload: json!(null),
    };
    assert!(event::encode(&value, &authority, &binding(), revision).is_ok());
    for name in ["other", "with.dot", "bad\n"] {
        value.node_id = name.into();
        assert!(event::encode(&value, &authority, &binding(), revision).is_err());
    }
    value.node_id = "refresh".into();
    value.action = "forged".into();
    assert!(event::encode(&value, &authority, &binding(), revision).is_err());
    value.action = "refresh".into();
    value.class = "local".into();
    assert!(event::encode(&value, &authority, &binding(), revision).is_err());
    value.class = "commit".into();
    value.payload = json!("中".repeat(30000));
    assert!(event::encode(&value, &authority, &binding(), revision).is_err());
}
#[test]
fn javascript_wire_limits_precede_allocation_and_reject_trailing_or_unknown_fields() {
    let mut bytes = b"OJS1".to_vec();
    bytes.extend_from_slice(&((wire::MAX_FRAME + 1) as u32).to_le_bytes());
    assert!(wire::read::<wire::Response>(&mut bytes.as_slice()).is_err());
    assert!(wire::encode(&"x".repeat(wire::MAX_FRAME)).is_err());
    let mut bytes = Vec::new();
    wire::write(
        &mut bytes,
        &json!({"sequence":1,"output":null,"unexpected":true}),
    )
    .unwrap();
    assert!(wire::read::<wire::Response>(&mut bytes.as_slice()).is_err());
    assert!(super::snapshot(&json!({"revision":0,"scene":{}})).is_err());
    assert!(super::resources(&json!({"bad/path":"data:"})).is_err());
}
#[test]
fn javascript_emit_budget_returns_false_without_poisoning_live_session() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"NeuroSurface.define({mount({root,emit}){
      const event = {nodeId:'refresh',event:'click',action:'refresh',class:'discrete'};
      const accepted=emit({...event,payload:'x'.repeat(60*1024-2)});
      const ascii=emit({...event,payload:'x'.repeat(60*1024-1)});
      const unicode=emit({...event,payload:'中'.repeat(20480)});
      root.textContent=`${accepted}:${ascii}:${unicode}`;
    }});"#;
    let (session, frame) = Session::mount(
        binding(),
        source.into(),
        snapshot(1, 1),
        resources(),
        &|| false,
    )
    .unwrap();
    assert_eq!(text(&frame.scene), "true:false:false");
    assert_eq!(frame.events.len(), 1);
    assert!(frame.events[0].bytes().len() <= crate::surface_click_event::MAX_EVENT_BYTES);
    session.dispose(&|| false).unwrap();
}
#[test]
fn javascript_emit_budget_preserves_exact_json_utf8_boundary_and_surrogate_pairs() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for expression in [
        "'x'.repeat(61438)",
        "'¢'.repeat(30719)",
        "'中'.repeat(20479)+'x'",
        "'🙂'.repeat(15359)+'xx'",
        r"'\n'.repeat(30719)",
        r#"'\"'.repeat(30719)"#,
        r"'\\'.repeat(30719)",
    ] {
        let source = format!(
            r#"NeuroSurface.define({{mount({{root,emit}}){{
              const event={{nodeId:'refresh',event:'click',action:'refresh',class:'discrete'}};
              const payload={expression};
              root.textContent=`${{emit({{...event,payload}})}}:${{emit({{...event,payload:payload+'x'}})}}`;
            }}}});"#
        );
        let started = Instant::now();
        let (session, frame) =
            Session::mount(binding(), source, snapshot(1, 1), resources(), &|| false)
                .unwrap_or_else(|error| {
                    panic!(
                        "emit boundary {expression} failed after {:?}: {error}",
                        started.elapsed()
                    )
                });
        eprintln!("emit boundary {expression}: {:?}", started.elapsed());
        assert_eq!(text(&frame.scene), "true:false", "{expression}");
        assert_eq!(frame.events.len(), 1);
        let event: Value = serde_json::from_slice(frame.events[0].bytes()).unwrap();
        assert_eq!(
            serde_json::to_vec(&event["payload"]).unwrap().len(),
            60 * 1024
        );
        session.dispose(&|| false).unwrap();
    }
}
#[test]
fn javascript_two_sessions_keep_independent_closures_and_enforce_capacity() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut first, initial) = Session::mount(
        binding(),
        SOURCE.into(),
        snapshot(1, 42),
        resources(),
        &|| false,
    )
    .unwrap();
    let (mut second, _) = Session::mount(
        binding(),
        SOURCE.into(),
        snapshot(1, 45),
        resources(),
        &|| false,
    )
    .unwrap();
    let target = initial.scene.root().children()[1].id();
    assert!(Session::mount(
        binding(),
        SOURCE.into(),
        snapshot(1, 99),
        resources(),
        &|| false
    )
    .is_err());
    assert_eq!(
        text(&first.click(target, &|| false).unwrap().scene),
        "原生🙂:42:1"
    );
    assert_eq!(
        text(&second.click(target, &|| false).unwrap().scene),
        "原生🙂:45:1"
    );
    first.dispose(&|| false).unwrap();
    second.dispose(&|| false).unwrap();
    assert_eq!(CAPACITY.used(), 0);
}

#[test]
fn javascript_diagnostic_real_child_failure_keeps_error_and_reclaims_original_owner() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = "NeuroSurface.define({mount({root}){root.textContent='ready'},update(){throw Error('PRIVATE_CHILD_MARKER')}});";
    let (mut session, _) = Session::mount(
        binding(),
        source.into(),
        snapshot(1, 1),
        resources(),
        &|| false,
    )
    .unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let error = session
        .update(snapshot(2, 2), resources(), &|| false)
        .err()
        .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "Invalid JavaScript Surface operation");
    assert!(session.child.is_none());
    assert!(probe.exited());
    assert!(session.resume(&|| false).is_err());
    drop(session);
    assert_eq!(CAPACITY.used(), 0);
}

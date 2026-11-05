//! 文档子集在真实 child 中执行；产品门另需外部摘要，不能从缓存自我授权。
use super::*;
use sha2::{Digest, Sha256};
const SOURCE: &str = include_str!("../tests/fixtures/javascript-document.js");
fn button(scene: &Scene) -> String {
    fn find(node: &crate::surface_scene::Node) -> Option<&str> {
        if node.kind() == crate::surface_scene::Kind::Button {
            Some(node.id())
        } else {
            node.children().iter().find_map(find)
        }
    }
    find(scene.root()).unwrap().to_owned()
}
fn exercise(mut session: Session, frame: Frame) {
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    let target = button(&frame.scene);
    assert_eq!(fixture::text(&frame.scene), "ready");
    let frame = session.click(&target, &|| false).unwrap();
    assert_eq!(fixture::text(&frame.scene), "clicked:1");
    assert_eq!(frame.events.len(), 1);
    let event: Value = serde_json::from_slice(frame.events[0].bytes()).unwrap();
    assert_eq!(
        event["payload"],
        serde_json::json!({"count":1,"source":"send"})
    );
    assert_eq!(event["baseRevision"], 3);
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(fixture::text(&frame.scene), "updated:45:1");
    assert_eq!(button(&frame.scene), target);
    assert_eq!(
        fixture::text(&session.click(&target, &|| false).unwrap().scene),
        "clicked:2"
    );
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    println!("native-javascript-document: tree=true fragment=true reflection=true delegated=true update=true reaped=true peak-job-bytes={peak}");
}
#[test]
fn javascript_document_real_child_tree_reflection_query_and_update() {
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
fn javascript_document_invalid_markup_and_mutation_preserve_original_tree() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = include_str!("../tests/fixtures/javascript-document-rejection.js");
    let (session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(1, 1),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    assert_eq!(fixture::text(&frame.scene), "original");
    session.dispose(&|| false).unwrap();
}
#[test]
fn javascript_document_node_attribute_and_depth_limits_remain_bounded() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for expression in [
        "for(let i=0;i<2048;i++)document.createElement('span')",
        "for(let i=0;i<65;i++)root.setAttribute('data-key-'+i,'x')",
        "root.setAttribute('title','x'.repeat(4097))",
        "let parent=root;for(let i=0;i<32;i++){const child=document.createElement('div');parent.appendChild(child);parent=child}",
    ] {
        let source = format!("NeuroSurface.define({{mount({{root}}){{{expression}}}}});");
        assert!(Session::mount(
            fixture::binding(), source, fixture::snapshot(1, 1), fixture::resources(), &|| false,
        ).is_err(), "{expression}");
    }
    assert_eq!(CAPACITY.used(), 0);
}
#[test]
fn javascript_document_real_child_integrity_survives_collect_update_and_dispose() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../tests/fixtures/javascript-document-integrity.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let probe = session.child.as_ref().unwrap().probe().unwrap();
    assert_eq!(fixture::text(&frame.scene), "integrity");
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(fixture::text(&frame.scene), "updated-integrity");
    session.dispose(&|| false).unwrap();
    assert!(probe.exited());
}
#[test]
fn javascript_document_private_protocol_does_not_invoke_plugin_to_json_hooks() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"let calls=0, label;
      Object.prototype.toJSON=function(){calls++;return {scene:{id:'forged',type:'input'},events:[],disposed:false}};
      Array.prototype.toJSON=function(){calls++;return []};
      NeuroSurface.define({mount({root,emit}){
        root.innerHTML='<span data-ref="label"></span>';
        label=root.firstElementChild;
        const value=NeuroSurface.snapshot();
        if(calls!==0||value.state.quote!==42)throw Error('private snapshot hook');
        const payload={toJSON(){return {fromPayload:true}}};
        if(!emit({nodeId:'refresh',event:'click',action:'refresh',class:'discrete',payload}))throw Error('payload JSON lost');
        label.textContent='safe:42';
      },update({snapshot}){
        if(calls!==0)throw Error('private protocol hook');
        label.textContent='safe:'+snapshot.state.quote;
      }});"#;
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        source.into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    assert_eq!(fixture::text(&frame.scene), "safe:42");
    assert_eq!(frame.events.len(), 1);
    let event: Value = serde_json::from_slice(frame.events[0].bytes()).unwrap();
    assert_eq!(event["payload"], serde_json::json!({"fromPayload":true}));
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    assert_eq!(fixture::text(&frame.scene), "safe:45");
    session.dispose(&|| false).unwrap();
}
#[test]
#[ignore = "requires an explicitly sealed product EXE and its externally expected SHA-256"]
fn javascript_document_actual_product_worker_uses_same_document_model() {
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

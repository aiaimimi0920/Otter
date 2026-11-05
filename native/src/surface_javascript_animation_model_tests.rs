//! 同一 QuickJS 的 rAF 顺序/取消/时钟/暂停/容量与污染合同；宿主排程另由 Application 验证。
use super::*;
fn mount(source: &str) -> (Engine, Output) {
    let mut engine = Engine::new().unwrap();
    let frame = engine
        .apply(
            Command::Mount {
                source: source.into(),
                snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
                resources: crate::surface_javascript::fixture::resources(),
            },
            None,
        )
        .unwrap();
    (engine, frame)
}
#[test]
fn javascript_animation_model_snapshot_cancel_microtasks_and_monotonic_timestamp() {
    let (mut engine, frame) = mount(
        r#"NeuroSurface.define({mount({root}){
      let cancel, timestamp, before=performance.now();root.textContent='';
      requestAnimationFrame(function(t){if(this!==globalThis||t<before||t>performance.now())throw Error('clock');
        timestamp=t;root.textContent+='a';requestAnimationFrame(next=>{if(next<t)throw Error('backward');root.textContent+='c'});
        Promise.resolve().then(()=>Promise.resolve()).then(()=>{cancelAnimationFrame(cancel);root.textContent+='m'});});
      cancel=requestAnimationFrame(()=>root.textContent+='cancelled');
      requestAnimationFrame(t=>{if(t!==timestamp||root.textContent!=='am')throw Error('same frame');root.textContent+='z'});
    }});"#,
    );
    assert!(frame.animation_pending);
    let frame = engine.apply(Command::AnimationFrame, None).unwrap();
    assert_eq!(frame.scene["children"][0]["props"]["text"], "amz");
    assert!(frame.animation_pending);
    let frame = engine.apply(Command::AnimationFrame, None).unwrap();
    assert_eq!(frame.scene["children"][0]["props"]["text"], "amzc");
    assert!(!frame.animation_pending);
}
#[test]
fn javascript_animation_model_queue_bound_atomic_rejection_and_suspend_resume_dispose() {
    let (mut engine, frame) = mount(
        r#"NeuroSurface.define({mount({root}){
      let n=0, ids=[]; for(let i=0;i<64;i++)ids.push(requestAnimationFrame(()=>root.textContent=String(++n)));
      for(const value of [null,1,{},undefined]){let rejected=false;try{requestAnimationFrame(value)}catch{rejected=true}if(!rejected)throw Error('callback type');}
      let rejected=false;try{requestAnimationFrame(()=>{})}catch{rejected=true}if(!rejected)throw Error('capacity');
      cancelAnimationFrame(ids[0]);cancelAnimationFrame(undefined);cancelAnimationFrame(99999);
      const next=requestAnimationFrame(()=>root.textContent=String(++n));if(next<=ids[63])throw Error('reused handle');
    }});"#,
    );
    assert!(frame.animation_pending);
    assert!(
        !engine
            .apply(Command::Suspend, None)
            .unwrap()
            .animation_pending
    );
    assert!(
        engine
            .apply(Command::Resume, None)
            .unwrap()
            .animation_pending
    );
    let frame = engine.apply(Command::AnimationFrame, None).unwrap();
    assert_eq!(frame.scene["children"][0]["props"]["text"], "64");
    assert!(!frame.animation_pending);
    assert!(
        !engine
            .apply(Command::Dispose, None)
            .unwrap()
            .animation_pending
    );
}
#[test]
fn javascript_animation_model_private_queue_ignores_replaced_collection_intrinsics() {
    let (mut engine, _) = mount(
        r#"NeuroSurface.define({mount({root}){
      requestAnimationFrame(()=>{root.textContent='first';requestAnimationFrame(()=>root.textContent='next');});
      for(const name of ['get','set','delete','forEach','clear'])Map.prototype[name]=()=>{throw Error('poisoned map')};
      Reflect.apply=()=>{throw Error('poisoned apply')};Array.prototype.push=()=>{throw Error('poisoned push')};
      Object.defineProperty(Object.prototype,'animationPending',{get(){throw Error('poisoned output')},configurable:true});
    }});"#,
    );
    assert_eq!(
        engine.apply(Command::AnimationFrame, None).unwrap().scene["children"][0]["props"]["text"],
        "first"
    );
    let frame = engine.apply(Command::AnimationFrame, None).unwrap();
    assert_eq!(frame.scene["children"][0]["props"]["text"], "next");
    assert!(!frame.animation_pending);
}

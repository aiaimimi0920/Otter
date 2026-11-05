//! 可控单调时钟验证 timer 算法；真实 Engine 另保护逐 callback microtask/原预算。
use super::*;
fn model(script: &str) {
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    let source = format!(
        "(()=>{{const fail=()=>{{throw Error('rejected')}};let time=0;const timers=({})(({})(fail,{}),fail,()=>time);const tick=()=>{{timers.begin();while(timers.step()){{}}}};const check=v=>{{if(!v)throw Error('assertion')}};{} }})()",
        include_str!("surface_javascript_timers.js"), include_str!("surface_javascript_tree.js"), include_str!("surface_javascript_event_target.js"), script
    );
    context.with(|ctx| ctx.eval::<(), _>(source)).unwrap();
}
#[test]
fn timer_model_shared_ids_receiver_arguments_cancel_and_next_task() {
    model(
        r#"let log='';const token={};const a=setTimeout(function(x,y){check(this===globalThis&&x===token&&y===7);log+='a';setTimeout(()=>log+='n',0)},0,token,7);
      const b=setInterval(()=>log+='b',0);clearTimeout(b);const c=setTimeout(()=>log+='c',0);clearInterval(c);
      check(a<b&&b<c&&timers.pending()===0);tick();check(log==='a'&&timers.pending()===0);tick();check(log==='an'&&timers.pending()===null);
      clearTimeout(undefined);clearInterval(0);clearTimeout(-1);"#,
    );
}
#[test]
fn timer_model_long_conversion_once_reentrancy_and_order() {
    model(
        r#"let log='',calls=0;setTimeout(()=>log+='later',5);setTimeout(()=>log+='first',-2);setTimeout(()=>log+='second',2147483648);
      setTimeout(()=>log+='third',{valueOf(){calls++;return '0'}});tick();check(calls===1&&log==='firstsecondthird'&&timers.pending()===5);
      time=4;tick();check(log==='firstsecondthird');time=5;tick();check(log==='firstsecondthirdlater');
      for(const bad of [1n,Symbol()]){let rejected=false;try{setTimeout(()=>{},bad)}catch{rejected=true}check(rejected)}
      for(const value of [NaN,Infinity,-Infinity,undefined,4294967296]){const id=setTimeout(()=>{},value);check(timers.pending()===0);clearInterval(id)}"#,
    );
}
#[test]
fn timer_model_capacity_cancellation_strings_and_arguments_reject_atomically() {
    model(
        r#"let n=0,ids=[];for(let i=0;i<64;i++)ids.push(setTimeout(()=>n++,0));
      let rejected=false;try{setInterval(()=>{},0)}catch{rejected=true}check(rejected);clearInterval(ids[0]);
      for(const bad of ['n++',{},null,undefined]){rejected=false;try{setTimeout(bad,0)}catch{rejected=true}check(rejected)}
      rejected=false;try{setTimeout(()=>{},0,...Array(33))}catch{rejected=true}check(rejected);
      const id=setTimeout(()=>n++,0);check(id>ids[63]);tick();check(n===64&&timers.pending()===null);
      let conversions=0;const timeout={valueOf(){conversions++;for(let i=0;i<64;i++)setTimeout(()=>{},100);return 0}};
      rejected=false;try{setTimeout(()=>{},timeout)}catch{rejected=true}check(rejected&&conversions===1);"#,
    );
}
#[test]
fn timer_model_interval_no_catchup_nesting_clamp_and_cancel_inside_callback() {
    model(
        r#"let n=0;const id=setInterval(()=>{n++;if(n===8)clearTimeout(id)},0);
      for(let i=0;i<6;i++){tick();check(n===i+1)}check(timers.pending()===4);time=1000;tick();check(n===7&&timers.pending()===4);
      tick();check(n===7);time=1004;tick();check(n===8&&timers.pending()===null);
      n=0;const nested=()=>{n++;if(n<8)setTimeout(nested,0)};setTimeout(nested,0);
      for(let i=0;i<6;i++)tick();check(n===6&&timers.pending()===4);"#,
    );
}
#[test]
fn timer_model_suspend_freezes_remaining_wait_and_dispose_releases() {
    model(
        r#"let n=0;setTimeout(()=>n++,20);time=5;timers.suspend();check(timers.pending()===null);time=1000;timers.resume();check(timers.pending()===15);
      time=1014;tick();check(n===0);time=1015;tick();check(n===1);setInterval(()=>n++,0);timers.dispose();check(timers.pending()===null);
      let rejected=false;try{setTimeout(()=>{},0)}catch{rejected=true}check(rejected);"#,
    );
}
#[test]
fn timer_model_intrinsic_poisoning_does_not_reach_private_queue() {
    model(
        r#"let n=0;const id=setTimeout(()=>n++,0);setTimeout(()=>{clearTimeout(id);n+=2},-1);
      for(const key of ['get','set','delete','clear','forEach'])Map.prototype[key]=()=>{throw Error('map')};
      Array.prototype.sort=Array.prototype.push=()=>{throw Error('array')};Math.ceil=()=>{throw Error('ceil')};Reflect.apply=()=>{throw Error('apply')};
      check(timers.pending()===0);tick();check(n===3&&timers.pending()===null);"#,
    );
}
#[test]
fn timer_engine_microtasks_cancel_later_callback_and_do_not_run_raf_early() {
    let mut engine = Engine::new().unwrap();
    let first = engine.apply(Command::Mount { source: r#"NeuroSurface.define({mount({root}){
      let cancelled;root.textContent='';requestAnimationFrame(()=>root.textContent+='r');
      setTimeout(()=>{root.textContent+='a';Promise.resolve().then(()=>{clearInterval(cancelled);root.textContent+='m';setTimeout(()=>root.textContent+='n',0)})},0);
      cancelled=setTimeout(()=>root.textContent+='bad',0);setTimeout(()=>root.textContent+='z',0);
    }});"#.into(), snapshot: serde_json::json!({"scene":{"id":"root","type":"column"}}), resources: serde_json::json!({}) }, None).unwrap();
    assert!(first.timer_delay_ms.is_some() && first.animation_pending);
    let frame = engine.apply(Command::TimerTick, None).unwrap();
    assert_eq!(frame.scene["children"][0]["props"]["text"], "amz");
    assert!(frame.animation_pending && frame.timer_delay_ms.is_some());
    let frame = engine.apply(Command::AnimationFrame, None).unwrap();
    assert_eq!(frame.scene["children"][0]["props"]["text"], "amznr");
    assert!(!frame.animation_pending && frame.timer_delay_ms.is_none());
}

//! 同生产 bootstrap 的事件身份/阶段/生命周期；真实 child/产品与 HWND 另验。
use super::document_model_tests::collect;
#[test]
fn javascript_event_target_model_intrinsics_and_return_value_use_private_state() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-event-integrity.js"
    ));
    assert_eq!(
        value["scene"]["children"][0]["props"]["text"],
        "event-integrity-ready"
    );
}
#[test]
fn javascript_event_target_model_capture_bubble_cancel_and_object_callbacks() {
    let value = collect(include_str!("../tests/fixtures/javascript-event-target.js"));
    assert_eq!(
        value["scene"]["children"][0]["props"]["text"],
        "event-target-ready"
    );
}
#[test]
fn javascript_event_target_model_mutation_fixed_path_once_and_throw_cleanup() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-event-mutation.js"
    ));
    assert_eq!(
        value["scene"]["children"][0]["props"]["text"],
        "event-mutation-ready"
    );
}
#[test]
fn javascript_event_target_model_budget_and_rejection_preserve_registrations() {
    collect(
        r#"NeuroSurface.define({mount(){
      const target=new EventTarget(), reject=operation=>{let failed=false;try{operation()}catch{failed=true}if(!failed)throw Error('not rejected')};
      let count=0;const callback=()=>count++;
      target.addEventListener('sample',callback);target.addEventListener('sample',callback,{once:true});
      for(let i=0;i<15;i++)target.addEventListener('sample',()=>count++);
      reject(()=>target.addEventListener('sample',()=>{}));
      reject(()=>target.addEventListener('sample',callback,{signal:{}}));
      reject(()=>target.addEventListener('x'.repeat(257),callback));
      reject(()=>target.addEventListener());reject(()=>target.removeEventListener('sample',3));
      reject(()=>target.dispatchEvent({type:'sample'}));
      reject(()=>EventTarget.prototype.dispatchEvent.call({},new Event('sample')));
      target.dispatchEvent(new Event('sample'));target.dispatchEvent(new Event('sample'));
      if(count!==32)throw Error('registration changed');
      for(let i=0;i<2046;i++)new EventTarget();reject(()=>new EventTarget());
    }});"#,
    );
}
#[test]
fn javascript_event_target_model_option_conversion_reentrancy_preserves_active_changes() {
    collect(
        r#"NeuroSurface.define({mount(){
      const target=new EventTarget(),marker={},log=[];const first=()=>log.push('first'),second=()=>log.push('second');
      target.addEventListener('value',first);
      const options={get capture(){target.removeEventListener('value',first);target.addEventListener('value',second);throw marker}};
      let error;try{target.addEventListener('value',first,options)}catch(value){error=value}
      target.dispatchEvent(new Event('value'));
      if(error!==marker||log.join(',')!=='second')throw Error('conversion atomicity');
    }});"#,
    );
}

//! Pointer 字段/继承/brand 与原 Event 传播、取消、finally 的同生产模型证明。
use super::document_model_tests::collect;
#[test]
fn javascript_pointer_model_synthetic_fields_brand_cancel_path_and_cleanup() {
    collect(
        r#"NeuroSurface.define({mount({root}){
      const c=document.createElement('div');root.append(c);
      const reject=f=>{let failed=false;try{f()}catch{failed=true}if(!failed)throw Error('not rejected')};
      reject(()=>new MouseEvent());reject(()=>new PointerEvent());
      if(new PointerEvent(undefined).type!=='undefined')throw Error('explicit undefined');
      const e=new PointerEvent('pointerdown',{bubbles:true,cancelable:true,composed:true,clientX:-3.25,clientY:4,
        button:0,buttons:1,pointerId:7,pointerType:'mouse',isPrimary:true,ctrlKey:true,shiftKey:true,altKey:true,metaKey:true});
      if(!(e instanceof MouseEvent)||!(e instanceof Event)||e.isTrusted||e.clientX!==-3.25||e.clientY!==4||e.pointerId!==7||!e.isPrimary)throw Error('fields');
      for(const key of ['Control','Shift','Alt','Meta'])if(!e.getModifierState(key))throw Error('modifier');
      if(e.getModifierState('CapsLock'))throw Error('unknown modifier');
      let log='';root.addEventListener('pointerdown',v=>{log+='capture>';if(v.eventPhase!==1)throw Error('capture')},true);
      c.addEventListener('pointerdown',v=>{log+='target>';v.preventDefault();if(v.target!==c||v.currentTarget!==c)throw Error('target')});
      root.addEventListener('pointerdown',()=>log+='bubble');
      if(c.dispatchEvent(e)!==false||log!=='capture>target>bubble'||!e.defaultPrevented||e.isTrusted||e.currentTarget!==null||e.eventPhase!==0||e.composedPath().length)throw Error('dispatch');
      const client=Object.getOwnPropertyDescriptor(MouseEvent.prototype,'clientX').get;
      const identity=Object.getOwnPropertyDescriptor(PointerEvent.prototype,'pointerId').get;
      reject(()=>client.call(new Event('x')));reject(()=>identity.call(new MouseEvent('x')));reject(()=>identity.call({}));
      for(const value of [NaN,Infinity,-Infinity,131073])reject(()=>new PointerEvent('x',{clientX:value}));
      reject(()=>new PointerEvent('x',{button:1.5}));reject(()=>new PointerEvent('x',{buttons:32}));reject(()=>new PointerEvent('x',{pointerId:-1}));
      const marker={};let error;try{new PointerEvent('x',{get clientX(){throw marker}})}catch(e){error=e}if(error!==marker)throw Error('conversion');
      const thrown=new PointerEvent('throw',{bubbles:true});c.addEventListener('throw',()=>{throw marker});
      try{c.dispatchEvent(thrown)}catch(e){if(e!==marker)throw e}
      if(thrown.currentTarget!==null||thrown.eventPhase!==0||thrown.composedPath().length)throw Error('finally');
    }});"#,
    );
}

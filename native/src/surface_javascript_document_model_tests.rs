//! Locate semantic failures directly in the same real QuickJS model; independent process gates still verify child/product budgets.
use super::*;
pub(super) fn collect(source: &str) -> serde_json::Value {
    let runtime = Runtime::new().unwrap();
    runtime.set_memory_limit(32 * 1024 * 1024);
    runtime.set_max_stack_size(256 * 1024);
    let context = Context::full(&runtime).unwrap();
    context.with(|ctx| {
        let result = (|| {
            let bridge: Object = ctx.eval(BOOTSTRAP)?;
            Module::evaluate(ctx.clone(), "surface:model-test", source)?.finish::<()>()?;
            let function: Function = bridge.get("apply")?;
            let command = Command::Mount {
                source: String::new(),
                snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
                resources: crate::surface_javascript::fixture::resources(),
            };
            let command = String::from_utf8(wire::encode(&command).unwrap()).unwrap();
            function.call::<_, Promise>((command,))?.finish::<()>()?;
            let function: Function = bridge.get("collect")?;
            function.call::<_, String>(())
        })();
        match result {
            Ok(value) => serde_json::from_str(&value).unwrap(),
            Err(error) => {
                let exception = ctx.catch();
                let stack = exception
                    .as_object()
                    .and_then(|object| object.get::<_, String>("stack").ok());
                panic!("QuickJS document model: {error:?}: {exception:?}: {stack:?}");
            }
        }
    })
}
#[test]
fn javascript_document_model_rejections_preserve_original_tree() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-document-rejection.js"
    ));
    assert_eq!(value["scene"]["children"][0]["props"]["text"], "original");
}
#[test]
fn javascript_document_model_markup_rejection_preserves_node_and_event_capacity() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<span>original</span>';
      const old=root.firstChild, long='x'.repeat(65537);
      const reject=source=>{
        try{root.innerHTML=source}catch(error){return}
        throw Error('unexpected admission');
      };
      for(let i=0;i<2048;i++){
        reject(long); reject('x\0'); reject('<span></div>');
      }
      if(root.firstChild!==old)throw Error('lost identity');
      // Root plus the first Fragment, span and Text used four identities.
      for(let i=0;i<2040;i++)document.createElement('span');
      // This reaches the node boundary mid-construction before the final insert.
      reject('<span>one</span><span>two</span>');
      root.innerHTML='<span>next</span>';
      const probe=document.createElement('span');
      probe.addEventListener('click',()=>{});
      reject('<span></span>');
    }});"#,
    );
    assert_eq!(value["scene"]["children"][0]["props"]["text"], "next");
}
#[test]
fn javascript_document_model_markup_converts_before_private_transaction() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      const append=Node.prototype.appendChild, replace=Node.prototype.replaceChildren;
      const attr=Element.prototype.setAttribute, exec=RegExp.prototype.exec;
      const starts=String.prototype.startsWith, superType=Object.getPrototypeOf(HTMLElement);
      const ErrorType=Error, trap=()=>{throw new ErrorType('public hook called')};
      let retained;
      Node.prototype.appendChild=Node.prototype.replaceChildren=Element.prototype.setAttribute=trap;
      RegExp.prototype.exec=String.prototype.startsWith=trap;
      Object.setPrototypeOf(HTMLElement, class {constructor(){trap()}});
      globalThis.Error=trap;
      try{
        try{root.innerHTML={toString(){
          root.innerHTML='<span>conversion</span>';retained=root.firstChild;
          return '<span>&#xD800;</span>';
        }};throw new ErrorType('unexpected admission')}catch(error){
          if(error.message!=='Unsupported or invalid native DOM operation')throw error;
        }
        if(root.firstChild!==retained)throw new ErrorType('conversion lost');
        root.innerHTML='<span title="&quot;">&amp;&#65;&#x1F600;&UNKNOWN;</span>';
        if(root.firstChild.textContent!=='&A😀&UNKNOWN;'||root.firstChild===retained)
          throw new ErrorType('entity or identity mismatch');
      }finally{
        Node.prototype.appendChild=append;Node.prototype.replaceChildren=replace;Element.prototype.setAttribute=attr;
        RegExp.prototype.exec=exec;String.prototype.startsWith=starts;Object.setPrototypeOf(HTMLElement,superType);
        globalThis.Error=ErrorType;
      }
    }});"#,
    );
    assert_eq!(
        value["scene"]["children"][0]["props"]["text"],
        "&A😀&UNKNOWN;"
    );
}
#[test]
fn javascript_document_model_generic_fragment_fixture() {
    let value = collect(include_str!("../tests/fixtures/javascript-document.js"));
    assert_eq!(value["scene"]["children"][0]["props"]["text"], "ready");
}
#[test]
fn javascript_document_model_valid_depth_boundary_uses_no_recursive_js_walks() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div>'.repeat(30)+'x'+'</div>'.repeat(30);
      if(root.textContent!=='x'||root.querySelectorAll('div').length!==30||!root.innerHTML.endsWith('</div>'))throw Error('valid boundary');
    }});"#,
    );
    assert_eq!(value["scene"]["children"].as_array().unwrap().len(), 1);
}
#[test]
fn javascript_document_model_integrity_survives_through_collect() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-document-integrity.js"
    ));
    assert_eq!(value["scene"]["children"][0]["props"]["text"], "integrity");
}
#[test]
fn javascript_document_model_repeated_fragment_and_individual_nodes_preserve_order() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      const fragment=document.createDocumentFragment(),a=document.createElement('span'),b=document.createElement('span');
      a.textContent='a';b.textContent='b';fragment.append(a,b);
      root.append(fragment,a,fragment);
      if(root.textContent!=='ba'||root.firstChild!==b||fragment.childNodes.length!==0)throw Error('repeated fragment append order');
      fragment.append(a,b);root.replaceChildren(fragment,a,fragment);
      if(root.textContent!=='ba'||root.lastChild!==a)throw Error('repeated fragment replace order');
      fragment.append(a,b);root.replaceChildren(b,fragment);
      if(root.textContent!=='ba'||root.lastChild!==a)throw Error('individual before fragment');
    }});"#,
    );
    assert_eq!(value["scene"]["children"][0]["props"]["text"], "b");
    assert_eq!(value["scene"]["children"][1]["props"]["text"], "a");
}
#[test]
fn javascript_document_model_mutable_intrinsics_cannot_observe_private_tree() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      const originals=[WeakMap.prototype.get,WeakMap.prototype.set,Map.prototype.get,Map.prototype.set,
        Array.prototype.map,Array.prototype.filter,Array.prototype[Symbol.iterator]];
      let leaked=false;
      const capture=value=>{if(value&&value.attrs&&value.children&&value.key)leaked=true;return value};
      WeakMap.prototype.get=function(key){return capture(originals[0].call(this,key))};
      WeakMap.prototype.set=function(key,value){capture(value);return originals[1].call(this,key,value)};
      Map.prototype.get=function(key){return capture(originals[2].call(this,key))};
      Map.prototype.set=function(key,value){capture(value);return originals[3].call(this,key,value)};
      Array.prototype.map=Array.prototype.filter=function(){throw Error('private array method dispatched')};
      Array.prototype[Symbol.iterator]=function(){throw Error('private iterator dispatched')};
      try{
        root.textContent='safe';
        const next=document.createElement('button');next.textContent='next';root.appendChild(next);
        if(root.textContent!=='safenext'||root.children.length!==1||leaked)throw Error('private tree leaked');
      }finally{
        WeakMap.prototype.get=originals[0];WeakMap.prototype.set=originals[1];Map.prototype.get=originals[2];Map.prototype.set=originals[3];
        Array.prototype.map=originals[4];Array.prototype.filter=originals[5];Array.prototype[Symbol.iterator]=originals[6];
      }
    }});"#,
    );
    assert_eq!(value["scene"]["children"][0]["props"]["text"], "safe");
    assert_eq!(value["scene"]["children"][1]["props"]["label"], "next");
}
#[test]
fn javascript_document_model_first_selector_matches_static_tree_order() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<section><span id="same" class="hit" data-x="v">first</span></section><span id="same" class="hit" data-x="v">last</span>';
      const fragment=document.createDocumentFragment();
      fragment.append(document.createTextNode('text'),document.createElement('span'),document.createElement('span'));
      for(const owner of [document,root,fragment]){
        for(const selector of ['*','span','.hit','#same','[data-x]','[data-x="v"]','[missing]']){
          const all=owner.querySelectorAll(selector);
          if(owner.querySelector(selector)!==(all[0]??null)||!Object.isFrozen(all)||all.item(999)!==null)throw Error('first/static mismatch');
        }
      }
      if(document.querySelector('*')!==root||root.querySelector('*')!==root.firstChild)throw Error('root inclusion');
      if(root.querySelector('.hit').textContent!=='first')throw Error('preorder');
      const snapshot=root.querySelectorAll('.hit');root.replaceChildren();
      if(snapshot.length!==2||snapshot.item(0).textContent!=='first'||root.querySelector('.hit')!==null)throw Error('static collection');
      root.textContent='selectors';
    }});"#,
    );
    assert_eq!(value["scene"]["children"][0]["props"]["text"], "selectors");
}
#[test]
fn javascript_document_model_first_selector_stops_after_match() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<span data-hit="yes">first</span><span data-hit="yes">last</span>';
      const original=Element.prototype.hasAttribute;let visits=0;
      Element.prototype.hasAttribute=function(name){visits++;return original.call(this,name)};
      try{
        if(root.querySelector('[data-hit]')!==root.firstChild||visits!==1)throw Error('element scanned later match');
        visits=0;
        if(document.querySelector('[data-hit]')!==root.firstChild||visits!==2)throw Error('document scanned later match');
        visits=0;root.setAttribute('data-hit','yes');
        if(document.querySelector('[data-hit]')!==root||visits!==1)throw Error('root did not stop');
        visits=0;
        if(root.querySelectorAll('[data-hit]').length!==2||visits!==2)throw Error('all stopped early');
      }finally{Element.prototype.hasAttribute=original}
      root.textContent='short-circuit';
    }});"#,
    );
    assert_eq!(
        value["scene"]["children"][0]["props"]["text"],
        "short-circuit"
    );
}
#[test]
fn javascript_document_model_selector_validation_and_conversion_keep_timing() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      const fragment=document.createDocumentFragment();
      for(const owner of [document,root,fragment])for(const method of ['querySelector','querySelectorAll']){
        let rejected=false;try{owner[method]('div > span')}catch{rejected=true}
        if(!rejected)throw Error('empty tree accepted invalid selector');
      }
      const target=document.createElement('span');target.id='target';
      if(root.querySelector({toString(){root.append(target);return '#target'}})!==target)throw Error('conversion before walk');
      if(document.getElementById({toString(){target.remove();return 'target'}})!==target)throw Error('id conversion after collection');
      root.textContent='conversion';
    }});"#,
    );
    assert_eq!(value["scene"]["children"][0]["props"]["text"], "conversion");
}
#[test]
fn javascript_document_model_query_bounds_and_static_collection_integrity() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-document-query.js"
    ));
    assert_eq!(value["scene"]["children"][0]["props"]["text"], "queries");
}
#[test]
fn javascript_document_model_live_children_mutation_and_reentrant_item() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-document-collections.js"
    ));
    assert_eq!(
        value["scene"]["children"][0]["props"]["text"],
        "collections"
    );
}

//! 直接在同一真实 QuickJS 模型上定位语义异常；child/产品预算仍由独立进程门验证。
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

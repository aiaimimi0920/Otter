//! 私有JSON bulk descriptor扫描保留原字符串/可枚举数据属性合同，不调用插件getter/toJSON。
use super::*;

fn evaluate(script: &str) {
    let runtime = Runtime::new().unwrap();
    runtime.set_memory_limit(32 * 1024 * 1024);
    runtime.set_max_stack_size(256 * 1024);
    let context = Context::full(&runtime).unwrap();
    context.with(|ctx| {
        let factory = include_str!("surface_javascript_json.js");
        // 仅在模型副本暴露原迭代copy，单独检查其深度门；native stringify还受原256KiB限制。
        let factory = factory.replacen("return { encode", "return { copy, encode", 1);
        let source = format!("const codec=({factory});{script}");
        if let Err(error) = ctx.eval::<(), _>(source) {
            panic!("Private JSON model: {error:?}: {:?}", ctx.catch());
        }
    });
}

#[test]
fn javascript_json_bulk_descriptors_preserve_data_keys_array_holes_and_depth() {
    evaluate(
        r#"
      const check=(v,expected)=>{if(codec.encode(v)!==expected)throw Error('private JSON mismatch')};
      check({a:1,b:'中🙂',c:true,d:null},'{"a":1,"b":"中🙂","c":true,"d":null}');
      const a=[];a[2]='x';a.extra='ignored';Object.defineProperty(a,'length',{writable:false});
      check(a,'[null,null,"x"]');check(new Array(3),'[]');
      const v=Object.create({inherited:'ignored'});v.second=2;v.first=1;
      Object.defineProperty(v,'hidden',{get(){throw Error('hidden getter')},enumerable:false});
      v[Symbol('ignored')]=()=>{};check(v,'{"second":2,"first":1}');
      let value=0;for(let i=0;i<128;i++)value={value};codec.copy(value);
      let rejected=false;try{codec.copy({value})}catch{rejected=true}if(!rejected)throw Error('depth');
      for(const bad of [undefined,()=>{},Symbol('bad'),1n,NaN,Infinity]){
        rejected=false;try{codec.encode({bad})}catch{rejected=true}if(!rejected)throw Error('invalid value');
      }
      check(codec.clone({nested:[1,'ok']}),'{"nested":[1,"ok"]}');
      check(codec.decode('{"__proto__":{"safe":1},"constructor":2}'),'{"__proto__":{"safe":1},"constructor":2}');
    "#,
    );
}

#[test]
fn javascript_json_bulk_descriptors_use_captured_intrinsics_and_reject_accessors() {
    evaluate(
        r#"
      const reject=value=>{let caught=false;try{codec.encode(value)}catch{caught=true}if(!caught)throw Error('accessor accepted')};
      const value={};Object.defineProperty(value,'x',{enumerable:true,get(){throw Error('getter invoked')}});
      reject(value);
      Object.defineProperty(Object.prototype,'value',{get(){throw Error('inherited descriptor value')},configurable:true});
      reject(value);
      Object.prototype.toJSON=()=>{throw Error('inherited toJSON')};
      const deny=()=>{throw Error('public intrinsic')};
      Object.getOwnPropertyDescriptors=Object.getOwnPropertyDescriptor=Object.keys=Object.hasOwn=deny;
      if(codec.encode({safe:[1,2]})!=='{"safe":[1,2]}')throw Error('captured descriptors');
    "#,
    );
}

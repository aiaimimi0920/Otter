//! 同生产工厂/真实 QuickJS 验证投影缓存；fake boxes 不替代原生布局、预算或 HWND 门。
use super::*;

fn model(script: &str) {
    let runtime = Runtime::new().unwrap();
    runtime.set_memory_limit(32 * 1024 * 1024);
    runtime.set_max_stack_size(256 * 1024);
    let context = Context::full(&runtime).unwrap();
    let factory = include_str!("surface_javascript_geometry.js");
    // 在cache miss处计数，不增加函数包装栈；故障注入只存在于测试求值的副本。
    let counted = factory.replace(
        "const source = json.encode(project(dom.root));",
        "projections++; const source = json.encode(project(dom.root)); if(projectionMutation){projectionMutation=false;dom.root.title='during-projection'}",
    );
    assert_ne!(factory, counted);
    let bootstrap = BOOTSTRAP.replacen(factory, &counted, 1);
    let json = include_str!("surface_javascript_json.js");
    let mut steps = script.split("await operation(");
    let setup = steps.next().unwrap();
    let source = format!(
        r#"
          let reads=0,lastSource,root,rejectRead=0;
          const parse=JSON.parse,stringify=({json}).encode;
          const check=value=>{{if(!value)throw Error('projection assertion')}};
          const bridge=__projectionBridge;delete globalThis.__projectionBridge;
          bridge.bindGeometry(source=>{{reads++;lastSource=source;return reads===rejectRead?'':'[0,0,20,20,20,20,20,20,0,0,20,20]'}});
          bridge.bindAnimationClock(()=>0);bridge.bindCanvas(()=>true);
          const current=()=>{{root.clientWidth;check(lastSource===stringify(parse(bridge.collect()).scene));return parse(lastSource)}};
          const changed=edit=>{{const before=projections;edit();root.clientWidth;check(projections===before+1);check(lastSource===stringify(parse(bridge.collect()).scene))}};
          const rejected=edit=>{{const before=projections,source=lastSource;let caught=false;try{{edit()}}catch{{caught=true}}check(caught);current();check(projections===before&&lastSource===source)}};
          {setup}
        "#
    );
    context.with(|ctx| {
        // 像Engine一样从Rust交付operation；模型不额外嵌套eval/async调度栈或放宽256KiB。
        let result = (|| {
            ctx.eval::<(), _>("let projections=0,projectionMutation=false;")?;
            let bridge: Object = ctx.eval(bootstrap)?;
            ctx.globals().set("__projectionBridge", bridge.clone())?;
            ctx.eval::<(), _>(source)?;
            for step in steps {
                let (kind, checks) = step.split_once(");").unwrap();
                let kind = kind.trim_matches('\'');
                bridge.get::<_, Function>("beginGeometry")?.call::<_, ()>(("1",))?;
                bridge.get::<_, Function>("beginCanvas")?.call::<_, ()>(())?;
                let command = serde_json::json!({"type":kind,"snapshot":{"scene":{"id":"root","type":"column"}},"resources":{}}).to_string();
                bridge.get::<_, Function>("apply")?.call::<_, Promise>((command,))?.finish::<()>()?;
                ctx.eval::<(), _>(checks)?;
            }
            Ok::<_, rquickjs::Error>(())
        })();
        if let Err(error) = result {
            let exception = ctx.catch();
            let stack = exception
                .as_object()
                .and_then(|object| object.get::<_, String>("stack").ok());
            panic!("QuickJS projection model: {error:?}: {exception:?}: {stack:?}");
        }
    });
}

#[test]
fn javascript_geometry_projection_reuses_source_not_native_reads_and_resets_each_operation() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;root.innerHTML='<button>measure</button>';
          const box=root.firstChild;for(let i=0;i<2;i++){box.getBoundingClientRect();box.clientWidth;box.clientHeight;box.offsetWidth}
          check(projections===1&&reads===8);current();check(projections===1&&reads===9)},update(){current()}});
        await operation('mount');const source=lastSource;await operation('update');
        check(projections===2&&reads===10&&source===lastSource);"#,
    );
}

#[test]
fn javascript_geometry_projection_tree_text_fragment_markup_and_removal_invalidate() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;current();
          changed(()=>root.appendChild(document.createElement('div')));
          changed(()=>root.firstChild.textContent='a');
          changed(()=>root.firstChild.firstChild.data='b');
          changed(()=>root.firstChild.firstChild.textContent='c');
          changed(()=>{const f=document.createDocumentFragment();f.append(document.createElement('button'));root.append(f)});
          changed(()=>root.appendChild(root.firstChild));
          changed(()=>root.removeChild(root.lastChild));
          changed(()=>root.firstChild.remove());
          changed(()=>root.innerHTML='<div><p>markup</p></div>');
          changed(()=>root.replaceChildren(document.createElement('button')));
        }});await operation('mount');"#,
    );
}

#[test]
fn javascript_geometry_projection_attribute_reflection_and_selector_cascade_invalidate() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;root.innerHTML='<button>button</button>';
          const sheet=new CSSStyleSheet();sheet.replaceSync('button:disabled{opacity:0.5} .a{color:red} [data-x]{background:blue} #id{color:green}');
          document.adoptedStyleSheets=[sheet];const b=root.firstChild;current();
          changed(()=>b.setAttribute('class','a'));changed(()=>b.removeAttribute('class'));
          changed(()=>b.className='a');changed(()=>b.classList.remove('a'));changed(()=>b.classList.add('a'));
          changed(()=>b.classList.toggle('a'));changed(()=>b.classList.toggle('a',true));
          changed(()=>b.id='id');changed(()=>b.dataset.x='yes');changed(()=>delete b.dataset.x);
          changed(()=>b.disabled=true);changed(()=>b.disabled=false);
        }});await operation('mount');"#,
    );
}

#[test]
fn javascript_geometry_projection_inline_shorthand_removal_and_constructed_sheets_invalidate() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;root.style.boxSizing='border-box';current();
          changed(()=>root.style.width='20px');changed(()=>root.style.setProperty('padding','1px'));
          changed(()=>root.style.removeProperty('padding'));changed(()=>root.style.width='');
          changed(()=>root.style.display='grid');changed(()=>root.style.setProperty('gap','1px 2px'));
          changed(()=>root.style.removeProperty('gap'));changed(()=>root.style.display='');
          changed(()=>root.style.cssText='color:red');changed(()=>root.style='color:blue');
          const sheet=new CSSStyleSheet();changed(()=>sheet.replaceSync(':root{color:green}'));
          changed(()=>document.adoptedStyleSheets=[sheet]);changed(()=>sheet.replaceSync(':root{color:red}'));
          changed(()=>document.adoptedStyleSheets=[]);
        }});await operation('mount');"#,
    );
}

#[test]
fn javascript_geometry_projection_canvas_dimensions_invalidate_but_drawing_does_not() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;root.appendChild(document.createElement('canvas'));
          const c=root.firstChild;current();changed(()=>c.width=40);changed(()=>c.height=30);
          changed(()=>c.setAttribute('width','50'));changed(()=>c.setAttribute('height','20'));
          changed(()=>c.removeAttribute('width'));changed(()=>c.removeAttribute('height'));
          const before=projections;c.getContext('2d').fillRect(0,0,1,1);current();check(projections===before);
        }});await operation('mount');"#,
    );
}

#[test]
fn javascript_geometry_projection_rejected_mutations_do_not_install_or_invalidate_source() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;root.innerHTML='<div><canvas></canvas></div>';
          root.style.boxSizing='border-box';const sheet=new CSSStyleSheet();sheet.replaceSync(':root{color:red}');document.adoptedStyleSheets=[sheet];current();
          rejected(()=>root.appendChild(root));rejected(()=>root.setAttribute('onclick','code'));
          rejected(()=>root.setAttribute('title','x'.repeat(4097)));
          rejected(()=>root.style.setProperty('padding','1px invalid'));rejected(()=>root.style.cssText='color:blue;unknown:1');
          rejected(()=>root.style='color:blue;unknown:1');rejected(()=>sheet.replaceSync(':root{color:blue;unknown:1}'));
          rejected(()=>document.adoptedStyleSheets=[{}]);rejected(()=>root.firstChild.firstChild.width=4097);
        }});await operation('mount');"#,
    );
}

#[test]
fn javascript_geometry_projection_failed_projection_is_not_cached_or_sent_to_native() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;root.textContent='safe';current();
          const count=projections,native=reads;root.style.width='10px';
          for(let i=0;i<2;i++){let caught=false;try{root.clientWidth}catch{caught=true}check(caught)}
          check(projections===count+2&&reads===native);root.style.width='';current();check(projections===count+3);
        }});await operation('mount');"#,
    );
}

#[test]
fn javascript_geometry_projection_conversion_reentrancy_never_commits_preconversion_source() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;current();
          let before=projections,calls=0;
          root.style.cssText={toString(){calls++;root.clientWidth;root.title='nested';root.clientWidth;return 'color:red'}};
          current();check(projections===before+2&&calls===1);before=projections;
          root.setAttribute('title',{toString(){root.clientWidth;root.style.color='blue';root.clientWidth;return 'outer'}});
          current();check(projections===before+2);before=projections;
          root.textContent={toString(){root.clientWidth;root.id='changed';root.clientWidth;return 'text'}};
          current();check(projections===before+2);before=projections;
          const sheet=new CSSStyleSheet();sheet.replaceSync({toString(){root.clientWidth;return ':root{color:green}'}});
          current();check(projections===before+1);before=projections;
          document.adoptedStyleSheets=Object.defineProperty([sheet],0,{get(){root.clientWidth;return sheet}});
          current();check(projections===before+1);
        }});await operation('mount');"#,
    );
}

#[test]
fn javascript_geometry_projection_mid_projection_mutation_rejects_without_cache_commit() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;current();const count=projections,native=reads;
          root.title='before';projectionMutation=true;let caught=false;try{root.clientWidth}catch{caught=true}
          check(caught&&projections===count+1&&reads===native);current();check(projections===count+2&&reads===native+1);
        }});await operation('mount');"#,
    );
}

#[test]
fn javascript_geometry_projection_cache_hit_still_rejects_empty_native_result() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;current();const count=projections,native=reads;
          rejectRead=reads+1;let caught=false;try{root.clientWidth}catch{caught=true}
          check(caught&&projections===count&&reads===native+1);rejectRead=0;current();check(projections===count&&reads===native+2);
        }});await operation('mount');"#,
    );
}

#[test]
fn javascript_geometry_projection_private_token_survives_intrinsic_pollution_and_disposal() {
    model(
        r#"NeuroSurface.define({mount(value){root=value.root;root.textContent='safe';current();
          check(root.revision===undefined&&root.changed===undefined&&document.revision===undefined);
          Object.create=()=>{throw Error('create')};Object.prototype.toJSON=()=>{throw Error('toJSON')};
          Map.prototype.set=Map.prototype.get=WeakMap.prototype.set=WeakMap.prototype.get=()=>{throw Error('map')};
          changed(()=>root.title='new');changed(()=>root.style.color='red');
        }});await operation('mount');await operation('dispose');
        let caught=false;try{root.clientWidth}catch{caught=true}check(caught);"#,
    );
}

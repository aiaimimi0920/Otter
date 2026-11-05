//! 真实 QuickJS + 原生测量的 observer 顺序、内容盒、暂停、容量和反例；不是 JS 尺寸 mock。
use super::*;
use crate::surface_viewport::Viewport;
fn view() -> Option<Viewport> {
    Some(Viewport::new([800, 480], 192).unwrap())
}
fn mount(source: &str) -> io::Result<(Engine, Output)> {
    let mut engine = Engine::new()?;
    let output = engine.apply(
        Command::Mount {
            source: source.into(),
            snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
            resources: crate::surface_javascript::fixture::resources(),
        },
        view(),
    )?;
    Ok((engine, output))
}
fn report(output: &Output) -> serde_json::Value {
    serde_json::from_str(
        output.scene["children"][1]["props"]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn javascript_resize_model_real_boxes_initial_then_raf_and_idle() {
    let (mut engine, output) =
        mount(include_str!("../tests/fixtures/javascript-resize.js")).unwrap();
    let first = report(&output);
    assert_eq!(first["count"], 1);
    assert_eq!(first["frames"], 0);
    assert_eq!(
        first["entry"],
        serde_json::json!({"width":188,"height":68,"left":4,"top":4,"border":200,"content":188,"device":376})
    );
    assert!(output.resize_observed && output.animation_pending && output.layout_read);
    let output = engine.apply(Command::AnimationFrame, view()).unwrap();
    assert_eq!(report(&output)["bounds"], 200);
    assert_eq!(report(&output)["frames"], 1);
    assert!(!output.animation_pending && output.resize_observed);
    let output = engine.apply(Command::AnimationFrame, view()).unwrap();
    assert_eq!(report(&output)["count"], 1);
    assert_eq!(report(&output)["frames"], 1);
}
#[test]
fn javascript_resize_model_microtask_disconnect_and_registration_order() {
    let source = r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div></div><p></p>';const box=root.firstElementChild,label=root.lastElementChild;
      box.style.cssText='box-sizing:border-box;width:100px;height:40px';let result='';
      const first=new ResizeObserver(function(entries,owner){if(this!==first||owner!==first)throw Error('this');
        result+='a';Promise.resolve().then(()=>{result+='m';second.disconnect();label.textContent=result});});
      const second=new ResizeObserver(()=>{throw Error('cancelled callback')});
      first.observe(box);second.observe(box);
    }});"#;
    let (_, output) = mount(source).unwrap();
    assert_eq!(output.scene["children"][1]["props"]["text"], "am");
}
#[test]
fn javascript_resize_model_suspend_resize_resume_and_dispose_drop_observations() {
    let (mut engine, output) =
        mount(include_str!("../tests/fixtures/javascript-resize.js")).unwrap();
    assert!(output.resize_observed);
    let output = engine.apply(Command::Suspend, view()).unwrap();
    assert!(!output.resize_observed && !output.animation_pending);
    let wider = Some(Viewport::new([1000, 480], 192).unwrap());
    let output = engine
        .apply(
            Command::Update {
                snapshot: crate::surface_javascript::fixture::snapshot(4, 42),
                resources: crate::surface_javascript::fixture::resources(),
            },
            wider,
        )
        .unwrap();
    assert_eq!(report(&output)["count"], 1);
    let output = engine.apply(Command::Resume, wider).unwrap();
    assert_eq!(report(&output)["count"], 2);
    assert_eq!(report(&output)["entry"]["border"], 250);
    let output = engine.apply(Command::Dispose, wider).unwrap();
    assert!(!output.resize_observed && !output.animation_pending && output.disposed);
}
#[test]
fn javascript_resize_model_bad_inputs_bounds_and_private_intrinsics() {
    let (_, output)=mount(r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div></div><p></p>';const target=root.firstElementChild,label=root.lastElementChild;
      target.style.cssText='box-sizing:border-box;width:100px;height:40px';
      const ro=new ResizeObserver(entries=>{label.textContent=String(entries[0].contentRect.width)});
      const reject=f=>{let caught=false;try{f()}catch{caught=true}if(!caught)throw Error('accepted')};
      reject(()=>new ResizeObserver(null));reject(()=>ro.observe({}));reject(()=>ro.observe(document.createTextNode('x')));
      reject(()=>ro.observe(document.createDocumentFragment()));reject(()=>ro.observe(target,{box:'unknown'}));
      reject(()=>ro.observe(target,{box:null}));
      ro.observe(target);for(let i=0;i<63;i++)ro.observe(document.createElement('div'));
      reject(()=>ro.observe(document.createElement('div')));ro.unobserve(target);ro.observe(target);
      for(let i=1;i<32;i++)new ResizeObserver(()=>{});reject(()=>new ResizeObserver(()=>{}));
      ro.disconnect();ro.observe(target);
      for(const name of ['get','set','delete','forEach','clear','entries'])Map.prototype[name]=()=>{throw Error('map')};
      Object.getPrototypeOf(new Map()[Symbol.iterator]()).next=()=>{throw Error('iterator')};
      Object.freeze=Reflect.apply=()=>{throw Error('intrinsic')};
      Object.defineProperty(Object.prototype,'resizeObserved',{get(){throw Error('output')},configurable:true});
    }});"#).unwrap();
    assert_eq!(output.scene["children"][1]["props"]["text"], "100");
}
#[test]
fn javascript_resize_model_loop_and_callback_failure_fail_closed() {
    for callback in [
        "throw Error('callback')",
        "target.style.width='120px'",
        "while(true){}",
    ] {
        let source=format!("NeuroSurface.define({{mount({{root}}){{root.innerHTML='<div></div>';const target=root.firstElementChild;target.style.cssText='box-sizing:border-box;width:100px;height:40px';new ResizeObserver(()=>{{{callback}}}).observe(target)}}}});");
        assert!(mount(&source).is_err(), "{callback}");
    }
}
#[test]
fn javascript_resize_model_selected_box_changes_not_viewport_or_position_alone() {
    let (mut engine, output) = mount(r#"let box,label,counts=[0,0,0];NeuroSurface.define({mount({root}){
      root.innerHTML='<div></div><p></p>';box=root.firstElementChild;label=root.lastElementChild;
      box.style.cssText='position:relative;box-sizing:border-box;width:100px;height:40px;padding:4px;border-width:2px;border-style:solid';
      label.style.position='relative';
      ['content-box','border-box','device-pixel-content-box'].forEach((mode,index)=>new ResizeObserver(entries=>{
        if(!Object.isFrozen(entries[0])||!Object.isFrozen(entries[0].contentBoxSize))throw Error('mutable');
        counts[index]++;label.textContent=JSON.stringify(counts);
      }).observe(box,{box:mode}));},update({snapshot}){
        if(snapshot.state.quote===1)box.style.borderWidth='4px';
        else box.style.left='10px';
      }});"#).unwrap();
    assert_eq!(report(&output), serde_json::json!([1, 1, 1]));
    let mut update = |revision, quote, view| {
        engine
            .apply(
                Command::Update {
                    snapshot: crate::surface_javascript::fixture::snapshot(revision, quote),
                    resources: crate::surface_javascript::fixture::resources(),
                },
                view,
            )
            .unwrap()
    };
    let output = update(4, 1, view());
    assert_eq!(report(&output), serde_json::json!([2, 1, 2]));
    let output = update(5, 2, view());
    assert_eq!(report(&output), serde_json::json!([2, 1, 2]));
    let output = update(6, 2, Some(Viewport::new([1000, 600], 192).unwrap()));
    assert_eq!(report(&output), serde_json::json!([2, 1, 2]));
    let output = update(7, 2, Some(Viewport::new([1000, 600], 96).unwrap()));
    assert_eq!(report(&output), serde_json::json!([2, 1, 3]));
}
#[test]
fn javascript_resize_model_deeper_delivery_uses_current_layout_and_microtasks() {
    let (_,output)=mount(r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div><div></div></div><p></p>';
      const parent=root.firstElementChild,child=parent.firstElementChild,label=root.lastElementChild;
      parent.style.cssText='box-sizing:border-box;width:150px;height:80px';
      child.style.cssText='box-sizing:border-box;width:50px;height:30px';
      new ResizeObserver(()=>{Promise.resolve().then(()=>child.style.width='75px')}).observe(parent);
      new ResizeObserver(entries=>{label.textContent=String(entries[0].contentRect.width)}).observe(child);
    }});"#).unwrap();
    assert_eq!(output.scene["children"][1]["props"]["text"], "75");
}

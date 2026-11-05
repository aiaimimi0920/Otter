//! 独立RGBA oracle核调用时Canvas、后续修改和原操作预算；不调用新host转换来自证。
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;

fn mount(source: &str) -> Command {
    Command::Mount {
        source: source.into(),
        snapshot: crate::surface_javascript::fixture::snapshot(1, 1),
        resources: json!({}),
    }
}
fn rgba(frame: &wire::canvas::CanvasFrame) -> Vec<u8> {
    let bytes = STANDARD.decode(&frame.png).unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    assert_eq!(
        [reader.info().width, reader.info().height],
        [frame.width, frame.height]
    );
    let mut out = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut out).unwrap();
    out.truncate(info.buffer_size());
    out
}
#[test]
fn canvas_service_call_time_pixels_intrinsic_size_alpha_and_later_edits_are_independent() {
    let mut engine = Engine::new().unwrap();
    let mut calls = 0;
    let output=engine.apply_services(mount(r#"
      NeuroSurface.define({async mount({root}) {
        const c=document.createElement('canvas');c.width=3;c.height=1;c.style.boxSizing='border-box';c.style.width='120px';root.appendChild(c);
        const g=c.getContext('2d');g.fillStyle='red';g.fillRect(0,0,1,1);
        g.fillStyle='rgba(0,0,255,0.5)';g.fillRect(1,0,1,1);
        const promise=NeuroSurface.serviceCanvases('s',{},[c],5000);
        c.width=1;g.fillStyle='green';g.fillRect(0,0,1,1);
        await promise;
      }});
    "#),None,&mut |call| {
        calls+=1;assert!(call.images.is_empty());assert_eq!(call.canvases.len(),1);
        let f=&call.canvases[0];assert_eq!([f.width,f.height],[3,1]);
        assert_eq!(rgba(f),[255,0,0,255, 0,0,255,128, 0,0,0,0]);
        Ok(wire::service::ResultValue::Succeeded{output:json!(null)})
    }).unwrap();
    assert_eq!(calls, 1);
    let scene = wire::canvas::scene(&output.scene).unwrap();
    let images = wire::canvas::images(&scene, output.canvases).unwrap();
    let image = images.values().next().unwrap();
    assert_eq!([image.width, image.height], [1, 1]);
    assert_eq!(image.pixels.as_ref(), [0, 128, 0, 255]);
}
#[test]
fn canvas_service_noop_final_collect_preserves_original_eight_submission_budget() {
    let mut engine = Engine::new().unwrap();
    let mut calls = 0;
    let output=engine.apply_services(mount(r#"
      NeuroSurface.define({async mount({root}) {
        const a=[];for(let i=0;i<8;i++){const c=document.createElement('canvas');c.width=1;c.height=1;root.appendChild(c);a.push(c);}
        await NeuroSurface.serviceCanvases('s',null,[a[0],a[1],a[2],a[3]]);
      }});
    "#),None,&mut |call| {calls+=1;assert_eq!(call.canvases.len(),4);Ok(wire::service::ResultValue::Succeeded{output:json!(null)})}).unwrap();
    assert_eq!(calls, 1);
    assert_eq!(output.canvases.len(), 8);
    let mut engine = Engine::new().unwrap();
    let mut calls = 0;
    assert!(engine.apply_services(mount(r#"
      NeuroSurface.define({async mount(){const c=document.createElement('canvas');c.width=1;c.height=1;const g=c.getContext('2d');
       for(let i=0;i<9;i++){g.fillRect(0,0,1,1);await NeuroSurface.serviceCanvases('s',null,[c]);}}});
    "#),None,&mut |_| {calls+=1;Ok(wire::service::ResultValue::Succeeded{output:json!(null)})}).is_err());
    assert_eq!(calls, 8);
}
#[test]
fn canvas_service_rejects_foreign_duplicate_zero_getter_and_noncanvas_before_dispatch() {
    for expression in [
        "[{}]",
        "[document.createElement('span')]",
        "[c,c]",
        "[]",
        "[c,c,c,c,c]",
        "(()=>{c.width=0;return [c]})()",
        "(()=>{const a=[];Object.defineProperty(a,0,{get(){throw 1}});a.length=1;return a})()",
    ] {
        let source=format!("NeuroSurface.define({{async mount(){{const c=document.createElement('canvas');c.width=1;c.height=1;await NeuroSurface.serviceCanvases('s',null,{expression});}}}});");
        let mut engine = Engine::new().unwrap();
        let mut calls = 0;
        assert!(
            engine
                .apply_services(mount(&source), None, &mut |_| {
                    calls += 1;
                    Ok(wire::service::ResultValue::Succeeded {
                        output: json!(null),
                    })
                })
                .is_err(),
            "{expression}"
        );
        assert_eq!(calls, 0);
    }
}

#[test]
fn canvas_service_rejected_selection_and_proxy_reentry_preserve_successful_call_sequence() {
    let mut engine = Engine::new().unwrap();
    let mut calls = 0;
    engine.apply_services(mount(r#"
      NeuroSurface.define({async mount({root}) {
        const c=document.createElement('canvas');c.width=1;c.height=1;
        for(const values of [[root],[c,c],[],[{}]]) {
          let rejected=false;try{NeuroSurface.serviceCanvases('s',null,values);}catch{rejected=true;}
          if(!rejected)throw new Error('invalid selection accepted');
        }
        let blocked=false;
        const values=new Proxy([c],{getOwnPropertyDescriptor(target,key){
          try{NeuroSurface.service('s',null);}catch{blocked=true;}
          return Reflect.getOwnPropertyDescriptor(target,key);
        }});
        await NeuroSurface.serviceCanvases('s',null,values);
        if(!blocked)throw new Error('service preparation reentered');
        await NeuroSurface.service('s',null);
      }});
    "#), None, &mut |call| {
        calls += 1;
        assert_eq!(call.id, calls);
        Ok(wire::service::ResultValue::Succeeded { output:json!(null) })
    }).unwrap();
    assert_eq!(calls, 2);
}

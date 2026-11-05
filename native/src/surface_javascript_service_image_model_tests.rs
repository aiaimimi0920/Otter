//! 实际引擎图像追加、预算事务及参数重入；旧图片不会被重名结果替换。
use super::*;
use serde_json::json;
fn result(id: &str) -> wire::service::ResultValue {
    let mut image = wire::images::tests::resource();
    image.id = id.into();
    wire::service::ResultValue::Image {
        output: json!(42),
        image: wire::images::headers(std::slice::from_ref(&image))
            .unwrap()
            .remove(0),
        png: Some(image.png),
    }
}
fn mount(source: &str) -> Command {
    Command::Mount {
        source: source.into(),
        snapshot: crate::surface_javascript::fixture::snapshot(1, 1),
        resources: json!({}),
    }
}
#[test]
fn service_image_model_uses_private_handle_and_cumulative_atomic_capacity() {
    let mut engine = Engine::new().unwrap();
    let mut calls = 0;
    let output=engine.apply_services(mount(r#"
      NeuroSurface.define({async mount({root}) {
       const first=await NeuroSurface.serviceImage('s',null);
       for(let i=1;i<8;i++) await NeuroSurface.serviceImage('s',null);
       let rejected=false;try{await NeuroSurface.serviceImage('s',null);}catch(e){rejected=e.message==='image_install_failed';}
       if(!rejected)throw new Error('capacity');
       const c=document.createElement('canvas');c.width=2;c.height=2;root.appendChild(c);c.getContext('2d').drawImage(first.image,0,0);
      }});
    "#),None,&mut |call|{calls+=1;assert!(call.image_output);Ok(result(&format!("service:{calls}")))}).unwrap();
    assert_eq!(calls, 9);
    let scene = wire::canvas::scene(&output.scene).unwrap();
    let images = wire::canvas::images(&scene, output.canvases).unwrap();
    assert_eq!(
        images.into_values().next().unwrap().pixels.as_ref(),
        [0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 0, 128, 128, 128]
    );
}
#[test]
fn service_image_model_rejects_options_getters_duplicate_ids_and_reentry_without_sequence_gap() {
    let mut engine = Engine::new().unwrap();
    let mut calls = 0;
    engine.apply_services(mount(r#"
      NeuroSurface.define({async mount({root}) {
       for(const options of [{get timeoutMs(){throw 1;}},{timeoutMs:0},{images:[]},{path:'x'}]) {
        let bad=false;try{NeuroSurface.serviceImage('s',null,options);}catch{bad=true;}if(!bad)throw new Error('options');
       }
       const p=new Proxy({}, {ownKeys(){let bad=false;try{NeuroSurface.service('s',null);}catch{bad=true;}if(!bad)throw new Error('reentrant');return [];}});
       const first=await NeuroSurface.serviceImage('s',null,p);
       let duplicate=false;try{await NeuroSurface.serviceImage('s',null);}catch(e){duplicate=e.message==='image_install_failed';}
       if(!duplicate)throw new Error('replaced');root.textContent=first.output;
      }});
    "#),None,&mut |call|{calls+=1;assert_eq!(call.id,calls);Ok(result("service:one"))}).unwrap();
    assert_eq!(calls, 2);
}

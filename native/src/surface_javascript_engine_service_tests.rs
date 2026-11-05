//! 真实 QuickJS Promise 续接与协议边界；等待使用合成 dispatcher，不调用网络。
use super::*;
use serde_json::json;
fn mount(source: &str) -> Command {
    Command::Mount {
        source: source.into(),
        snapshot: crate::surface_javascript::fixture::snapshot(1, 1),
        resources: json!({}),
    }
}
#[test]
fn javascript_service_engine_sequential_calls_and_detached_microtasks_settle_before_frame() {
    let mut engine = Engine::new().unwrap();
    let mut count = 0;
    let output = engine.apply_services(mount(r#"
      NeuroSurface.define({async mount({root}) {
        const label=document.createElement('span'); root.appendChild(label);
        for(let i=0;i<3;i++) await NeuroSurface.service('capability:vendor:service.run',{i},5000);
        NeuroSurface.service('capability:vendor:service.run',{i:3},5000).then(v=>{label.textContent=v;});
      }});
    "#), None, &mut |call| {
        count += 1;
        assert_eq!(call.id, count);
        Ok(wire::service::ResultValue::Succeeded { output: json!("settled") })
    }).unwrap();
    assert_eq!(count, 4);
    assert!(output.scene.to_string().contains("settled"));
}
#[test]
fn javascript_service_engine_caps_calls_and_refuses_unrelated_pending_promise() {
    for (source, expected) in [
        ("NeuroSurface.define({async mount(){for(let i=0;i<33;i++)await NeuroSurface.service('s',null);}});", 32),
        ("NeuroSurface.define({async mount(){await new Promise(()=>{});}});", 0),
        ("NeuroSurface.service('s',null); NeuroSurface.define({mount(){}});", 0),
        ("NeuroSurface.define({async mount(){await NeuroSurface.service('s',{get x(){throw 1;}});}});", 0),
        ("NeuroSurface.define({async mount(){await NeuroSurface.service('s','中'.repeat(30000));}});", 0),
    ] {
        let mut engine = Engine::new().unwrap();
        let mut count = 0;
        assert!(engine.apply_services(mount(source), None, &mut |_| {
            count += 1;
            Ok(wire::service::ResultValue::Succeeded { output: json!(null) })
        }).is_err());
        assert_eq!(count, expected);
    }
}
#[test]
fn javascript_service_engine_wait_preserves_geometry_and_canvas_operation_limits() {
    let mut engine = Engine::new().unwrap();
    let mut count = 0;
    let output = engine.apply_services(mount(r#"
      NeuroSurface.define({async mount({root}) {
        const label=document.createElement('span'); label.textContent='before'; root.appendChild(label);
        const canvas=document.createElement('canvas'); canvas.width=2; canvas.height=1; root.appendChild(canvas);
        const context=canvas.getContext('2d'); context.fillStyle='red'; context.fillRect(0,0,1,1);
        const before=root.getBoundingClientRect().width;
        await NeuroSurface.service('s',null,5000);
        const after=root.getBoundingClientRect().width;
        context.fillStyle='blue'; context.fillRect(1,0,1,1);
        label.textContent=before === after ? 'preserved' : 'bad';
      }});
    "#), Some(crate::surface_viewport::Viewport::new([200,80],96).unwrap()), &mut |_| {
        count += 1;
        std::thread::sleep(std::time::Duration::from_millis(350));
        Ok(wire::service::ResultValue::Succeeded { output: json!(null) })
    });
    if output.is_err() {
        eprintln!(
            "service wait count={count}; {:?}; geometry={:?}",
            engine.diagnostic(),
            engine.geometry.borrow().trace
        );
        engine.context.with(|ctx| {
            let exception = ctx.catch();
            let stack = exception
                .as_object()
                .and_then(|value| value.get::<_, String>("stack").ok());
            eprintln!("synthetic service fixture exception={exception:?}; stack={stack:?}");
        });
    }
    let output = output.unwrap();
    assert_eq!(count, 1);
    assert!(output.layout_read && output.scene.to_string().contains("preserved"));
    let scene = wire::canvas::scene(&output.scene).unwrap();
    let images = wire::canvas::images(&scene, output.canvases).unwrap();
    let image = images.into_values().next().unwrap();
    assert_eq!(image.pixels.as_ref(), &[0, 0, 255, 255, 255, 0, 0, 255]);
}
#[test]
fn javascript_service_wire_rejects_wrong_types_unknown_identity_and_output_limits() {
    let mut value = json!({"id":1,"service":"s","input":null,"timeoutMs":5000});
    let call: wire::service::Call = serde_json::from_value(value.clone()).unwrap();
    call.validate().unwrap();
    value["grant"] = json!("forged");
    assert!(serde_json::from_value::<wire::service::Call>(value).is_err());
    assert!(
        serde_json::from_value::<wire::service::ResultValue>(json!({"status":"succeeded"}))
            .is_err()
    );
    let success: wire::service::ResultValue =
        serde_json::from_value(json!({"status":"succeeded","output":null})).unwrap();
    success.validate().unwrap();
    assert!(wire::service::ResultValue::Succeeded {
        output: json!("x".repeat(65536))
    }
    .validate()
    .is_err());
    assert!(wire::service::ResultValue::Failed {
        error: "secret=detail".into()
    }
    .validate()
    .is_err());
}
#[test]
fn javascript_service_confirmation_requests_prompt_but_cannot_supply_approval_or_gesture() {
    let mut engine = Engine::new().unwrap();
    let output = engine
        .apply_services(
            mount(
                r#"
        NeuroSurface.define({async mount({root}) {
            const label=document.createElement('span');root.appendChild(label);
            try {await NeuroSurface.serviceWithConfirmation('s',{n:1},{timeoutMs:1000});}
            catch(error) {label.textContent=error.message;}
        }});
    "#,
            ),
            None,
            &mut |call| {
                assert!(call.confirmation);
                assert!(!call.image_output);
                assert_eq!(call.input, json!({"n":1}));
                Ok(wire::service::ResultValue::Failed {
                    error: "local_art_confirmation_denied".into(),
                })
            },
        )
        .unwrap();
    assert!(output
        .scene
        .to_string()
        .contains("local_art_confirmation_denied"));
    for options in [
        "{approve:true}",
        "{userGestureToken:'forged'}",
        "{get timeoutMs(){return 5000}}",
        "{imageOutput:'true'}",
    ] {
        let source=format!("NeuroSurface.define({{async mount(){{await NeuroSurface.serviceWithConfirmation('s',null,{options});}}}});");
        let mut engine = Engine::new().unwrap();
        let mut calls = 0;
        assert!(engine
            .apply_services(mount(&source), None, &mut |_| {
                calls += 1;
                Ok(wire::service::ResultValue::Succeeded {
                    output: json!(null),
                })
            })
            .is_err());
        assert_eq!(calls, 0);
    }
}

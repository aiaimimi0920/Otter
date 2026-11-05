//! 真实引擎诊断同步 callback；运行失败打印私有异常仅在测试夹具，不改变产品错误脱敏。
use super::*;
fn reentrant_mount(source: &str, mode: i32) -> Output {
    let mut engine = Engine::new().unwrap();
    let output = engine.apply(
        Command::Mount {
            source: source.into(),
            snapshot: crate::surface_javascript::fixture::snapshot(3, mode),
            resources: crate::surface_javascript::fixture::resources(),
        },
        Some(crate::surface_viewport::Viewport::new([600, 400], 96).unwrap()),
    );
    if output.is_err() {
        eprintln!(
            "Reentrant native geometry trace: {:?}",
            engine.geometry.borrow().trace
        );
        engine.context.with(|ctx| {
            let exception = ctx.catch();
            let stack = exception
                .as_object()
                .and_then(|object| object.get::<_, String>("stack").ok());
            eprintln!("Reentrant Engine exception: {exception:?}: {stack:?}");
        });
    }
    output.unwrap()
}

#[test]
fn javascript_event_target_engine_native_click_preserves_synchronous_geometry_stack() {
    for (case, source) in [
        include_str!("../tests/fixtures/javascript-geometry.js"),
        include_str!("../tests/fixtures/javascript-spatial.js"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut engine = Engine::new().unwrap();
        let view = Some(crate::surface_viewport::Viewport::new([600, 400], 192).unwrap());
        let output = engine
            .apply(
                Command::Mount {
                    source: source.into(),
                    snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
                    resources: crate::surface_javascript::fixture::resources(),
                },
                view,
            )
            .unwrap();
        let node = if case == 0 {
            &output.scene["children"][0]
        } else {
            &output.scene["children"][0]["children"][2]
        };
        let target = node["id"].as_str().unwrap().to_owned();
        let result = engine.apply(Command::Click { target }, view);
        if result.is_err() {
            eprintln!(
                "Event native geometry case={case} trace: {:?}",
                engine.geometry.borrow().trace
            );
            engine.context.with(|ctx| {
                let exception = ctx.catch();
                let stack = exception
                    .as_object()
                    .and_then(|object| object.get::<_, String>("stack").ok());
                eprintln!("Event Engine exception: {exception:?}: {stack:?}");
            });
        }
        let output = result.unwrap();
        let text = output.scene["children"][if case == 0 { 2 } else { 1 }]["props"]["text"]
            .as_str()
            .unwrap();
        let report: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(
            if case == 0 {
                &report["first"]["width"]
            } else {
                &report["overlay"]["left"]
            },
            if case == 0 { 70. } else { 82. }
        );
    }
}
#[test]
fn javascript_geometry_engine_conversion_reentrancy_uses_real_native_read_and_original_stack() {
    let output = reentrant_mount(
        include_str!("../tests/fixtures/javascript-geometry-reentrancy.js"),
        42,
    );
    assert_eq!(output.scene["layout"]["width"], "60px");
    assert_eq!(output.scene["style"]["color"], "green");
    assert_eq!(output.scene["children"][0]["props"]["text"], "ready");
}

#[test]
fn javascript_geometry_engine_conversion_reentrancy_style_sheet_throw_and_reject_are_atomic() {
    for mode in 0..6 {
        eprintln!("Reentrant conversion mode: {mode}");
        let output = reentrant_mount(
            include_str!("../tests/fixtures/javascript-geometry-reentrancy-variants.js"),
            mode,
        );
        assert_eq!(
            output.scene["layout"]["width"],
            if mode < 2 { "60px" } else { "50px" }
        );
        assert_eq!(
            output.scene["style"]["color"],
            if mode < 2 { "green" } else { "blue" }
        );
        assert_eq!(output.scene["children"][0]["props"]["text"], "ready");
    }
}

#[test]
fn javascript_geometry_engine_reports_native_callback_failures_without_worker_protocol_loss() {
    for source in [
        include_str!("../tests/fixtures/javascript-geometry.js"),
        include_str!("../tests/fixtures/javascript-batch-geometry.js"),
    ] {
        let mut engine = Engine::new().unwrap();
        let output = engine.apply(
            Command::Mount {
                source: source.into(),
                snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
                resources: crate::surface_javascript::fixture::resources(),
            },
            Some(crate::surface_viewport::Viewport::new([600, 400], 192).unwrap()),
        );
        if std::env::var_os("OTTER_JS_GEOMETRY_TRACE").is_some() || output.is_err() {
            eprintln!(
                "QuickJS native geometry stages (microseconds): {:?}",
                engine.geometry.borrow().trace
            );
        }
        if let Err(error) = output {
            engine.context.with(|ctx| {
                let exception = ctx.catch();
                let stack = exception
                    .as_object()
                    .and_then(|object| object.get::<_, String>("stack").ok());
                panic!(
                    "QuickJS geometry: {error:?}: {exception:?}: {stack:?}; native-valid={}",
                    engine.geometry.borrow().valid()
                );
            });
        }
    }
}

#[test]
fn javascript_geometry_flat_lifecycle_keeps_receiver_arguments_and_cleanup() {
    let source = r#"let receiver, cleaned=false;
      function check(self,args){if(self!==receiver||args.length!==1||args[0]!==undefined)throw Error('lifecycle arguments');}
      NeuroSurface.define({async mount({root}){receiver=this;root.textContent='live';await Promise.resolve();
        return function(){if(this!==undefined||arguments.length)throw Error('cleanup receiver');cleaned=true;};},
        async update(value){if(this!==receiver||arguments.length!==1||!value.snapshot)throw Error('update arguments');},
        suspend(){check(this,arguments);},resume(){check(this,arguments);},
        dispose(){check(this,arguments);if(!cleaned)throw Error('cleanup order');}});"#;
    let mut engine = Engine::new().unwrap();
    for command in [
        Command::Mount {
            source: source.into(),
            snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
            resources: crate::surface_javascript::fixture::resources(),
        },
        Command::Update {
            snapshot: crate::surface_javascript::fixture::snapshot(4, 45),
            resources: crate::surface_javascript::fixture::resources(),
        },
        Command::Suspend,
        Command::Resume,
        Command::Dispose,
    ] {
        engine.apply(command, None).unwrap();
    }
}

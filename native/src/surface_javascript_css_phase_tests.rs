//! 测试副本的阶段归因，非生产Engine/吞吐门：仅此诊断允许至2秒以观察超250ms的尾部。
//! 原Engine/child/产品仍执行原250ms压力断言；不从该诊断退出码推断预算通过。
use super::*;

#[test]
#[ignore = "instrumented model diagnostic only, not a passing operation budget gate"]
fn javascript_css_projection_profile_phase_attribution() {
    let runtime = Runtime::new().unwrap();
    runtime.set_memory_limit(32 * 1024 * 1024);
    runtime.set_max_stack_size(256 * 1024);
    let origin = Instant::now();
    runtime.set_interrupt_handler(Some(Box::new(move || origin.elapsed().as_secs_f64() > 2.)));
    let context = Context::full(&runtime).unwrap();
    let projection = include_str!("surface_javascript_css_projection.js");
    let measured = projection
        .replacen("const project = root => {", "const project = root => { const projectStart=__cssNow();", 1)
        .replacen("} else css = sheets.computed(item.node, item.parent);", "} else {const start=__cssNow();css=sheets.computed(item.node,item.parent);__cssTimes.computed+=__cssNow()-start;}", 1)
        .replacen("const prepare = node => {", "const prepare = node => { const prepareStart=__cssNow();", 1)
        .replacen("return output;", "__cssTimes.prepare+=__cssNow()-prepareStart; return output;", 1);
    let offset = measured.rfind("return output;").unwrap();
    let measured = format!(
        "{}__cssTimes.project=__cssNow()-projectStart; {}",
        &measured[..offset],
        &measured[offset..]
    );
    let json = include_str!("surface_javascript_json.js");
    let measured_json = json.replacen(
        "const copy = value => {",
        "const copy = value => { const copyStart=__cssNow();",
        1,
    );
    let offset = measured_json.rfind("return output;").unwrap();
    let measured_json = format!(
        "{}__cssTimes.copy+=__cssNow()-copyStart; {}",
        &measured_json[..offset],
        &measured_json[offset..]
    );
    let measured_json = measured_json.replacen("const encode = value => stringify(copy(value));", "const encode = value => {const start=__cssNow();const output=stringify(copy(value));__cssTimes.encode+=__cssNow()-start;return output;};", 1);
    let bootstrap = BOOTSTRAP
        .replacen(projection, &measured, 1)
        .replacen(json, &measured_json, 1);
    context.with(|ctx| {
        let result = (|| {
            ctx.globals().set(
                "__cssNow",
                Function::new(ctx.clone(), move || origin.elapsed().as_secs_f64() * 1000.)?,
            )?;
            ctx.eval::<(), _>("var __cssTimes={computed:0,prepare:0,project:0,copy:0,encode:0};")?;
            let bridge: Object = ctx.eval(bootstrap)?;
            Module::evaluate(
                ctx.clone(),
                "surface:phase-profile",
                include_str!("../tests/fixtures/javascript-css-pressure.js"),
            )?
            .finish::<()>()?;
            let command = Command::Mount {
                source: String::new(),
                snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
                resources: crate::surface_javascript::fixture::resources(),
            };
            let command = String::from_utf8(wire::encode(&command).unwrap()).unwrap();
            bridge
                .get::<_, Function>("apply")?
                .call::<_, Promise>((command,))?
                .finish::<()>()?;
            ctx.eval::<(), _>("__cssTimes={computed:0,prepare:0,project:0,copy:0,encode:0};")?;
            let started = Instant::now();
            let output = bridge
                .get::<_, Function>("collect")?
                .call::<_, String>(())?;
            let elapsed = started.elapsed();
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&output).unwrap()["scene"]["children"]
                    .as_array()
                    .unwrap()
                    .len(),
                256
            );
            let times: String = ctx.eval("JSON.stringify(__cssTimes)")?;
            let phases: serde_json::Value = serde_json::from_str(&times).unwrap();
            let read = |name: &str| phases[name].as_f64().unwrap();
            assert!(read("copy") >= 0. && read("copy") <= read("encode"));
            assert!(read("computed") + read("prepare") <= read("project") + 1.);
            println!(
                "native-css-phase-profile: collectMs={} phases={times}",
                elapsed.as_secs_f64() * 1000.
            );
            Ok::<_, rquickjs::Error>(())
        })();
        if let Err(error) = result {
            panic!("CSS phase diagnostic: {error:?}: {:?}", ctx.catch());
        }
    });
}

//! 同生产工厂的winner容器差分/原型边界；操作计数不替代原250ms压力门。
use super::*;

// 测试副本仅恢复旧winner容器；声明、rank、遍历顺序与其他工厂均保持当前生产代码。
pub(super) fn map_winners(bootstrap: &str) -> String {
    let mut bootstrap = bootstrap.to_owned();
    for (current, old) in [
        ("const winners = op.record(), context = selectors.context(node);",
         "const winners = new op.MapType(), context = selectors.context(node);"),
        ("previous = winners[name]", "previous = op.mapGet(winners, name)"),
        ("else if (wins) winners[name] = { important, inline, specificity, value: item.value };",
         "else if (wins) op.mapSet(winners, name, { important, inline, specificity, value: item.value });"),
        ("for (const name in winners) result[name] = winners[name].value;",
         "const iterator = op.mapEntries(winners); let next;\n    while (!(next = op.mapNext(iterator)).done) result[next.value[0]] = next.value[1].value;"),
    ] {
        assert_eq!(bootstrap.matches(current).count(), 1, "{current}");
        bootstrap = bootstrap.replacen(current, old, 1);
    }
    bootstrap
}

fn counted(bootstrap: &str, source: &str) -> (serde_json::Value, serde_json::Value) {
    let start = "const computed = (node, parent) => {";
    assert_eq!(bootstrap.matches(start).count(), 1);
    let mut bootstrap = bootstrap.replacen(
        start,
        "const computed = (node, parent) => { __cascadeStats.nodes++;",
        1,
    );
    let is_map = bootstrap.contains("previous = op.mapGet(winners, name)");
    for (current, measured) in [
        ("previous = op.mapGet(winners, name)", "previous = (__cascadeStats.reads++, op.mapGet(winners, name))"),
        ("else if (wins) op.mapSet(winners, name, { important, inline, specificity, value: item.value });", "else if (wins) {__cascadeStats.writes++;op.mapSet(winners, name, { important, inline, specificity, value: item.value });}"),
        ("const iterator = op.mapEntries(winners); let next;", "const iterator = (__cascadeStats.iterators++, op.mapEntries(winners)); let next;"),
    ] {
        assert_eq!(bootstrap.matches(current).count(), usize::from(is_map), "{current}");
        bootstrap = bootstrap.replacen(current, measured, 1);
    }
    let runtime = Runtime::new().unwrap();
    runtime.set_memory_limit(32 * 1024 * 1024);
    runtime.set_max_stack_size(256 * 1024);
    let context = Context::full(&runtime).unwrap();
    context.with(|ctx| {
        let result = (|| {
            ctx.eval::<(), _>("var __cascadeStats={nodes:0,reads:0,writes:0,iterators:0};")?;
            let bridge: Object = ctx.eval(bootstrap)?;
            Module::evaluate(ctx.clone(), "surface:cascade-record-count", source)?
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
            let output = bridge
                .get::<_, Function>("collect")?
                .call::<_, String>(())?;
            let counts: String = ctx.eval("JSON.stringify(__cascadeStats)")?;
            Ok::<_, rquickjs::Error>((
                serde_json::from_str(&output).unwrap(),
                serde_json::from_str(&counts).unwrap(),
            ))
        })();
        result.unwrap_or_else(|error| panic!("Cascade record model: {error:?}: {:?}", ctx.catch()))
    })
}
#[test]
fn javascript_css_cascade_record_avoids_per_declaration_winner_map_intrinsics() {
    let source = r#"NeuroSurface.define({mount({root}){
      const sheet=new CSSStyleSheet();sheet.replaceSync('.row{opacity:.5;color:green}'.repeat(16));
      document.adoptedStyleSheets=[sheet];
      for(let i=0;i<64;i++){const n=document.createElement('div');n.className='row';root.append(n)}
    }});"#;
    let (output, counts) = counted(BOOTSTRAP, source);
    let nodes = output["scene"]["children"].as_array().unwrap();
    assert_eq!(nodes.len(), 64);
    for node in nodes {
        assert_eq!(node["style"]["opacity"], 0.5);
        assert_eq!(node["style"]["color"], "green");
    }
    assert_eq!(
        counts,
        serde_json::json!({"nodes":65,"reads":0,"writes":0,"iterators":0})
    );
    let (reference, counts) = counted(&map_winners(BOOTSTRAP), source);
    assert_eq!(reference, output);
    assert_eq!(
        counts,
        serde_json::json!({"nodes":65,"reads":2048,"writes":128,"iterators":65})
    );
}

#[test]
fn javascript_css_cascade_record_keys_ignore_inherited_accessors_and_enumeration() {
    let source = r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div class="row"></div><div></div>';
      root.style.color='blue';
      const sheet=new CSSStyleSheet();
      sheet.replaceSync('.row{box-sizing:border-box;width:30px;height:10px;opacity:.5;color:green;background:red}');
      document.adoptedStyleSheets=[sheet];root.firstChild.style.opacity='.75';
      for(const key of ['opacity','color','background','background-color','width','constructor','__proto__']){
        Object.defineProperty(Object.prototype,key,{configurable:true,enumerable:true,
          get(){throw Error('inherited CSS key read')},set(){throw Error('inherited CSS key write')}});
      }
      Object.keys=Object.entries=Object.create=()=>{throw Error('public record intrinsic')};
      Map.prototype.get=Map.prototype.set=Map.prototype.entries=()=>{throw Error('public Map intrinsic')};
    }});"#;
    let (output, _) = counted(BOOTSTRAP, source);
    let children = output["scene"]["children"].as_array().unwrap();
    assert_eq!(children[0]["style"]["opacity"], 0.75);
    assert_eq!(children[0]["style"]["color"], "green");
    assert_eq!(children[0]["style"]["background"], "red");
    assert_eq!(children[0]["layout"]["width"], "30px");
    assert_eq!(children[1]["style"]["color"], "blue");
    assert!(children[1]["style"].get("opacity").is_none());
    assert_eq!(counted(&map_winners(BOOTSTRAP), source).0, output);
}

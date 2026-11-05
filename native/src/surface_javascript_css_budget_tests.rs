//! 同一生产Engine的固定样式投影诊断；不修改250ms预算，不把诊断退出码当吞吐门。
use super::*;
use crate::surface_javascript::fixture;

fn observe(label: &str, round: u32, selector: &str, rules: usize) {
    let source = format!(
        r#"NeuroSurface.define({{mount({{root}}){{
          for(let i=0;i<256;i++){{const node=document.createElement('button');node.className='row';root.appendChild(node)}}
          const rules=Array.from({{length:{rules}}},(_,i)=>'{selector}'.replace('INDEX',i+1)+'{{opacity:.5}}').join('');
          const sheet=new CSSStyleSheet();sheet.replaceSync(rules);document.adoptedStyleSheets=[sheet];
        }}}});"#
    );
    let mut engine = Engine::new().unwrap();
    let started = Instant::now();
    let result = engine.apply(
        Command::Mount {
            source,
            snapshot: fixture::snapshot(3, 42),
            resources: fixture::resources(),
        },
        None,
    );
    println!(
        "native-css-projection-profile: {}",
        serde_json::json!({"label":label,"round":round,"nodes":256,"rules":rules,
            "wallUs":started.elapsed().as_micros(),"operationSucceeded":result.is_ok(),
            "children":result.as_ref().ok().map(|out| out.scene["children"].as_array().unwrap().len()),
            "diagnostic":engine.diagnostic()})
    );
    if let Err(error) = result {
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}

#[test]
#[ignore = "fixed diagnostic matrix only; explicit before/after run, not a passing budget gate"]
fn javascript_css_projection_profile_fixed_nodes_and_selector_variants() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for (label, selector, rules) in [
        ("no-rules", "button", 0),
        ("tag", "button", 16),
        ("class", ".row", 16),
        ("tag-nth", "button:nth-child(INDEXn+1)", 16),
        ("class-nth", ".row:nth-child(INDEXn+1)", 16),
    ] {
        for round in 0..3 {
            observe(label, round, selector, rules);
        }
    }
}

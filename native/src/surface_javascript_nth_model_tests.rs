//! 生产 QuickJS 的 An+B 与 Rust 整数 oracle；不以模型代替真实 child/像素/预算。
use super::document_model_tests::collect;
#[test]
fn javascript_nth_child_model_formulas_match_integer_oracle_and_ignore_text() {
    let mut cases = vec![
        ("odd".to_owned(), 2, 1),
        ("EVEN".to_owned(), 2, 0),
        ("n".to_owned(), 1, 0),
        ("+n".to_owned(), 1, 0),
        ("-n+3".to_owned(), -1, 3),
        (" 2n + 1 ".to_owned(), 2, 1),
        ("0".to_owned(), 0, 0),
        ("+3".to_owned(), 0, 3),
        ("-3".to_owned(), 0, -3),
        ("2147483647n-2147483646".to_owned(), 2147483647, -2147483646),
        (
            "-2147483647n+2147483647".to_owned(),
            -2147483647,
            2147483647,
        ),
    ];
    for a in -4..=4 {
        for b in -6..=6 {
            cases.push((format!("{a}n{b:+}"), a, b));
        }
    }
    for (formula, a, b) in cases {
        let source = format!(
            r#"NeuroSurface.define({{mount({{root}}){{
            root.innerHTML=' \n'+('<button></button> \n<div></div> \n').repeat(6);
            const sheet=new CSSStyleSheet();sheet.replaceSync('button,div{{opacity:.1}}:root > :nth-child({formula}){{opacity:1}}');
            document.adoptedStyleSheets=[sheet];
        }}}});"#
        );
        let value = collect(&source);
        let children = value["scene"]["children"].as_array().unwrap();
        assert_eq!(children.len(), 12);
        for (offset, child) in children.iter().enumerate() {
            let difference = offset as i64 + 1 - i64::from(b);
            let a = i64::from(a);
            let matches = if a == 0 {
                difference == 0
            } else {
                difference % a == 0 && difference / a >= 0
            };
            assert_eq!(
                child["style"]["opacity"],
                if matches { 1. } else { 0.1 },
                "{formula}, index={}",
                offset + 1
            );
        }
    }
}
#[test]
fn javascript_nth_child_model_rejects_invalid_formulas_atomically() {
    let source = r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<button></button><button></button>';
      const sheet=new CSSStyleSheet();sheet.replaceSync('button:nth-child(2){color:green}');document.adoptedStyleSheets=[sheet];
      for(const bad of ['', 'n+', '2 n', '+ n', 'n + -1', '1.5n', '2n of button', 'var(--a)', '2n+1.0', '2n--1', '2n+1 junk', 'oddly', '2147483648', '-2147483648n', 'n+2147483648', '9'.repeat(400), '2n\u00a0+1']){
        let rejected=false;try{sheet.replaceSync('button:nth-child('+bad+'){color:red}')}catch{rejected=true}
        if(!rejected)throw Error('invalid formula accepted');
      }
    }});"#;
    let value = collect(source);
    assert_eq!(value["scene"]["children"][1]["style"]["color"], "green");
}
#[test]
fn javascript_nth_child_model_hidden_siblings_ancestors_and_specificity() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
        root.innerHTML='<div class="group"></div> \n<div class="group"><button class="cell"></button> \n<button class="cell"></button></div>';
        const sheet=new CSSStyleSheet();sheet.replaceSync(':root:nth-child(1){opacity:.5}.group:first-child{display:none}.group:nth-child(2) > button:nth-child(2){color:green}.cell{color:blue}.group:nth-child(2) button:nth-child(-n+1){opacity:.25}');
        document.adoptedStyleSheets=[sheet];
    }});"#,
    );
    let root = &value["scene"];
    assert_eq!(root["style"]["opacity"], 0.5);
    assert_eq!(root["children"][0]["props"]["visible"], false);
    let buttons = &root["children"][1]["children"];
    assert_eq!(buttons[0]["style"]["color"], "blue");
    assert_eq!(buttons[0]["style"]["opacity"], 0.25);
    assert_eq!(buttons[1]["style"]["color"], "green");
}
#[test]
#[ignore = "explicit 256-node/16-rule pressure gate; original completion assertion and 250ms budget remain strict"]
fn javascript_nth_child_engine_multiple_rules_use_original_operation_budget() {
    let mut engine = super::Engine::new().unwrap();
    let output = engine.apply(super::Command::Mount {
        source: r#"NeuroSurface.define({mount({root}){
          for(let i=0;i<256;i++){const node=document.createElement('button');node.className='row';root.appendChild(node)}
          const rules=Array.from({length:16},(_,i)=>'.row:nth-child('+(i+1)+'n+1){opacity:.5}').join('');
          const sheet=new CSSStyleSheet();sheet.replaceSync(rules);document.adoptedStyleSheets=[sheet];
        }});"#.into(),
        snapshot: crate::surface_javascript::fixture::snapshot(3,42),
        resources: crate::surface_javascript::fixture::resources(),
    }, None);
    diagnose(&engine, &output);
    let output = output.unwrap();
    assert_eq!(output.scene["children"].as_array().unwrap().len(), 256);
    println!(
        "native-javascript-nth-child-pressure: nodes=256 rules=16 report={:?}",
        engine.diagnostic()
    );
}
fn diagnose(engine: &super::Engine, output: &std::io::Result<super::Output>) {
    if output.is_err() {
        eprintln!("Nth trusted Engine report: {:?}", engine.diagnostic());
        engine.context.with(|ctx| {
            let exception = ctx.catch();
            let stack = exception
                .as_object()
                .and_then(|object| object.get::<_, String>("stack").ok());
            eprintln!("Nth trusted fixture exception: {exception:?}: {stack:?}");
        });
    }
}
#[test]
fn javascript_nth_child_engine_synchronous_geometry_uses_original_stack() {
    let mut engine = super::Engine::new().unwrap();
    let output = engine.apply(
        super::Command::Mount {
            source: include_str!("../tests/fixtures/javascript-nth-child-geometry.js").into(),
            snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
            resources: crate::surface_javascript::fixture::resources(),
        },
        Some(crate::surface_viewport::Viewport::new([200, 80], 96).unwrap()),
    );
    diagnose(&engine, &output);
    output.unwrap();
}

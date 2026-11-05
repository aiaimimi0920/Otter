//! 动态谓词的原 Engine 压力/深度边界；真实 child 与 HWND 另验，不放宽预算。
use super::*;
fn apply(engine: &mut Engine, command: Command) -> Output {
    let result = engine.apply(
        command,
        Some(crate::surface_viewport::Viewport::new([200, 100], 96).unwrap()),
    );
    if result.is_err() {
        eprintln!("Hover trusted Engine report: {:?}", engine.diagnostic());
        engine.context.with(|ctx| {
            let error = ctx.catch();
            let stack = error
                .as_object()
                .and_then(|object| object.get::<_, String>("stack").ok());
            eprintln!("Hover fixture exception: {error:?}: {stack:?}");
        });
    }
    result.unwrap()
}
#[test]
fn javascript_css_hover_engine_fixture_uses_original_stack_and_geometry_budget() {
    let mut engine = Engine::new().unwrap();
    mount(
        &mut engine,
        crate::surface_javascript::css_hover_tests::SOURCE,
    );
}
fn mount(engine: &mut Engine, source: &str) -> Output {
    apply(
        engine,
        Command::Mount {
            source: source.into(),
            snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
            resources: crate::surface_javascript::fixture::resources(),
        },
    )
}
#[test]
fn javascript_css_hover_engine_private_chain_matches_32_levels_and_detachment_clears_all() {
    let mut engine = Engine::new().unwrap();
    let frame = mount(
        &mut engine,
        r#"
      let top;
      NeuroSurface.define({mount({root}){
        const sheet=new CSSStyleSheet();sheet.replaceSync('div{opacity:.1}div:hover{opacity:.5}');document.adoptedStyleSheets=[sheet];
        let parent=root;for(let i=0;i<31;i++){const n=document.createElement('div');parent.append(n);parent=n;if(i===0)top=n}
      },update(){top.remove()}});
    "#,
    );
    let mut node = &frame.scene["children"][0];
    for _ in 0..30 {
        node = &node["children"][0];
    }
    let id = node["id"].as_str().unwrap();
    let frame = apply(
        &mut engine,
        Command::LocalInput {
            input: crate::surface_javascript::hover_tests::input(Some(id)),
        },
    );
    let mut node = &frame.scene["children"][0];
    for level in 0..31 {
        assert_eq!(node["style"]["opacity"], 0.5, "level={level}");
        if level < 30 {
            node = &node["children"][0];
        }
    }
    let frame = apply(
        &mut engine,
        Command::Update {
            snapshot: crate::surface_javascript::fixture::snapshot(4, 45),
            resources: crate::surface_javascript::fixture::resources(),
        },
    );
    assert!(frame.scene["children"].as_array().unwrap().is_empty());
    apply(
        &mut engine,
        Command::LocalInput {
            input: crate::surface_javascript::hover_tests::input(None),
        },
    );
}
#[test]
#[ignore = "explicit 256-node/16-rule hover pressure gate with unchanged 250ms operation budget"]
fn javascript_css_hover_engine_256_nodes_16_rules_use_original_budget() {
    let mut engine = Engine::new().unwrap();
    let frame = mount(
        &mut engine,
        r#"NeuroSurface.define({mount({root}){
      const rules=Array.from({length:16},()=>'.row:hover{opacity:.5}').join('');
      const sheet=new CSSStyleSheet();sheet.replaceSync('.row{opacity:.1}'+rules);document.adoptedStyleSheets=[sheet];
      for(let i=0;i<256;i++){const n=document.createElement('div');n.className='row';root.append(n)}
    }});"#,
    );
    let ids: Vec<_> = frame.scene["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(ids.len(), 256);
    for target in [Some(ids[127].as_str()), None] {
        let frame = apply(
            &mut engine,
            Command::LocalInput {
                input: crate::surface_javascript::hover_tests::input(target),
            },
        );
        let nodes = frame.scene["children"].as_array().unwrap();
        assert_eq!(nodes.len(), 256);
        for (index, node) in nodes.iter().enumerate() {
            assert_eq!(node["id"], ids[index]);
            assert_eq!(
                node["style"]["opacity"],
                if target.is_some() && index == 127 {
                    0.5
                } else {
                    0.1
                }
            );
        }
    }
    println!("native-css-hover-pressure: nodes=256 rules=16 original-budget=true same-engine=true");
}

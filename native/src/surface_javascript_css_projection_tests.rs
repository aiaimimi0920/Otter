//! 稀疏投影/层叠与原算法在真实QuickJS差分；不复制CSS转换或放宽产品预算。
use super::*;

fn collect(bootstrap: &str, source: &str) -> Option<serde_json::Value> {
    let runtime = Runtime::new().unwrap();
    runtime.set_memory_limit(32 * 1024 * 1024);
    runtime.set_max_stack_size(256 * 1024);
    let context = Context::full(&runtime).unwrap();
    let geometry = Rc::new(RefCell::new(geometry::Geometry::default()));
    geometry
        .borrow_mut()
        .begin(
            Some(crate::surface_viewport::Viewport::new([200, 100], 96).unwrap()),
            Instant::now() + std::time::Duration::from_millis(250),
        )
        .unwrap();
    context.with(|ctx| {
        let result = (|| {
            let bridge: Object = ctx.eval(bootstrap)?;
            let measure = geometry.clone();
            bridge
                .get::<_, Function>("bindGeometry")?
                .call::<_, ()>((Function::new(
                    ctx.clone(),
                    move |source: String, target: String| {
                        measure.borrow_mut().read(&source, &target)
                    },
                )?,))?;
            bridge
                .get::<_, Function>("beginGeometry")?
                .call::<_, ()>(("1",))?;
            Module::evaluate(ctx.clone(), "surface:projection-test", source)?.finish::<()>()?;
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
            bridge.get::<_, Function>("collect")?.call::<_, String>(())
        })();
        match result {
            Ok(value) => Some(serde_json::from_str(&value).unwrap()),
            Err(error) => {
                eprintln!(
                    "Projection differential rejection: {error:?}: {:?}",
                    ctx.catch()
                );
                None
            }
        }
    })
}

fn reference() -> String {
    let bootstrap = super::cascade_record_tests::map_winners(BOOTSTRAP);
    let projection = include_str!("surface_javascript_css_projection.js");
    let sparse = "for (const name in css) {\n        const definition = op.mapGet(values.definitions, name);";
    let full = "const iterator = op.mapEntries(values.definitions); let next;\n      while (!(next = op.mapNext(iterator)).done) {\n        const name = next.value[0], definition = next.value[1];\n        if (!op.own(css, name)) continue;";
    assert_eq!(projection.matches(sparse).count(), 1);
    let reference = projection.replacen(sparse, full, 1);
    assert_eq!(bootstrap.matches(projection).count(), 1);
    let bootstrap = bootstrap.replacen(projection, &reference, 1).replacen(
        "selector.matches(node, context)",
        "selector.matches(node)",
        1,
    );
    let compiled = "const matches = selectors.list(selector), store = declarations.parse(body), entries = op.array();";
    assert_eq!(bootstrap.matches(compiled).count(), 1);
    let bootstrap = bootstrap.replacen(compiled, "const matches = selectors.list(selector), store = declarations.parse(body), entries = store;", 1);
    let snapshot = "while (!(next = op.mapNext(iterator)).done) op.push(entries, next.value);";
    assert_eq!(bootstrap.matches(snapshot).count(), 1);
    let bootstrap = bootstrap.replacen(snapshot, "", 1);
    let apply = "const iterator = inline ? op.mapEntries(store) : null; let index = 0, next;\n      while (inline ? !(next = op.mapNext(iterator)).done : index < store.length) {\n        const entry = inline ? next.value : store[index++], name = entry[0], item = entry[1];";
    assert_eq!(bootstrap.matches(apply).count(), 1);
    let bootstrap = bootstrap.replacen(apply, "const iterator = op.mapEntries(store); let next;\n      while (!(next = op.mapNext(iterator)).done) {\n        const name = next.value[0], item = next.value[1];", 1);
    let rank = r#"const important = item.priority === "important", previous = op.mapGet(winners, name);
        const wins = !previous || (important !== previous.important ? important :
          inline !== previous.inline ? inline > previous.inline : greater(specificity, previous.specificity));
        if (wins && previous) {
          // winner只活在本次computed内；更新已有自有字段，不为相同属性的后来声明反复分配。
          previous.important = important; previous.inline = inline;
          previous.specificity = specificity; previous.value = item.value;
        } else if (wins) op.mapSet(winners, name, { important, inline, specificity, value: item.value });"#;
    assert_eq!(bootstrap.matches(rank).count(), 1);
    let winners = "const winners = new op.MapType(), context = selectors.context(node);";
    assert_eq!(bootstrap.matches(winners).count(), 1);
    let bootstrap = bootstrap.replacen(winners, "const winners = new op.MapType(), context = selectors.context(node); let order = 0;", 1)
        .replacen(rank, r#"const rank = [op.number(item.priority === "important"), inline, specificity[0], specificity[1], specificity[2], order++];
        const previous = op.mapGet(winners, name);
        if (!previous || greater(rank, previous.rank)) op.mapSet(winners, name, { rank, value: item.value });"#, 1);
    let greater = "const greater = (a, b) => a[0] !== b[0] ? a[0] > b[0] :\n    a[1] !== b[1] ? a[1] > b[1] : a[2] >= b[2];";
    assert_eq!(bootstrap.matches(greater).count(), 1);
    let bootstrap = bootstrap.replacen(greater, "const greater = (a, b) => {for(let i=0;i<a.length;i++)if(a[i]!==b[i])return a[i]>b[i];return true;};", 1);
    let predicate = "classes++; op.push(tests, s => op.exec(pattern, op.mapGet(s.attrs, \"class\") ?? \"\") !== null);";
    assert_eq!(bootstrap.matches(predicate).count(), 1);
    bootstrap.replacen(predicate, "classes++; op.push(tests, s => op.indexOf(op.words(op.mapGet(s.attrs, \"class\") ?? \"\"), value) >= 0);", 1)
}

#[test]
fn javascript_css_projection_sparse_matches_full_definition_scan() {
    let reference = reference();
    for source in [
        include_str!("../tests/fixtures/javascript-css.js"),
        include_str!("../tests/fixtures/javascript-css-rejection.js"),
        include_str!("../tests/fixtures/javascript-css-integrity.js"),
        include_str!("../tests/fixtures/javascript-grid.js"),
        include_str!("../tests/fixtures/javascript-grid-rejection.js"),
        include_str!("../tests/fixtures/javascript-grid-integrity.js"),
        include_str!("../tests/fixtures/javascript-spatial.js"),
        include_str!("../tests/fixtures/javascript-spatial-rejection.js"),
        include_str!("../tests/fixtures/javascript-spatial-integrity.js"),
        include_str!("../tests/fixtures/javascript-padding.js"),
        include_str!("../tests/fixtures/javascript-padding-integrity.js"),
        include_str!("../tests/fixtures/javascript-fonts.js"),
        include_str!("../tests/fixtures/javascript-fonts-integrity.js"),
        include_str!("../tests/fixtures/javascript-nth-child.js"),
    ] {
        let expected = collect(&reference, source).expect("reference fixture must succeed");
        assert_eq!(collect(BOOTSTRAP, source), Some(expected), "{source}");
    }
}

#[test]
fn javascript_css_projection_sparse_preserves_contextual_rejections() {
    let reference = reference();
    for css in [
        "width:10px",
        "border-width:1px;border-style:solid", // 正 used width 仍必须显式 border-box。
        "display:flex;align-items:baseline",
        "grid-row-start:1",
        "top:1px",
        "overflow-x:hidden",
    ] {
        let source =
            format!("NeuroSurface.define({{mount({{root}}){{root.style.cssText='{css}'}}}});");
        assert!(collect(&reference, &source).is_none(), "{css}");
        assert!(collect(BOOTSTRAP, &source).is_none(), "{css}");
    }
}

#[test]
fn javascript_css_projection_cascade_rank_matches_lexicographic_oracle() {
    let reference = reference();
    let selectors = [
        ("button", [0, 0, 1]),
        (".row", [0, 1, 0]),
        ("#target", [1, 0, 0]),
        ("button.row", [0, 1, 1]),
        ("#missing, button", [0, 0, 1]),
        ("[data-x=yes]", [0, 1, 0]),
        (":root > .row", [0, 2, 0]),
        (".row.row.row.row", [0, 4, 0]),
    ];
    // 固定排列覆盖important/inline/三项specificity和相同rank的跨sheet源序。
    for round in 0..32 {
        let mut sheets = [String::new(), String::new()];
        let mut candidates = Vec::new();
        for offset in 0..16 {
            let index = (offset + round) % selectors.len();
            let (selector, specificity) = selectors[index];
            let important = (round >> (offset % 5)) & 1 != 0;
            let value = (offset + 1) as f64 / 100.;
            let sheet = offset / 8;
            sheets[sheet].push_str(&format!(
                "{selector}{{opacity:{value}{}}}",
                if important { "!important" } else { "" }
            ));
            candidates.push((important, 0, specificity, sheet, offset % 8, value));
        }
        let reverse = round % 2 != 0;
        let inline_important = round % 3 == 0;
        let mut expected = ((false, 0, [0, 0, 0], 0, 0), 0.);
        for (important, inline, specificity, sheet, order, value) in candidates {
            let source = if reverse { 1 - sheet } else { sheet };
            let rank = (important, inline, specificity, source, order);
            if rank >= expected.0 {
                expected = (rank, value);
            }
        }
        let rank = (inline_important, 1, [0, 0, 0], 2, 0);
        if rank >= expected.0 {
            expected = (rank, 0.75);
        }
        let source = format!(
            r#"NeuroSurface.define({{mount({{root}}){{
              root.innerHTML='<button id="target" class="row" data-x="yes"></button>';
              const a=new CSSStyleSheet(),b=new CSSStyleSheet();a.replaceSync({});b.replaceSync({});
              document.adoptedStyleSheets={};
              root.firstChild.style.setProperty('opacity','.75',{});
            }}}});"#,
            serde_json::to_string(&sheets[0]).unwrap(),
            serde_json::to_string(&sheets[1]).unwrap(),
            if reverse { "[b,a]" } else { "[a,b]" },
            if inline_important {
                "'important'"
            } else {
                "''"
            }
        );
        let output = collect(BOOTSTRAP, &source).unwrap();
        assert_eq!(
            output["scene"]["children"][0]["style"]["opacity"],
            expected.1
        );
        assert_eq!(collect(&reference, &source), Some(output), "round={round}");
    }
}

#[test]
fn javascript_css_projection_cascade_winners_are_local_and_own_data() {
    let source = r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div class="row a"></div><div class="row b"></div>';
      const a=root.children[0],b=root.children[1],base=new CSSStyleSheet(),extra=new CSSStyleSheet();
      base.replaceSync('.row{box-sizing:border-box;width:30px;height:10px;color:red;opacity:.1}.row{color:blue;opacity:.2}');
      extra.replaceSync('.a{width:40px!important;opacity:.3}.b{height:14px}');
      document.adoptedStyleSheets=[base,extra];b.style.width='50px';
      const check=(node,width,height)=>{const r=node.getBoundingClientRect();if(r.width!==width||r.height!==height)throw Error('cascade geometry changed')};
      check(a,40,10);check(b,50,14);
      extra.replaceSync('.a{height:16px;color:green}.b{opacity:.4}');
      check(a,30,16);check(b,50,10);
      b.style.removeProperty('width');document.adoptedStyleSheets=[base];
      check(a,30,10);check(b,30,10);
      extra.replaceSync('.a{color:green;opacity:.3}.b{color:red;opacity:.4}');
      document.adoptedStyleSheets=[base,extra];
      // value最后安装，避免测试自己的后续PropertyDescriptor继承它，污染本次winner字段即可。
      for(const key of ['important','inline','specificity','value'])Object.defineProperty(Object.prototype,key,{
        configurable:true,get(){throw Error('inherited winner read')},set(){throw Error('inherited winner write')}
      });
    }});"#;
    let output = collect(BOOTSTRAP, source).unwrap();
    let children = output["scene"]["children"].as_array().unwrap();
    assert_eq!(children[0]["style"]["color"], "green");
    assert_eq!(children[0]["style"]["opacity"], 0.3);
    assert_eq!(children[1]["style"]["color"], "red");
    assert_eq!(children[1]["style"]["opacity"], 0.4);
    assert_eq!(collect(&reference(), source), Some(output));
}

#[test]
fn javascript_css_projection_sparse_does_not_enumerate_public_prototypes() {
    let value = super::document_model_tests::collect(
        r#"NeuroSurface.define({mount({root}){
          root.innerHTML='<p>safe</p>';root.className='row';
          Object.defineProperty(Object.prototype,'width',{enumerable:true,get(){throw Error('prototype width')}});
          Object.defineProperty(Object.prototype,'unknown',{enumerable:true,get(){throw Error('prototype unknown')}});
          Object.keys=Object.entries=Object.hasOwn=()=>{throw Error('public enumeration')};
          Map.prototype.get=Map.prototype.entries=()=>{throw Error('public map')};
          const originalRegExp=RegExp;globalThis.RegExp=()=>{throw Error('public constructor')};
          originalRegExp.prototype.exec=()=>{throw Error('public regexp')};
          const sheet=new CSSStyleSheet();sheet.replaceSync('.row{color:green}');
          document.adoptedStyleSheets=[sheet];root.style.cssText='font-weight:700;opacity:.5';
        }});"#,
    );
    assert_eq!(value["scene"]["style"]["color"], "green");
    assert_eq!(value["scene"]["style"]["fontWeight"], 700);
    assert_eq!(value["scene"]["style"]["opacity"], 0.5);
    assert!(value["scene"]["layout"].get("width").is_none());
    assert_eq!(value["scene"]["children"][0]["style"]["color"], "green");
}

#[test]
fn javascript_css_projection_class_boundaries_match_ascii_word_oracle() {
    let fields = vec![
        "".to_owned(),
        "a".to_owned(),
        "A".to_owned(),
        "a-b".to_owned(),
        "_x0".to_owned(),
        "aa".to_owned(),
        "ba ab".to_owned(),
        "b a b".to_owned(),
        "a a".to_owned(),
        "\ta\nA\r_x0\x0ca-b ".to_owned(),
        "a\u{a0}b".to_owned(),
        "\x0ba".to_owned(),
        "a.b".to_owned(),
        "x_y-z9".to_owned(),
        "a-bb".to_owned(),
        "a".repeat(4096),
        format!("{}a-b", " ".repeat(4093)),
    ];
    for token in ["a", "A", "a-b", "_x0", "x_y-z9"] {
        let source = format!(
            r#"NeuroSurface.define({{mount({{root}}){{
              for(const value of {}){{const node=document.createElement('button');node.className=value;root.appendChild(node)}}
              const sheet=new CSSStyleSheet();sheet.replaceSync('.{token}{{opacity:.25}}');document.adoptedStyleSheets=[sheet];
            }}}});"#,
            serde_json::to_string(&fields).unwrap()
        );
        let value = collect(BOOTSTRAP, &source).unwrap();
        for (child, field) in value["scene"]["children"]
            .as_array()
            .unwrap()
            .iter()
            .zip(&fields)
        {
            let matched = field
                .split(['\t', '\n', '\x0c', '\r', ' '])
                .any(|word| word == token);
            assert_eq!(
                child["style"].get("opacity").is_some(),
                matched,
                "{token}: {field:?}"
            );
        }
    }
}

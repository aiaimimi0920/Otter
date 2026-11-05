//! 同生产 bootstrap 的真实 QuickJS；保护声明展开、层叠、反射与原生消费字段。
use super::document_model_tests::collect;
#[test]
fn javascript_grid_model_generic_fixture_uses_same_native_grid_keys() {
    let value = collect(include_str!("../tests/fixtures/javascript-grid.js"));
    let root = &value["scene"];
    assert_eq!(root["type"], "grid");
    assert_eq!(root["layout"]["gridTemplateColumns"], "40px minmax(0,1fr)");
    assert_eq!(root["layout"]["gridTemplateRows"], "30px 30px");
    assert_eq!(root["layout"]["rowGap"], "4px");
    assert_eq!(root["layout"]["columnGap"], "8px");
    assert_eq!(root["layout"]["alignContent"], "start");
    assert_eq!(root["children"][0]["layout"]["width"], "auto");
    assert_eq!(root["children"][2]["layout"]["gridColumn"], "1 / -1");
}
#[test]
fn javascript_grid_model_shorthand_resets_longhands_before_independent_cascade() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<button>x</button>';
      const s=new CSSStyleSheet();s.replaceSync(`
        :root{display:grid;gap:4px;row-gap:8px;gap:2px;justify-items:end!important;place-items:center;place-content:start center}
        button{grid-column-end:4;grid-column:2;align-self:end!important;place-self:center}
      `);document.adoptedStyleSheets=[s];
    }});"#,
    );
    let root = &value["scene"];
    assert_eq!(root["layout"]["rowGap"], "2px");
    assert_eq!(root["layout"]["columnGap"], "2px");
    assert_eq!(root["layout"]["align"], "center");
    assert_eq!(root["layout"]["justifyItems"], "end");
    assert_eq!(root["layout"]["alignContent"], "start");
    assert_eq!(root["layout"]["justify"], "center");
    assert_eq!(root["children"][0]["layout"]["gridColumn"], "2 / auto");
    assert_eq!(root["children"][0]["layout"]["alignSelf"], "end");
    assert_eq!(root["children"][0]["layout"]["justifySelf"], "center");
}
#[test]
fn javascript_grid_model_inline_reflection_removal_priority_and_atomic_setter() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      const s=new CSSStyleSheet();s.replaceSync(':root{display:grid;gap:7px 9px;place-items:end}');document.adoptedStyleSheets=[s];
      root.style.cssText='row-gap:8px!important;gap:2px;place-items:start center';
      if(root.style.rowGap!=='8px'||root.style.columnGap!=='2px'||root.style.gap!==''||root.style.getPropertyPriority('gap')!=='')throw Error('mixed priority');
      root.style.setProperty('gap','3px 4px');
      if(root.style.gap!=='3px 4px'||root.style.getPropertyPriority('row-gap')!==''||root.style.length!==4)throw Error('replacement');
      if(root.style.removeProperty('gap')!=='3px 4px'||root.style.rowGap!==''||root.style.columnGap!=='')throw Error('remove');
      root.style.placeItems='';root.innerHTML='<button>x</button>';
      const b=root.firstChild;b.style.gridColumnEnd='4';b.style.gridColumn='+2';
      if(b.style.gridColumnEnd!=='auto'||b.style.gridColumn!=='2 / auto'||b.style.item(0)!=='grid-column-end')throw Error('reset');
      b.style.gridRow='2 span / -1';
    }});"#,
    );
    let root = &value["scene"];
    assert_eq!(root["layout"]["rowGap"], "7px");
    assert_eq!(root["layout"]["columnGap"], "9px");
    assert_eq!(root["layout"]["justifyItems"], "end");
    assert_eq!(root["children"][0]["layout"]["gridRow"], "span 2 / -1");
}
#[test]
fn javascript_grid_model_parser_preserves_old_declarations_on_rejection() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-grid-rejection.js"
    ));
    assert_eq!(value["scene"]["layout"]["rowGap"], "8px");
    assert_eq!(value["scene"]["layout"]["columnGap"], "4px");
    assert_eq!(value["scene"]["layout"]["gridTemplateColumns"], "40px auto");
}
#[test]
fn javascript_grid_model_intrinsic_pollution_cannot_change_parser_or_reflection() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-grid-integrity.js"
    ));
    assert_eq!(
        value["scene"]["layout"]["gridTemplateColumns"],
        "minmax(0,0.75fr) minmax(0,0.75fr)"
    );
    assert_eq!(value["scene"]["layout"]["rowGap"], "3px");
    assert_eq!(
        value["scene"]["children"][0]["layout"]["gridColumn"],
        "span 2 / auto"
    );
}
#[test]
fn javascript_grid_model_track_limit_and_large_valid_list_keep_original_budgets() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.style.display='grid';root.style.gridTemplateColumns=Array(64).fill('minmax(0,1fr)').join(' ');
      if(root.style.gridTemplateColumns.length<=256)throw Error('not a long list');
      const before=root.style.gridTemplateColumns;let failed=false;
      try{root.style.gridTemplateColumns=Array(65).fill('1fr').join(' ');}catch{failed=true;}
      if(!failed||root.style.gridTemplateColumns!==before)throw Error('limit');
      root.style.gridTemplateRows='20% 1fr';root.style.gridAutoRows='minmax(0,.5fr)';
    }});"#,
    );
    assert_eq!(
        value["scene"]["layout"]["gridTemplateColumns"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .count(),
        64
    );
    assert_eq!(value["scene"]["layout"]["gridAutoRows"], "minmax(0,0.5fr)");
}
#[test]
fn javascript_grid_model_flex_main_axis_gap_and_alignment_remain_consumed() {
    for (direction, expected) in [("row", "5px"), ("column", "3px")] {
        let value = collect(&format!(
            r#"NeuroSurface.define({{mount({{root}}){{root.style.cssText='display:flex;flex-direction:{direction};gap:3px 5px;align-items:normal;justify-content:start';}}}});"#
        ));
        assert_eq!(value["scene"]["layout"]["gap"], expected);
        assert_eq!(value["scene"]["layout"]["align"], "stretch");
        assert_eq!(value["scene"]["layout"]["justify"], "flex-start");
    }
}
#[test]
fn javascript_grid_model_self_normal_stretches_while_auto_inherits_parent_items() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.style.cssText='display:grid;place-items:center';root.innerHTML='<div></div><div></div>';
      root.children[0].style.placeSelf='normal';root.children[1].style.placeSelf='auto';
    }});"#,
    );
    let children = &value["scene"]["children"];
    assert_eq!(children[0]["layout"]["alignSelf"], "stretch");
    assert_eq!(children[0]["layout"]["justifySelf"], "stretch");
    assert_eq!(children[1]["layout"]["alignSelf"], "auto");
    assert_eq!(children[1]["layout"]["justifySelf"], "auto");
}

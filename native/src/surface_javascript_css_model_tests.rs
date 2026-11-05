//! 同生产 bootstrap/真实 QuickJS 的 cascade 语义诊断；进程预算、HWND 与像素另验。
use super::document_model_tests::collect;
#[test]
fn javascript_font_family_model_inherits_and_rejected_declarations_are_atomic() {
    let value = collect(include_str!("../tests/fixtures/javascript-fonts.js"));
    let root = &value["scene"];
    assert_eq!(
        root["style"]["fontFamily"],
        "\"MissingOtterFamily\",\"Consolas\",monospace"
    );
    assert_eq!(
        root["children"][0]["style"]["fontFamily"],
        root["style"]["fontFamily"]
    );
    assert_eq!(
        root["children"][1]["style"]["fontFamily"],
        "\"Segoe UI\",sans-serif"
    );
}
#[test]
fn javascript_font_family_model_uses_captured_intrinsics() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-fonts-integrity.js"
    ));
    assert_eq!(
        value["scene"]["style"]["fontFamily"],
        "\"MissingOtterFamily\",\"Microsoft YaHei\",monospace"
    );
}
#[test]
fn javascript_css_model_flex_item_auto_width_and_explicit_solid_border_have_native_meaning() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      const sheet=new CSSStyleSheet();sheet.replaceSync('*{box-sizing:border-box}:root{display:flex}div{border-style:solid}');
      document.adoptedStyleSheets=[sheet];root.innerHTML='<div><span>auto</span></div>';
    }});"#,
    );
    assert_eq!(value["scene"]["children"][0]["layout"]["width"], "auto");
    assert_eq!(value["scene"]["children"][0]["style"]["borderWidth"], "3px");
}
#[test]
fn javascript_css_model_mutable_intrinsics_cannot_change_private_css_or_its_validation() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-css-integrity.js"
    ));
    assert_eq!(value["scene"]["style"]["opacity"], 0.5);
    assert_eq!(value["scene"]["style"]["fontWeight"], 700);
    assert_eq!(value["scene"]["style"]["background"], "green");
}
#[test]
fn javascript_css_model_container_can_hide_with_retained_flex_direction_and_inherit_line_height() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      const sheet=new CSSStyleSheet();sheet.replaceSync(':root{display:flex;flex-direction:row;font-size:12px;line-height:18px}.hide{display:none}');
      document.adoptedStyleSheets=[sheet];root.className='hide';root.textContent='hidden';
    }});"#,
    );
    assert_eq!(value["scene"]["props"]["visible"], false);
    assert_eq!(value["scene"]["children"][0]["style"]["lineHeight"], "18px");
}
#[test]
fn javascript_css_model_author_cascade_maps_to_native_consumers() {
    let value = collect(include_str!("../tests/fixtures/javascript-css.js"));
    let root = &value["scene"];
    assert_eq!(root["type"], "row");
    assert_eq!(root["layout"]["gap"], "8px");
    for side in ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"] {
        assert_eq!(root["layout"][side], "4px");
    }
    assert_eq!(root["style"]["borderWidth"], "2px");
    assert_eq!(root["children"][0]["layout"]["width"], "32px");
    assert_eq!(root["children"][0]["style"]["background"], "#00ff00");
    assert_eq!(root["children"][0]["style"]["color"], "#ffffff");
    assert_eq!(root["children"][1]["style"]["textAlign"], "left");
}
#[test]
fn javascript_css_model_rejected_replacements_are_atomic() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-css-rejection.js"
    ));
    assert_eq!(value["scene"]["style"]["color"], "green");
    assert_eq!(value["scene"]["children"][0]["style"]["color"], "blue");
}
#[test]
fn javascript_css_model_specificity_order_alias_comments_and_static_pseudo_classes() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div class="a"><div class="a"><button class="x" disabled data-x="a,b">yes</button></div></div>';
      const a=new CSSStyleSheet(),b=new CSSStyleSheet();
      a.replaceSync('*{box-sizing:border-box} .a > .a .x{color:red} #surface-root button{color:blue} button:disabled{opacity:0.5} [data-x="a,b"]{background:red;background-color:green;background:blue}');
      b.replaceSync('/*first*/ #surface-root button:first-child{color:yellow} :root{font-weight:bold}');
      document.adoptedStyleSheets=[a,b];
    }});"#,
    );
    let button = &value["scene"]["children"][0]["children"][0]["children"][0];
    assert_eq!(button["style"]["color"], "yellow");
    assert_eq!(button["style"]["opacity"], 0.5);
    assert_eq!(button["style"]["background"], "blue");
    assert_eq!(button["style"]["fontWeight"], 700);
}
#[test]
fn javascript_css_model_inline_api_removal_restores_sheet_and_adoption_copy_isolated() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      const sheet=new CSSStyleSheet();sheet.replaceSync(':root{background:red!important;color:green}');
      document.adoptedStyleSheets=[sheet];root.style.cssText='background:blue!important;color:red';
      if(root.style.getPropertyValue('background-color')!=='blue'||root.style.getPropertyPriority('background')!=='important')throw Error('reflection');
      if(root.style.removeProperty('background')!=='blue')throw Error('remove');root.style.color='';
      document.adoptedStyleSheets.push(new CSSStyleSheet());
    }});"#,
    );
    assert_eq!(value["scene"]["style"]["background"], "red");
    assert_eq!(value["scene"]["style"]["color"], "green");
}

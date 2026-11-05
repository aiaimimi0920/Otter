//! 同生产 QuickJS 的空间 CSS 声明展开、层叠、反射与原子准入；像素/进程另验。
use super::document_model_tests::collect;
#[test]
fn javascript_spatial_model_shorthands_reset_and_cascade_per_longhand() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
          const s=new CSSStyleSheet();s.replaceSync(':root{position:relative;top:9px!important;inset:1px 2px 3px;overflow-x:hidden!important;overflow:visible;overflow-y:hidden}');document.adoptedStyleSheets=[s];
        }});"#,
    );
    let layout = &value["scene"]["layout"];
    for (key, expected) in [
        ("top", "9px"),
        ("right", "2px"),
        ("bottom", "3px"),
        ("left", "2px"),
        ("overflowX", "hidden"),
        ("overflowY", "hidden"),
    ] {
        assert_eq!(layout[key], expected, "{key}");
    }
}
#[test]
fn javascript_spatial_model_inset_reflection_compression_priorities_and_removal() {
    collect(
        r#"NeuroSurface.define({mount({root}){
          root.style.position='relative';
          for(const [input,expected] of [['1px','1px'],['1px 2px','1px 2px'],['1px 2px 3px','1px 2px 3px'],['1px 2px 3px 4px','1px 2px 3px 4px'],['1px 2px 1px 2px','1px 2px']]){
            root.style.inset=input;if(root.style.inset!==expected)throw Error('quad reflection');
          }
          root.style.setProperty('top','8px','important');
          if(root.style.inset!==''||root.style.getPropertyPriority('inset')!=='')throw Error('mixed priority');
          root.style.setProperty('inset','auto','important');
          if(root.style.inset!=='auto'||root.style.length!==5||root.style.getPropertyPriority('inset')!=='important')throw Error('replacement');
          if(root.style.removeProperty('inset')!=='auto'||root.style.top!==''||root.style.left!==''||root.style.length!==1)throw Error('removal');
          root.style.overflow='hidden visible';if(root.style.overflow!=='hidden visible')throw Error('axes');
          root.style.removeProperty('overflow');if(root.style.overflowX!==''||root.style.overflowY!=='')throw Error('axes removal');
        }});"#,
    );
}
#[test]
fn javascript_spatial_model_invalid_replacements_preserve_all_original_longhands() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-spatial-rejection.js"
    ));
    assert_eq!(value["scene"]["style"]["color"], "green");
}
#[test]
fn javascript_spatial_model_decimal_normalization_and_absolute_auto_width() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
          root.style.position='relative';root.innerHTML='<div></div>';
          root.firstChild.style.cssText='position:absolute;inset:.5px 1.px 0 0';
        }});"#,
    );
    let layout = &value["scene"]["children"][0]["layout"];
    assert_eq!(layout["top"], "0.5px");
    assert_eq!(layout["right"], "1px");
    assert_eq!(layout["width"], "auto");
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
          root.style.position='relative';root.innerHTML='<div></div>';
          root.firstChild.style.cssText='box-sizing:border-box;position:absolute;left:0;top:0;width:.5px;height:10.px;min-width:.25px;padding:.25px';
        }});"#,
    );
    let layout = &value["scene"]["children"][0]["layout"];
    assert_eq!(layout["width"], "0.5px");
    assert_eq!(layout["height"], "10px");
    assert_eq!(layout["minWidth"], "0.25px");
    for side in ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"] {
        assert_eq!(layout[side], "0.25px");
    }
    assert!(layout.get("padding").is_none());
}
#[test]
fn javascript_spatial_model_intrinsics_cannot_change_private_spatial_declarations() {
    let value = collect(include_str!(
        "../tests/fixtures/javascript-spatial-integrity.js"
    ));
    let layout = &value["scene"]["children"][0]["layout"];
    assert_eq!(layout["left"], "2px");
    assert_eq!(layout["overflowX"], "hidden");
    assert_eq!(layout["overflowY"], "hidden");
}

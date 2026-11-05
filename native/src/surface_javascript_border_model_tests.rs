//! 四边简写的 cascade/reset/反射与原子拒绝；不以字符串准入替代 native 几何和像素。
use super::document_model_tests::collect;
#[test]
fn javascript_border_sides_default_none_has_zero_used_width_without_box_sizing() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.style.borderWidth='2px';root.innerHTML='<div></div>';
      root.firstChild.style.cssText='border-left:4px red;border-right:hidden';
    }});"#,
    );
    assert_eq!(value["scene"]["style"]["borderWidth"], "0");
    assert_eq!(value["scene"]["children"][0]["style"]["borderWidth"], "0");
}
#[test]
fn javascript_border_sides_shorthands_cascade_reset_and_reflection() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div></div>';const box=root.firstChild;box.id='border-box';
      box.style.cssText='box-sizing:border-box;border:2px solid red;border-left:4px solid blue';
      if(box.style.border!==''||box.style.borderLeft!=='4px solid blue'||box.style.borderWidth!=='2px 2px 2px 4px')throw Error('reflect sides');
      box.style.border='green solid thin';
      if(box.style.border!=='1px solid green'||box.style.borderColor!=='green')throw Error('reset shorthand');
      const sheet=new CSSStyleSheet();sheet.replaceSync('#border-box{border-bottom:6px solid blue!important}');document.adoptedStyleSheets=[sheet];
      box.style.borderTop='0';box.style.borderRight='hidden';box.style.borderLeft='thick solid';
    }});"#,
    );
    let style = &value["scene"]["children"][0]["style"];
    assert_eq!(style["borderTopWidth"], "0");
    assert_eq!(style["borderRightWidth"], "0");
    assert_eq!(style["borderBottomWidth"], "6px");
    assert_eq!(style["borderLeftWidth"], "5px");
    assert_eq!(style["borderTopColor"], "currentcolor");
    assert_eq!(style["borderBottomColor"], "blue");
}
#[test]
fn javascript_border_sides_quad_values_atomic_rejections_and_noninheritance() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div><span>text</span></div>';const box=root.firstChild;
      box.style.cssText='box-sizing:border-box;border-width:1px 2px 3px 4px;border-style:solid none hidden solid;border-color:rgba(255, 0, 0, .5) blue';
      for(const bad of ['-1px solid red','2% solid red','1px dashed red','1px solid red blue','1px 2px solid','url(secret)','1px solid rgb(1,2)','inherit']){
        const before=box.style.cssText;let failed=false;try{box.style.cssText='color:blue;border:'+bad}catch{failed=true}
        if(!failed||box.style.cssText!==before)throw Error('atomic border');
      }
      if(box.style.removeProperty('border-left-style')!=='solid'||box.style.borderStyle!=='')throw Error('remove component');
      box.style.borderLeftStyle='solid';
    }});"#,
    );
    let node = &value["scene"]["children"][0];
    assert_eq!(node["style"]["borderLeftWidth"], "4px");
    assert_eq!(node["style"]["borderRightWidth"], "0");
    assert_eq!(node["style"]["borderTopColor"], "rgba(255, 0, 0, .5)");
    assert!(node["children"][0]["style"].get("borderWidth").is_none());
    assert!(node["children"][0]["style"]
        .get("borderLeftWidth")
        .is_none());
}
#[test]
fn javascript_border_sides_do_not_call_polluted_public_string_or_object_intrinsics() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<div></div>';const box=root.firstChild;box.style.boxSizing='border-box';
      String.prototype.lastIndexOf=String.prototype.toUpperCase=()=>{throw Error('public string')};
      Object.defineProperty(Object.prototype,'border-top-width',{get(){throw Error('prototype')},enumerable:true});
      box.style.border='1px solid rgba(255, 0, 0, .5)';box.style.borderLeft='2px solid blue';
    }});"#,
    );
    assert_eq!(
        value["scene"]["children"][0]["style"]["borderLeftWidth"],
        "2px"
    );
}

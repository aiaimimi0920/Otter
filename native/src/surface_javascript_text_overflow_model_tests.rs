//! 声明解析/层叠/非继承与污染边界；最终布局和像素由独立 native/child 门保护。
use super::document_model_tests::collect;
#[test]
fn javascript_text_overflow_model_cascade_noninheritance_and_atomic_rejections() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<span><span>full text</span></span>';
      const title=root.firstChild,sheet=new CSSStyleSheet();
      sheet.replaceSync('span{white-space:nowrap}#title{overflow:hidden;text-overflow:ellipsis!important}');
      title.id='title';document.adoptedStyleSheets=[sheet];title.style.textOverflow='clip';
      for(const bad of ['fade','ellipsis clip','"..."','inherit','url(secret)']){
        let rejected=false;try{title.style.cssText='color:red;text-overflow:'+bad}catch{rejected=true}
        if(!rejected||title.style.textOverflow!=='clip'||title.style.color!=='')throw Error('atomic inline');
        rejected=false;try{sheet.replaceSync('span{color:red}#title{text-overflow:'+bad+'}')}catch{rejected=true}
        if(!rejected)throw Error('atomic sheet');
      }
      title.style.textOverflow='ellipsis';
      if(title.style.removeProperty('text-overflow')!=='ellipsis'||title.style.textOverflow!=='')throw Error('remove');
    }});"#,
    );
    let text = &value["scene"]["children"][0];
    assert_eq!(text["style"]["textOverflow"], "ellipsis");
    assert_eq!(text["layout"]["overflowX"], "hidden");
    assert_eq!(text["layout"]["overflowY"], "hidden");
    assert_eq!(text["props"]["text"], "full text");
    assert!(value["scene"]["style"].get("textOverflow").is_none());
}
#[test]
fn javascript_text_overflow_model_uses_private_records_and_captured_intrinsics() {
    let value = collect(
        r#"NeuroSurface.define({mount({root}){
      root.innerHTML='<span>full text</span>';
      root.firstChild.style.cssText='overflow:hidden;white-space:nowrap;text-overflow:ellipsis';
      for(const name of ['text-overflow','textOverflow','overflow-x'])
        Object.defineProperty(Object.prototype,name,{get(){throw Error('prototype')},enumerable:true});
      Object.hasOwn=Object.keys=Map.prototype.get=Map.prototype.set=()=>{throw Error('public intrinsic')};
    }});"#,
    );
    assert_eq!(
        value["scene"]["children"][0]["style"]["textOverflow"],
        "ellipsis"
    );
}
#[test]
fn javascript_text_overflow_engine_reentrant_geometry_uses_original_budget() {
    let mut engine = super::Engine::new().unwrap();
    engine
        .apply(
            super::Command::Mount {
                source: include_str!("../tests/fixtures/javascript-text-overflow.js").into(),
                snapshot: crate::surface_javascript::fixture::snapshot(3, 42),
                resources: crate::surface_javascript::fixture::resources(),
            },
            Some(crate::surface_viewport::Viewport::new([400, 80], 96).unwrap()),
        )
        .unwrap();
}

//! 真实引擎的显式结果生命周期；排队不是正式工作区提交，旧 Session 仍失败关闭。
use super::*;
fn mount(source: &str) -> Command {
    Command::Mount {
        source: source.into(),
        snapshot: crate::surface_javascript::fixture::snapshot(1, 1),
        resources: serde_json::json!({}),
    }
}
#[test]
fn local_result_is_explicit_and_does_not_reappear_in_next_operation() {
    let mut engine = Engine::new().unwrap();
    let output = engine
        .apply(
            mount(
                r#"
      NeuroSurface.define({mount(){
        const outputs={answer:{kind:'value',value:null}};
        NeuroSurface.commitResult(outputs);
        outputs.answer.value='changed';
      }});
    "#,
            ),
            None,
        )
        .unwrap();
    let result = output.result.unwrap();
    result.validate().unwrap();
    assert!(
        matches!(&result.outputs["answer"], wire::result::Output::Value { value } if value.is_null())
    );
    assert!(engine
        .apply(Command::TimerTick, None)
        .unwrap()
        .result
        .is_none());
    assert!(engine
        .apply(Command::Dispose, None)
        .unwrap()
        .result
        .is_none());
}
#[test]
fn local_result_rejects_double_commit_lifecycle_and_reentrant_overwrite() {
    for source in [
        "NeuroSurface.commitResult({}); NeuroSurface.define({mount(){}});",
        "NeuroSurface.define({mount(){NeuroSurface.commitResult({});NeuroSurface.commitResult({});}});",
        "NeuroSurface.define({mount(){NeuroSurface.commitResult({toJSON(){NeuroSurface.commitResult({});return {};}});}});",
    ] {
        assert!(Engine::new().unwrap().apply(mount(source), None).is_err());
    }
    let mut engine = Engine::new().unwrap();
    engine
        .apply(
            mount("NeuroSurface.define({mount(){},suspend(){NeuroSurface.commitResult({});}});"),
            None,
        )
        .unwrap();
    assert!(engine.apply(Command::Suspend, None).is_err());
}
#[test]
fn local_result_preserves_canvas_reference_but_never_encodes_a_path() {
    let mut engine = Engine::new().unwrap();
    let output = engine
        .apply(
            mount(
                r#"
      NeuroSurface.define({mount({root}){
        const canvas=document.createElement('canvas');canvas.id='result';
        canvas.width=2;canvas.height=1;root.appendChild(canvas);
        canvas.getContext('2d').fillRect(0,0,2,1);
        NeuroSurface.commitResult({image:{kind:'canvas',nodeId:'result'}});
      }});
    "#,
            ),
            None,
        )
        .unwrap();
    output.result.as_ref().unwrap().validate().unwrap();
    let scene = wire::canvas::scene(&output.scene).unwrap();
    assert_eq!(
        wire::canvas::images(&scene, output.canvases).unwrap().len(),
        1
    );
}

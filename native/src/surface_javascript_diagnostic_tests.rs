//! 原 Engine 的固定阶段/隐私回归；不读取 JS 异常属性，不改变原失败与期限。
use super::*;
use crate::{
    surface_javascript::fixture, surface_javascript_wire::diagnostics::Stage,
    surface_viewport::Viewport,
};
fn mount(engine: &mut Engine, source: &str) -> io::Result<Output> {
    engine.apply(
        Command::Mount {
            source: source.into(),
            snapshot: fixture::snapshot(3, 42),
            resources: fixture::resources(),
        },
        Some(Viewport::new([200, 80], 96).unwrap()),
    )
}
#[test]
fn javascript_diagnostic_engine_exception_keeps_fixed_error_and_no_private_text() {
    let mut engine = Engine::new().unwrap();
    let error = mount(
        &mut engine,
        r#"NeuroSurface.define({mount(){
        const e=new Error('PRIVATE_DIAGNOSTIC_MARKER');
        Object.defineProperty(e,'stack',{get(){throw Error('PRIVATE_STACK_GETTER')}}); throw e;
    }});"#,
    )
    .err()
    .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "Invalid JavaScript Surface operation");
    let report = engine.diagnostic();
    assert_eq!(report.engine.last().unwrap().stage, Stage::Apply);
    assert!(!report.canvas_failed);
    let encoded = serde_json::to_string(&report).unwrap();
    assert!(!encoded.contains("PRIVATE"));
    assert!(!encoded.contains("stack"));
}
#[test]
fn javascript_diagnostic_engine_timeout_uses_original_interrupt_and_phase() {
    for (source, stage) in [
        ("while(true){}", Stage::Module),
        (
            "NeuroSurface.define({mount(){while(true){}}});",
            Stage::Apply,
        ),
    ] {
        let mut engine = Engine::new().unwrap();
        assert_eq!(
            mount(&mut engine, source).err().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
        let report = engine.diagnostic();
        assert!(report.engine_expired);
        assert_eq!(report.engine.last().unwrap().stage, stage);
        assert!(!report.canvas_failed);
    }
}
#[test]
fn javascript_diagnostic_canvas_success_records_real_snapshot_and_sticky_failure() {
    let mut engine = Engine::new().unwrap();
    let result = mount(
        &mut engine,
        include_str!("../tests/fixtures/javascript-canvas.js"),
    );
    // 显式运行此用例时保留成功/失败双方阶段；只输出固定枚举和单调时间。
    wire::diagnostics::log(Some(&engine.diagnostic()));
    let output = result.unwrap();
    assert_eq!(output.canvases.len(), 1);
    assert!(!output.canvases[0].png.is_empty());
    let report = engine.diagnostic();
    assert_eq!(report.engine.last().unwrap().stage, Stage::Complete);
    for expected in [
        Stage::CanvasRendererReady,
        Stage::CanvasBacking,
        Stage::CanvasBitmapReady,
        Stage::CanvasTargetReady,
        Stage::CanvasBindReady,
        Stage::CanvasInitialEndDraw,
        Stage::CanvasBackingReady,
        Stage::CanvasImagesReady,
        Stage::CanvasCommandsBegin,
        Stage::CanvasCommandsReady,
        Stage::CanvasEndDrawReady,
        Stage::CanvasDrawReturned,
        Stage::CanvasSubmitComplete,
        Stage::CanvasSnapshotComplete,
    ] {
        assert!(
            report.canvas.iter().any(|sample| sample.stage == expected),
            "{expected:?}"
        );
    }
    assert!(!report.canvas_failed);
    // 沿原 Store 注入不存在的背板；begin 不得清除粘性失败。
    let mut unknown = [wire::canvas::CanvasFrame {
        id: "unknown".into(),
        width: 1,
        height: 1,
        png: String::new(),
    }];
    assert!(engine.canvas.borrow_mut().snapshots(&mut unknown).is_err());
    assert!(engine
        .apply(
            Command::Update {
                snapshot: fixture::snapshot(4, 43),
                resources: fixture::resources()
            },
            Some(Viewport::new([200, 80], 96).unwrap())
        )
        .is_err());
    let report = engine.diagnostic();
    assert!(report.canvas_failed && !report.canvas_valid);
    assert!(report
        .canvas
        .iter()
        .any(|sample| sample.stage == Stage::CanvasSubmitFailed));
    assert!(!report
        .engine
        .iter()
        .any(|sample| sample.stage == Stage::Complete));
}

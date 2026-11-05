//! 自身 src 回退的应用准入门；复用原图输入/注册链，不建立替代执行实现。
use super::*;
fn inputs(
    app: &Application,
    shader: bool,
    manual: bool,
) -> Option<(Vec<String>, Vec<crate::desktop::loom_input_job::Source>)> {
    let selected = app.documents[0].state.selected_art.borrow();
    let selected = selected.as_ref().unwrap();
    let cap = selected.catalog.handshake.capability(0).unwrap();
    Application::image_inputs(
        &app.documents,
        &app.live_sessions,
        &app.workspace,
        &app.documents[0].state.unit_id,
        manual,
        shader.then_some(&cap),
        None,
    )
    .unwrap()
    .map(|inputs| (inputs.ports, inputs.sources))
}
pub(super) unsafe fn guards(app: &mut Application) {
    assert_eq!(
        inputs(app, false, false).unwrap().0,
        ["reference"],
        "ordinary Art must not acquire own-src fallback"
    );
    assert_eq!(inputs(app, true, false).unwrap().0, ["reference", "input"]);
    let reference = app
        .workspace
        .links()
        .iter()
        .find(|link| link.to_port == "reference")
        .unwrap()
        .clone();
    assert!(app.workspace.remove_link(&reference.id));
    assert!(
        inputs(app, true, false).is_none(),
        "own input must not waive missing reference"
    );
    app.workspace.connect(reference).unwrap();
    let (base, editor) = {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        (canvas.image.take(), std::mem::take(&mut canvas.editor))
    };
    // 与 session_states 的 preview-only 恢复一致：Canvas.image 有画面但 editor 没有 src。
    app.documents[0].state.canvas.borrow_mut().image = Some(Image {
        width: 1,
        height: 1,
        pixels: vec![240, 240, 240, 255].into(),
    });
    assert_eq!(
        inputs(app, true, true).unwrap().0,
        ["reference"],
        "display-only preview is not an input"
    );
    crate::desktop::execution_start::request(app.documents[0].window.0, &app.documents[0].state)
        .unwrap();
    app.start_requested_preparations(false);
    assert!(app.input_jobs.is_empty());
    assert!(app.executions.is_idle());
    assert!(app.documents[0]
        .state
        .preparation_error
        .borrow()
        .as_ref()
        .unwrap()
        .contains("主图像输入"));
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas.image = base;
        canvas.editor = editor;
    }
    prepare(app);
    // 同像素/版本的新文档身份也必须使已准备的自身输入失效，且不连接网络。
    let replacement = app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .export_snapshot();
    app.documents[0].state.canvas.borrow_mut().editor = replacement;
    app.pending_windows(false).unwrap();
    assert!(app.documents[0].state.pending_execution.borrow().is_none());
    assert!(app.execution_jobs.is_empty());
    assert!(app.executions.is_idle());
}

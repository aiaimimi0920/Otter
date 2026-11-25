//! Application admission gate for own-src fallback; reuse original image-input/registration paths without an alternative execution implementation.
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
    // Match session_states preview-only restoration: Canvas.image has a picture but editor has no src.
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
        .contains(crate::ui_text::text("app-shader-local-inputs-failed")));
    {
        let mut canvas = app.documents[0].state.canvas.borrow_mut();
        canvas.image = base;
        canvas.editor = editor;
    }
    prepare(app);
    // A new document identity with identical pixels/version must still invalidate prepared self-input, without network connection.
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

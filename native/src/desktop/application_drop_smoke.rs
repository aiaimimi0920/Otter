//! 向自有隐藏 HWND 投递 Shell 文件包，验证应用集合接收新钉图而非替换源文档。
use super::*;
use crate::desktop::{file_drop_smoke, open_job};

pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    let fixture = smoke::image_fixture::ImageFile::new()?;
    let path = fixture.path().to_owned();
    let count = app.documents.len();
    // 可见流程还会登记工作区附属窗口；回滚必须保留原清单，不能假定每图仅两个 HWND。
    let capture_windows = app.capture_windows.borrow().clone();
    let prepare = || super::super::open_image::PreparedDocument::open(&path, true);
    let mut staged_hwnd = HWND::default();
    let mut staged_panel = HWND::default();
    let mut calls = 0;
    let result = app.add_prepared_with(
        vec![prepare()?, prepare()?],
        false,
        |app, state, observe, _frame| {
            calls += 1;
            if calls == 2 {
                return Err(Error::new(E_FAIL, "fixture second window failure"));
            }
            let document = app.prepare_window(state, observe)?;
            staged_hwnd = document.window.0;
            staged_panel = document.panel.0;
            Ok(document)
        },
    );
    if result.is_ok()
        || app.documents.len() != count
        || IsWindow(Some(staged_hwnd)).as_bool()
        || IsWindow(Some(staged_panel)).as_bool()
        || *app.capture_windows.borrow() != capture_windows
        || app.workspace.units().len() != count
    {
        return Err(Error::new(E_FAIL, "Failed import retained partial windows"));
    }
    let source = &app.documents[0];
    let stamp = source.state.canvas.borrow().editor.document_stamp();
    file_drop_smoke::send_many(source.window.0, &[&path, &path])?;
    if source.state.open_job.borrow().is_none() {
        return Err(Error::new(E_FAIL, "Shell drop did not start worker"));
    }
    open_job::finish(source.window.0, &source.state, true);
    if !stamp.matches(&source.state.canvas.borrow().editor)
        || source.state.new_pin.borrow().is_none()
    {
        return Err(Error::new(E_FAIL, "Shell drop changed source or lost pin"));
    }
    app.pending_windows(false)?;
    if app.documents.len() != count + 2 {
        return Err(Error::new(
            E_FAIL,
            "Shell drop did not create independent document",
        ));
    }
    for pin in &app.documents[count..] {
        let canvas = pin.state.canvas.borrow();
        if !canvas.pinned
            || !canvas.editor.elements().is_empty()
            || canvas
                .image
                .as_ref()
                .map(|image| (image.width, image.height))
                != Some((32, 32))
        {
            return Err(Error::new(E_FAIL, "Shell drop pin has wrong content"));
        }
        drop(canvas);
        SendMessageW(pin.window.0, WM_CLOSE, None, None);
    }
    app.collect_closed();
    if app.documents.len() != count
        || !stamp.matches(&app.documents[0].state.canvas.borrow().editor)
    {
        return Err(Error::new(E_FAIL, "Closing dropped pin damaged source"));
    }
    println!("native-drop-pin-smoke: shell=true worker=true new-window=true source-preserved=true close-independent=true system-input=false");
    println!(
        "native-drop-batch-smoke: count=2 one-worker=true source-preserved=true system-input=false"
    );
    println!("native-import-rollback-smoke: staged-host-released=true staged-panel-released=true collection-unchanged=true");
    println!(
        "native-import-rollback-capture-smoke: documents={count} capture-windows={} unchanged=true",
        capture_windows.len()
    );
    Ok(())
}

//! 注入文件选择结果，验证原生打开链；不显示系统对话框或访问用户文件。
use super::*;
use crate::desktop::{open_image, open_job};

pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    let source = &app.documents[0];
    let hwnd = source.window.0;
    let state = &source.state;
    let stamp = state.canvas.borrow().editor.document_stamp();
    let fixture = smoke::image_fixture::ImageFile::new()?;
    let path = fixture.path().to_owned();
    open_image::open_with(hwnd, state, true, || Ok(None))?;
    if open_image::open_with(hwnd, state, true, || {
        Err(Error::new(E_FAIL, "fixture picker failure"))
    })
    .is_ok()
        || state.open_job.borrow().is_some()
        || !stamp.matches(&state.canvas.borrow().editor)
    {
        return Err(Error::new(
            E_FAIL,
            "Cancelled or failed picker changed source",
        ));
    }
    open_image::open_with(hwnd, state, true, || {
        Ok(Some(vec![path.clone(), path.clone()]))
    })?;
    open_job::finish(hwnd, state, true);
    if !stamp.matches(&state.canvas.borrow().editor) || state.new_pin.borrow().is_none() {
        return Err(Error::new(
            E_FAIL,
            "Open replaced source instead of preparing pin",
        ));
    }
    let count = app.documents.len();
    app.pending_windows(false)?;
    if app.documents.len() != count + 2 {
        return Err(Error::new(
            E_FAIL,
            "Open failed to create independent window",
        ));
    }
    for pin in &app.documents[count..] {
        let canvas = pin.state.canvas.borrow();
        if !canvas.pinned
            || canvas
                .image
                .as_ref()
                .map(|image| (image.width, image.height))
                != Some((32, 32))
        {
            return Err(Error::new(E_FAIL, "Open pin image mismatch"));
        }
        drop(canvas);
        SendMessageW(pin.window.0, WM_CLOSE, None, None);
    }
    app.collect_closed();
    let source = &app.documents[0];
    let state = &source.state;
    // 模拟选择器的嵌套消息改变源文档，不能继续交付旧选择。
    if open_image::open_with(source.window.0, state, true, || {
        state
            .canvas
            .borrow_mut()
            .replace_image(super::super::image::Image::open(&path)?)?;
        Ok(Some(vec![path]))
    })
    .is_ok()
        || state.open_job.borrow().is_some()
        || state.new_pin.borrow().is_some()
    {
        return Err(Error::new(E_FAIL, "Stale picker result started a worker"));
    }
    println!("native-open-pin-smoke: worker=true new-window=true source-preserved=true cancelled=true failed=true stale-rejected=true dialog-access=false");
    println!("native-open-batch-smoke: count=2 source-preserved=true dialog-access=false");
    Ok(())
}

//! 注入截图像素，通过真实后台准备/定时器交付创建钉图，不读取桌面或丢弃源文档。
use super::*;
use crate::{desktop::image::Image, editor::Tool, viewport::Point};

fn fixture_pixels(value: u8, count: usize) -> Result<crate::pixels::Pixels> {
    let lease = crate::pixels::reserve(count)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Fixture pixel budget exhausted"))?;
    Ok(crate::pixels::Pixels::from_reserved(
        vec![value; count],
        lease,
    ))
}

pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    let source = &app.documents[0];
    {
        let mut canvas = source.state.canvas.borrow_mut();
        canvas.replace_image(Image {
            width: 8,
            height: 8,
            pixels: fixture_pixels(255, 256)?,
        })?;
        canvas.select_tool(Tool::Brush);
        let origin = canvas.viewport.origin;
        canvas.pointer_down(Point {
            x: origin.x + 1.0,
            y: origin.y + 1.0,
        });
        canvas.pointer_up(Point {
            x: origin.x + 5.0,
            y: origin.y + 5.0,
        });
    }
    let stamp = source.state.canvas.borrow().editor.document_stamp();
    let pixels = fixture_pixels(128, 64)?;
    let captured = Image {
        width: 4,
        height: 4,
        pixels: pixels.clone(),
    };
    super::super::capture_input::run_positioned(source.window.0, &source.state, true, || {
        Ok(Some((captured, Some(POINT { x: -140, y: 75 }))))
    })?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while source.state.open_job.borrow().is_some() {
        if std::time::Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "New pin preparation timed out"));
        }
        SendMessageW(
            source.window.0,
            WM_TIMER,
            Some(WPARAM(super::super::open_job::OPEN_TIMER)),
            None,
        );
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    if !stamp.matches(&source.state.canvas.borrow().editor)
        || source.state.new_pin.borrow().is_none()
    {
        return Err(Error::new(
            E_FAIL,
            "Pin preparation replaced source or lost delivery",
        ));
    }
    if super::super::tool_panel_availability::Availability::capture(&source.state)
        .is_some_and(|available| available.allows(super::super::capture_input::DEFAULT))
    {
        return Err(Error::new(
            E_FAIL,
            "Capture remained enabled while a pin awaited window creation",
        ));
    }
    app.pending_windows(false)?;
    if !super::super::tool_panel_availability::Availability::capture(&app.documents[0].state)
        .is_some_and(|available| available.allows(super::super::capture_input::DEFAULT))
    {
        return Err(Error::new(
            E_FAIL,
            "Capture did not recover after pin delivery",
        ));
    }
    if app.documents.len() != 3 {
        return Err(Error::new(
            E_FAIL,
            "Prepared pin did not create an independent window",
        ));
    }
    let pin = &app.documents[2];
    let mut bounds = RECT::default();
    GetWindowRect(pin.window.0, &mut bounds)?;
    if bounds.left != -140
        || bounds.top != 75
        || bounds.right - bounds.left != 4
        || bounds.bottom - bounds.top != 4
    {
        return Err(Error::new(
            E_FAIL,
            "Captured pin lost physical screen placement",
        ));
    }
    let canvas = pin.state.canvas.borrow();
    if !canvas.pinned
        || !canvas
            .image
            .as_ref()
            .is_some_and(|image| image.pixels.same_storage(&pixels))
        || !canvas.editor.elements().is_empty()
        || !stamp.matches(&app.documents[0].state.canvas.borrow().editor)
    {
        return Err(Error::new(
            E_FAIL,
            "New pin lost pixels or shared editing state",
        ));
    }
    drop(canvas);
    let mut original = app.documents[0].state.canvas.borrow_mut();
    if original.editor.elements().len() != 1 || !original.editor.undo() || !original.editor.redo() {
        return Err(Error::new(
            E_FAIL,
            "Original document history did not survive new pin creation",
        ));
    }
    drop(original);
    SendMessageW(pin.window.0, WM_CLOSE, None, None);
    app.collect_closed();
    if app.documents.len() != 2 {
        return Err(Error::new(
            E_FAIL,
            "Closing new pin removed original documents",
        ));
    }
    println!("native-new-pin-smoke: worker=true new-window=true shared-pixels=true source-history=true desktop-access=false");
    println!(
        "native-pin-origin-smoke: negative-origin=true physical-size=true desktop-access=false"
    );
    Ok(())
}

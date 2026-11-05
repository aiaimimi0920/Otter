//! 真实 HWND 验证裁剪效果后台交付；内存夹具与窗口消息，不写用户数据或注入系统输入。
use super::State;
use crate::{
    blur_image::BlurImage,
    editor::{Editor, Tool},
    viewport::Point,
};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

fn require(value: bool, message: &'static str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::new(E_FAIL, message))
    }
}
unsafe fn rect(hwnd: HWND) -> Result<RECT> {
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect)?;
    Ok(rect)
}
fn same(a: RECT, b: RECT) -> bool {
    a.left == b.left && a.top == b.top && a.right == b.right && a.bottom == b.bottom
}
unsafe fn finish(hwnd: HWND, state: &State) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while state.crop_job.borrow().is_some() || state.flip_job.borrow().is_busy() {
        require(
            std::time::Instant::now() < deadline,
            "Crop effect timer timeout",
        )?;
        require(IsWindow(Some(hwnd)).as_bool(), "Crop effect window closed")?;
        let mut message = MSG::default();
        while PeekMessageW(&mut message, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if let Some(error) = state.error.borrow_mut().take() {
            return Err(error);
        }
        crate::desktop::smoke_timing::sleep(std::time::Duration::from_millis(1));
    }
    Ok(())
}
unsafe fn drag(hwnd: HWND, state: &State) -> Result<()> {
    let (left, top, width, height) = {
        let canvas = state.canvas.borrow();
        let image = canvas.image.as_ref().unwrap();
        let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(1) as f32 / 96.0;
        let view = canvas.viewport;
        (
            view.origin.x * dpi,
            view.origin.y * dpi,
            image.width as f32 * view.scale * dpi,
            image.height as f32 * view.scale * dpi,
        )
    };
    require(
        width >= 8.0 && height >= 8.0,
        "Crop effect fixture too small",
    )?;
    let point =
        |x: i32, y: i32| LPARAM(((x as u32 & 0xffff) | ((y as u32 & 0xffff) << 16)) as isize);
    SendMessageW(
        hwnd,
        WM_LBUTTONDOWN,
        Some(WPARAM(0)),
        Some(point(
            (left + width / 4.0).round() as i32,
            (top + height / 4.0).round() as i32,
        )),
    );
    SendMessageW(
        hwnd,
        WM_MOUSEMOVE,
        Some(WPARAM(0)),
        Some(point(
            (left + width * 0.75).round() as i32,
            (top + height * 0.75).round() as i32,
        )),
    );
    SendMessageW(
        hwnd,
        WM_LBUTTONUP,
        Some(WPARAM(0)),
        Some(point(
            (left + width * 0.75).round() as i32,
            (top + height * 0.75).round() as i32,
        )),
    );
    Ok(())
}
pub(super) unsafe fn run(hwnd: HWND, state: &State) -> Result<()> {
    let original_rect = rect(hwnd)?;
    let saved = {
        let mut canvas = state.canvas.borrow_mut();
        let mut layers = canvas.editor.layers().unwrap().clone();
        layers.annotations = Some(layers.base.clone());
        let mut editor = Editor::with_layers(layers.clone())
            .map_err(|_| Error::new(E_FAIL, "Crop fixture invalid"))?;
        let blurred =
            BlurImage::with_strength(layers.size.0, layers.size.1, layers.base.clone(), 2.4)
                .ok_or(Error::new(E_FAIL, "Crop fixture blur invalid"))?;
        editor
            .begin_blur(
                Point {
                    x: layers.size.0 as f32 / 2.0,
                    y: layers.size.1 as f32 / 2.0,
                },
                blurred,
            )
            .map_err(|_| Error::new(E_FAIL, "Crop fixture mark invalid"))?;
        editor.commit();
        canvas.select_tool(Tool::Select);
        canvas.crop_mode = true;
        std::mem::replace(&mut canvas.editor, editor)
    };
    let revision = state.canvas.borrow().editor.revision();
    let old_blur = state.canvas.borrow().editor.elements()[0]
        .blur
        .as_ref()
        .unwrap()
        .clone();
    drag(hwnd, state)?;
    require(
        state.crop_job.borrow().is_some(),
        "Crop effect worker not started",
    )?;
    require(
        state.canvas.borrow().editor.revision() == revision
            && state.canvas.borrow().editor.crop_view().is_none(),
        "Crop effect committed before timer",
    )?;
    require(
        same(rect(hwnd)?, original_rect),
        "Crop effect resized before completion",
    )?;
    finish(hwnd, state)?;
    let crop = state.canvas.borrow().editor.crop_view();
    require(crop.is_some(), "Crop effect view not committed")?;
    require(
        state.canvas.borrow().editor.revision() == revision + 1,
        "Crop effect history mismatch",
    )?;
    require(
        !state.canvas.borrow().editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(old_blur.pixels()),
        "Crop effect blur not refreshed",
    )?;
    if state.canvas.borrow().pinned {
        let cropped = rect(hwnd)?;
        require(
            cropped.right - cropped.left < original_rect.right - original_rect.left
                && cropped.bottom - cropped.top < original_rect.bottom - original_rect.top,
            "Crop effect pin did not shrink",
        )?;
    } else {
        require(
            same(rect(hwnd)?, original_rect),
            "Ordinary crop resized outer window",
        )?;
    }
    SendMessageW(hwnd, WM_PAINT, None, None);
    let flip_revision = state.canvas.borrow().editor.revision();
    super::flip_input::start(hwnd, state, super::flip_input::HORIZONTAL);
    require(
        state.flip_job.borrow().is_busy(),
        "Cropped overlay flip did not start",
    )?;
    finish(hwnd, state)?;
    {
        let canvas = state.canvas.borrow();
        let frame = crop.unwrap().source.rect;
        require(
            canvas.editor.revision() == flip_revision + 1 && canvas.editor.crop_view() == crop,
            "Cropped flip did not commit atomically",
        )?;
        require(
            canvas.editor.layers().unwrap().annotation_dimensions()
                == (
                    frame.width.round().max(1.0) as u32,
                    frame.height.round().max(1.0) as u32,
                ),
            "Cropped flip texture dimensions mismatch",
        )?;
    }
    SendMessageW(hwnd, WM_PAINT, None, None);
    let cropped_rect = rect(hwnd)?;
    super::crop_input::command(hwnd, state, super::crop_input::RESET);
    require(
        state.crop_job.borrow().is_some(),
        "Crop reset worker not started",
    )?;
    require(
        state.canvas.borrow().editor.crop_view() == crop && same(rect(hwnd)?, cropped_rect),
        "Crop reset changed view before timer",
    )?;
    finish(hwnd, state)?;
    require(
        state.canvas.borrow().editor.crop_view().is_none(),
        "Crop reset not committed",
    )?;
    // 重置可以因整数窗口尺寸取整偏差 1 px，不把它误判成事务故障。
    let restored = rect(hwnd)?;
    require(
        (restored.left - original_rect.left).abs() <= 1
            && (restored.top - original_rect.top).abs() <= 1
            && (restored.right - original_rect.right).abs() <= 1
            && (restored.bottom - original_rect.bottom).abs() <= 1,
        "Crop reset window mismatch",
    )?;
    super::crop_input::command(hwnd, state, super::crop_input::START);
    let revision = state.canvas.borrow().editor.revision();
    drag(hwnd, state)?;
    require(
        state.crop_job.borrow().is_some(),
        "Cancelled crop worker not started",
    )?;
    SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1B)), Some(LPARAM(0)));
    finish(hwnd, state)?;
    require(
        state.canvas.borrow().editor.revision() == revision
            && state.canvas.borrow().editor.crop_view().is_none(),
        "Cancelled crop modified document",
    )?;
    require(same(rect(hwnd)?, restored), "Cancelled crop resized window")?;
    {
        let mut canvas = state.canvas.borrow_mut();
        canvas.editor = saved;
        canvas.select_tool(Tool::Pan);
        canvas.sync_raster_source();
    }
    super::hit_region::update(hwnd, state)?;
    println!(
        "native-crop-effect-smoke: timer=true atomic-view=true reset=true escape=true window=true flip=true"
    );
    Ok(())
}

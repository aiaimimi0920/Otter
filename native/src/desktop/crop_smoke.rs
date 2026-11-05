//! 仅 --smoke-test 调用的隐藏 HWND 验证；不注入系统输入、不读取剪贴板或用户配置。
use super::State;
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
};

fn require(condition: bool, message: &'static str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::new(E_FAIL, message))
    }
}
unsafe fn window_rect(hwnd: HWND) -> Result<RECT> {
    let mut rect = RECT::default();
    GetWindowRect(hwnd, &mut rect)?;
    Ok(rect)
}
fn close(a: RECT, b: RECT) -> bool {
    (a.left - b.left).abs() <= 1
        && (a.top - b.top).abs() <= 1
        && (a.right - b.right).abs() <= 1
        && (a.bottom - b.bottom).abs() <= 1
}

pub(super) unsafe fn run(hwnd: HWND, state: &State) -> Result<()> {
    let original = window_rect(hwnd)?;
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client)?;
    require(
        client.right >= 8 && client.bottom >= 8,
        "Crop smoke requires an image at least 8 pixels across",
    )?;
    let source = state.canvas.borrow().editor.layers().unwrap().clone();
    let revision = state.canvas.borrow().editor.revision();
    super::crop_input::command(hwnd, state, super::crop_input::START);
    let point =
        |x: i32, y: i32| LPARAM(((x as u32 & 0xffff) | ((y as u32 & 0xffff) << 16)) as isize);
    let start = (client.right / 4, client.bottom / 4);
    let end = (client.right * 3 / 4, client.bottom * 3 / 4);
    SendMessageW(
        hwnd,
        WM_LBUTTONDOWN,
        Some(WPARAM(0)),
        Some(point(start.0, start.1)),
    );
    SendMessageW(
        hwnd,
        WM_MOUSEMOVE,
        Some(WPARAM(0)),
        Some(point(end.0, end.1)),
    );
    require(
        state.canvas.borrow().editor.revision() == revision,
        "Crop drag changed committed document",
    )?;
    SendMessageW(
        hwnd,
        WM_LBUTTONUP,
        Some(WPARAM(0)),
        Some(point(end.0, end.1)),
    );
    require(
        state.canvas.borrow().editor.revision() == revision + 1,
        "Crop pointer messages did not commit once",
    )?;
    let cropped = window_rect(hwnd)?;
    let expected = RECT {
        left: original.left + start.0,
        top: original.top + start.1,
        right: original.left + end.0,
        bottom: original.top + end.1,
    };
    require(
        close(cropped, expected),
        "Cropped HWND has wrong position or dimensions",
    )?;
    SendMessageW(hwnd, WM_PAINT, None, None);
    require(
        state
            .canvas
            .borrow()
            .editor
            .layers()
            .unwrap()
            .same_storage(&source),
        "Crop copied or changed source pixels",
    )?;

    // 使用真实 GDI 区域检查源框外的部分没有留在窗口命中区域中。
    state
        .canvas
        .borrow_mut()
        .select_tool(crate::editor::Tool::Pan);
    super::hit_region::update(hwnd, state)?;
    let region = CreateRectRgn(0, 0, 0, 0);
    if region.is_invalid() {
        return Err(Error::from_win32());
    }
    let kind = GetWindowRgn(hwnd, region);
    let mut bounds = RECT::default();
    let bounds_kind = GetRgnBox(region, &mut bounds);
    let _ = DeleteObject(region.into());
    require(
        kind != RGN_ERROR && bounds_kind != RGN_ERROR,
        "Crop HWND region query failed",
    )?;
    require(
        bounds.left >= 0
            && bounds.top >= 0
            && bounds.right <= cropped.right - cropped.left
            && bounds.bottom <= cropped.bottom - cropped.top,
        "Crop HWND hit region escapes client",
    )?;

    require(
        state.canvas.borrow_mut().editor.undo(),
        "Crop undo unavailable",
    )?;
    super::crop_window::sync(hwnd, state)?;
    require(
        close(window_rect(hwnd)?, original),
        "Crop undo did not restore HWND",
    )?;
    require(
        state.canvas.borrow_mut().editor.redo(),
        "Crop redo unavailable",
    )?;
    super::crop_window::sync(hwnd, state)?;
    require(
        close(window_rect(hwnd)?, cropped),
        "Crop redo did not restore cropped HWND",
    )?;
    super::crop_input::command(hwnd, state, super::crop_input::RESET);
    super::crop_window::sync(hwnd, state)?;
    require(
        close(window_rect(hwnd)?, original),
        "Crop reset did not restore HWND",
    )?;
    SendMessageW(hwnd, WM_PAINT, None, None);
    if let Some(error) = state.error.borrow_mut().take() {
        return Err(error);
    }
    println!(
        "native-crop-smoke: pointer=true window=true undo=true redo=true reset=true region=true"
    );
    Ok(())
}

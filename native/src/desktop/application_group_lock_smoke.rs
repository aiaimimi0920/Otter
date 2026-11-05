//! 分组锁定的真实 HWND 输入门禁；使用内嵌图片及窗口消息，不注入系统键鼠。
use super::*;
use crate::desktop::{context_menu, group_menu, image::Image, open_job};
use windows::Win32::UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::VK_DELETE};
pub(super) unsafe fn run(app: &mut Application) -> Result<()> {
    let fixture = smoke::image_fixture::ImageFile::new()?;
    let path = fixture.path().to_owned();
    let source = &app.documents[0];
    source
        .state
        .canvas
        .borrow_mut()
        .replace_image(Image::open(&path)?)?;
    source
        .state
        .canvas
        .borrow_mut()
        .select_tool(crate::editor::Tool::Brush);
    open_job::start_destination(
        source.window.0,
        &source.state,
        open_job::Input::File(path),
        false,
    )?;
    context_menu::execute(source.window.0, &source.state, group_menu::LOCK)?;
    if app.pending_groups(false).is_ok() || app.documents[0].state.group_locked.get() {
        return Err(Error::new(
            E_FAIL,
            "Busy editing task did not reject group lock",
        ));
    }
    let source = &app.documents[0];
    open_job::cancel(&source.state);
    open_job::finish(source.window.0, &source.state, true);
    context_menu::execute(source.window.0, &source.state, group_menu::LOCK)?;
    app.pending_groups(false)?;
    if !app.documents[0].state.group_locked.get()
        || !app.documents[1].state.group_locked.get()
        || app.documents[2].state.group_locked.get()
    {
        return Err(Error::new(E_FAIL, "Group lock applied to wrong windows"));
    }
    let source = &app.documents[0];
    let state = &source.state;
    let stamp = state.canvas.borrow().editor.document_stamp();
    let origin = state.canvas.borrow().viewport.origin;
    let dpi = GetDpiForWindow(source.window.0).max(1) as f32 / 96.0;
    let point = |offset: f32| {
        let x = ((origin.x + offset) * dpi) as i32;
        let y = ((origin.y + offset) * dpi) as i32;
        LPARAM(((x as u32 & 0xffff) | ((y as u32 & 0xffff) << 16)) as isize)
    };
    for message in [WM_LBUTTONDOWN, WM_LBUTTONUP] {
        SendMessageW(source.window.0, message, None, Some(point(2.0)));
    }
    SendMessageW(
        source.window.0,
        WM_KEYDOWN,
        Some(WPARAM(VK_DELETE.0 as usize)),
        None,
    );
    context_menu::execute(source.window.0, state, context_menu::APPLY_STYLE)?;
    if !stamp.matches(&state.canvas.borrow().editor)
        || state.canvas.borrow().editor.draft().is_some()
    {
        return Err(Error::new(E_FAIL, "Locked input modified document"));
    }
    context_menu::execute(source.window.0, state, group_menu::LOCK)?;
    app.pending_groups(false)?;
    let source = &app.documents[0];
    SendMessageW(source.window.0, WM_LBUTTONDOWN, None, Some(point(2.0)));
    SendMessageW(source.window.0, WM_LBUTTONUP, None, Some(point(8.0)));
    if app.documents[0].state.group_locked.get()
        || app.documents[1].state.group_locked.get()
        || stamp.matches(&source.state.canvas.borrow().editor)
    {
        return Err(Error::new(
            E_FAIL,
            "Unlocked document did not resume editing",
        ));
    }
    println!("native-group-lock-smoke: busy-rejected=true members-only=true pointer-blocked=true keyboard-blocked=true unlocked-edit=true system-input=false");
    Ok(())
}

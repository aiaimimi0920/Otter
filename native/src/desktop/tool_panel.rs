//! 按需重绘的 Win32 工具面板；owner 销毁时系统销毁其窗口与所有按钮。
use super::tool_panel_availability::Availability;
use super::{State, Window};
use windows::Win32::UI::Controls::DRAWITEMSTRUCT;
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
#[path = "tool_panel_children.rs"]
mod children;
#[path = "tool_panel_content.rs"]
mod content;
pub(super) use content::prepare;
#[cfg(test)]
#[path = "tool_panel_content_tests.rs"]
mod content_tests;
#[path = "tool_panel_draw.rs"]
mod draw;
#[path = "tool_panel_font.rs"]
pub(super) mod font;
#[path = "tool_panel_layout.rs"]
mod geometry;
#[path = "tool_panel_keyboard.rs"]
mod keyboard;
#[cfg(test)]
#[path = "tool_panel_layout_tests.rs"]
mod layout_tests;
#[path = "tool_panel_position.rs"]
mod position;
#[path = "tool_panel_scroll.rs"]
mod scroll;
use children::layout;
const EXECUTE: u32 = WM_APP + 90;
fn entries() -> impl Iterator<Item = (usize, &'static str)> {
    super::tool_commands::TOOLS
        .iter()
        .map(|entry| (entry.id, entry.label))
        .chain(super::tool_panel_actions::ACTIONS.iter().copied())
}

pub(super) unsafe fn translate(state: &State, message: &MSG) -> bool {
    let Some(owner) = keyboard::owner(state) else {
        return false;
    };
    let panel = state.tool_panel.get();
    if !IsWindow(Some(panel)).as_bool()
        || !(message.hwnd == panel || IsChild(panel, message.hwnd).as_bool())
    {
        return false;
    }
    refresh(owner);
    if keyboard::translate(state, message) {
        return true;
    }
    let handled = IsDialogMessageW(panel, message).as_bool();
    if handled {
        scroll::reveal_focus(panel);
    }
    handled
}

pub(super) unsafe fn smoke(owner: HWND, state: &State) -> Result<()> {
    prepare(state.tool_panel.get(), state)?;
    scroll::smoke(state.tool_panel.get())?;
    effect_controls_smoke(owner, state)?;
    let panel = state.tool_panel.get();
    for (id, _) in entries() {
        GetDlgItem(Some(panel), id as i32)?;
    }
    let old_tool = state.canvas.borrow().tool;
    let button = GetDlgItem(Some(panel), 0x53)?;
    let _ = SendMessageW(button, BM_CLICK, None, None);
    let mut message = MSG::default();
    if !PeekMessageW(&mut message, Some(owner), EXECUTE, EXECUTE, PM_REMOVE).as_bool() {
        return Err(windows::core::Error::new(
            E_FAIL,
            "Tool panel did not route click",
        ));
    }
    DispatchMessageW(&message);
    if state.canvas.borrow().tool != crate::editor::Tool::Select {
        return Err(windows::core::Error::new(
            E_FAIL,
            "Tool panel did not select tool",
        ));
    }
    let _ = SendMessageW(button, WM_PAINT, None, None);
    let before_style = state.canvas.borrow().editor.style();
    let has_image = state.canvas.borrow().image.is_some();
    if has_image {
        for command in [super::context_menu::WIDTH_UP, 0x4F, 0x46] {
            let action = GetDlgItem(Some(panel), command as i32)?;
            let _ = SendMessageW(action, BM_CLICK, None, None);
            if !PeekMessageW(&mut message, Some(owner), EXECUTE, EXECUTE, PM_REMOVE).as_bool() {
                return Err(windows::core::Error::new(
                    E_FAIL,
                    "Property button did not route click",
                ));
            }
            DispatchMessageW(&message);
        }
        let after_style = state.canvas.borrow().editor.style();
        if after_style.width() != before_style.width() + 1.0
            || after_style.opacity() == before_style.opacity()
            || after_style.fill().is_some() == before_style.fill().is_some()
            || state.tool_panel_style.get() != Some(after_style)
        {
            return Err(windows::core::Error::new(
                E_FAIL,
                "Property buttons did not update real style",
            ));
        }
        if super::tool_panel_actions::label(0x46, state).as_deref() != Some("填充 开") {
            return Err(windows::core::Error::new(
                E_FAIL,
                "Property label did not reflect style",
            ));
        }
        verify_captions(panel, state)?;
    }
    state.file_dialog.set(true);
    refresh(owner);
    let disabled = !IsWindowEnabled(button).as_bool();
    let guarded_style = state.canvas.borrow().editor.style();
    let _ = SendMessageW(
        owner,
        EXECUTE,
        Some(WPARAM(super::context_menu::WIDTH_UP)),
        None,
    );
    let unchanged = state.canvas.borrow().editor.style() == guarded_style;
    state.file_dialog.set(false);
    refresh(owner);
    if !disabled || !unchanged || !IsWindowEnabled(button).as_bool() {
        return Err(windows::core::Error::new(
            E_FAIL,
            "Tool panel busy gate failed",
        ));
    }
    let _ = ValidateRect(Some(panel), None);
    for (id, _) in entries() {
        let _ = ValidateRect(Some(GetDlgItem(Some(panel), id as i32)?), None);
    }
    for _ in 0..8 {
        let _ = SendMessageW(owner, WM_TIMER, Some(WPARAM(0xffff)), None);
    }
    if GetUpdateRect(panel, None, false).as_bool() || GetUpdateRect(button, None, false).as_bool() {
        return Err(windows::core::Error::new(
            E_FAIL,
            "Unchanged timer repainted tool panel",
        ));
    }
    state.canvas.borrow_mut().select_tool(old_tool);
    state.canvas.borrow_mut().editor.set_style(before_style);
    let _ = SendMessageW(owner, WM_NULL, None, None);
    verify_captions(panel, state)?;
    if state.tool_panel_tool.get() != Some(old_tool) {
        return Err(windows::core::Error::new(
            E_FAIL,
            "Tool panel did not observe completed tool change",
        ));
    }
    println!("native-tool-panel-smoke: controls=19 click=true command=true");
    if has_image {
        println!(
        "native-tool-properties-smoke: controls=16 width=true opacity=true fill=true labels=true"
    );
    }
    println!(
        "native-tool-panel-availability-smoke: disabled=true queued-rejected=true recovered=true"
    );
    println!("native-tool-panel-refresh-smoke: unchanged-timers=8 refreshed-after-change=true");
    Ok(())
}

unsafe fn verify_captions(panel: HWND, state: &State) -> Result<()> {
    for &(command, _) in super::tool_panel_actions::ACTIONS {
        let expected = super::tool_panel_actions::label(command, state).unwrap();
        if !super::tool_panel_actions::caption_matches(
            GetDlgItem(Some(panel), command as i32)?,
            &expected,
        ) {
            return Err(windows::core::Error::new(
                E_FAIL,
                "Native button caption is stale",
            ));
        }
    }
    Ok(())
}

unsafe fn effect_controls_smoke(owner: HWND, state: &State) -> Result<()> {
    use crate::editor::Tool;
    let original = state.canvas.borrow().tool;
    let has_image = state.canvas.borrow().image.is_some();
    let result = (|| {
        for tool in [Tool::Blur, Tool::Mosaic, Tool::Select] {
            // 仅检查实际 HWND 可用性刷新，不点击会打开模态输入的按钮，也不启动模糊任务。
            state.canvas.borrow_mut().tool = tool;
            SendMessageW(owner, WM_NULL, None, None);
            for (command, enabled) in [
                (
                    super::blur_default::COMMAND,
                    has_image && tool == Tool::Blur,
                ),
                (
                    super::numeric_style::MOSAIC,
                    has_image && tool == Tool::Mosaic,
                ),
                (super::blur_numeric::COMMAND, false),
            ] {
                let button = GetDlgItem(Some(state.tool_panel.get()), command as i32)?;
                if IsWindowEnabled(button).as_bool() != enabled {
                    return Err(windows::core::Error::new(
                        E_FAIL,
                        "Effect control availability mismatch",
                    ));
                }
            }
        }
        Ok(())
    })();
    state.canvas.borrow_mut().tool = original;
    SendMessageW(owner, WM_NULL, None, None);
    result?;
    println!("native-effect-controls-smoke: high-bits=true tool-gates=true");
    Ok(())
}

pub(super) unsafe fn create(owner: HWND, state: &State, visible: bool) -> Result<Window> {
    let instance = HINSTANCE(GetModuleHandleW(None)?.0);
    let class = w!("OtterNativeToolPanel");
    let descriptor = WNDCLASSW {
        hInstance: instance,
        lpszClassName: class,
        lpfnWndProc: Some(procedure),
        hCursor: LoadCursorW(None, IDC_ARROW)?,
        ..Default::default()
    };
    if RegisterClassW(&descriptor) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
        return Err(windows::core::Error::from_win32());
    }
    let dpi = GetDpiForWindow(owner).max(96);
    let scale = dpi as f32 / 96.0;
    let (width, height) = geometry::client(
        super::tool_commands::TOOLS.len(),
        super::tool_panel_actions::ACTIONS.len(),
        dpi,
    );
    let mut area = RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };
    let style = WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_HSCROLL | WS_VSCROLL;
    AdjustWindowRectExForDpi(&mut area, style, false, WS_EX_TOOLWINDOW, dpi)?;
    // AdjustWindowRectEx 系列不计标准滚动条；显式预留，避免正常工作区也裁掉最后一列。
    area.right += GetSystemMetricsForDpi(SM_CXVSCROLL, dpi);
    area.bottom += GetSystemMetricsForDpi(SM_CYHSCROLL, dpi);
    let mut anchor = RECT::default();
    GetWindowRect(owner, &mut anchor)?;
    let area = super::smoke_timing::run("tool-panel-position", || {
        position::initial(
            anchor,
            area.right - area.left,
            area.bottom - area.top,
            (8.0 * scale).round() as i32,
        )
    })?;
    let panel = super::smoke_timing::run("tool-panel-host", || {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class,
            w!("Otter · 工具（Ctrl+E / F4）"),
            style,
            area.left,
            area.top,
            area.right - area.left,
            area.bottom - area.top,
            Some(owner),
            None,
            Some(instance),
            Some(owner.0.cast()),
        )
    })
    .map(Window)?;
    super::smoke_timing::run("tool-panel-font", || {
        font::update(panel.0, GetDpiForWindow(panel.0))
    })?;
    super::smoke_timing::run("tool-panel-layout", || {
        layout(panel.0);
        Ok::<_, windows::core::Error>(())
    })?;
    state.tool_panel.set(panel.0);
    // 普通工作区保留提前准备合同；隐藏钉图首次使用才创建 BUTTON。
    if visible || !state.canvas.borrow().pinned {
        prepare(panel.0, state)?;
    }
    if visible {
        let _ = ShowWindow(panel.0, SW_SHOWNOACTIVATE);
    }
    Ok(panel)
}
pub(super) unsafe fn invalidate(state: &State) {
    if !content::ready(state.tool_panel.get()) {
        return;
    }
    state.tool_panel_tool.set(Some(state.canvas.borrow().tool));
    state
        .tool_panel_style
        .set(Some(state.canvas.borrow().editor.style()));
    let panel = state.tool_panel.get();
    if IsWindow(Some(panel)).as_bool() {
        if let Err(error) = super::tool_panel_actions::sync_captions(panel, state) {
            eprintln!("native-tool-panel-captions: {error}");
        }
        let _ = RedrawWindow(Some(panel), None, None, RDW_INVALIDATE | RDW_ALLCHILDREN);
    }
}
// 在命令/定时器处理后比较，而非处理前；最后一次异步交付也能立即反映到面板。
pub(super) unsafe fn refresh(owner: HWND) {
    if !IsWindow(Some(owner)).as_bool() {
        return;
    }
    let pointer = GetWindowLongPtrW(owner, GWLP_USERDATA) as *const State;
    if pointer.is_null() {
        return;
    }
    let state = &*pointer;
    if !IsWindow(Some(state.tool_panel.get())).as_bool() || !content::ready(state.tool_panel.get())
    {
        return;
    }
    // 重入消息可能发生在外层 UI 操作尚持有 Canvas 时；由外层返回后的检查补刷新。
    let Ok(canvas) = state.canvas.try_borrow() else {
        return;
    };
    let tool = canvas.tool;
    let style = canvas.editor.style();
    drop(canvas);
    let Some(available) = Availability::capture(state) else {
        return;
    };
    let mask = entries().enumerate().fold(0u64, |mask, (index, (id, _))| {
        mask | ((available.allows(id) as u64) << index)
    });
    if state.tool_panel_enabled.replace(mask) != mask {
        let panel = state.tool_panel.get();
        for (index, (id, _)) in entries().enumerate() {
            if let Ok(button) = GetDlgItem(Some(panel), id as i32) {
                let _ = EnableWindow(button, mask & (1 << index) != 0);
            }
        }
    }
    if state.tool_panel_tool.get() != Some(tool) || state.tool_panel_style.get() != Some(style) {
        invalidate(state);
    }
}
pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if let Some(result) = keyboard::dispatch(hwnd, state, msg, wp, lp) {
        return Some(result);
    }
    if msg == EXECUTE {
        if Availability::capture(state).is_some_and(|available| available.allows(wp.0)) {
            if let Err(error) = super::context_menu::execute(hwnd, state, wp.0) {
                eprintln!("native-tool-panel: {error}");
            }
            invalidate(state);
        }
        return Some(LRESULT(0));
    }
    None
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    std::panic::catch_unwind(|| route(hwnd, msg, wp, lp)).unwrap_or(LRESULT(0))
}
unsafe fn route(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let owner = HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut _);
    let state = GetWindowLongPtrW(owner, GWLP_USERDATA) as *const State;
    if !state.is_null() && (*state).session_end.paused() && matches!(msg, WM_COMMAND | WM_CLOSE) {
        return LRESULT(0);
    }
    if msg == WM_WINDOWPOSCHANGING && lp.0 != 0 && !state.is_null() {
        content::before_show(hwnd, &*state, &mut *(lp.0 as *mut WINDOWPOS));
    }
    if !state.is_null() && scroll::dispatch(hwnd, msg, wp, &(*state).tool_panel_wheel) {
        return LRESULT(0);
    }
    match msg {
        WM_MOUSEACTIVATE | WM_SETFOCUS => {
            // 其他文档的模态/截图状态可能在本窗口无消息时改变，交互前重算门禁。
            refresh(owner);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_COMMAND => {
            if (wp.0 >> 16) == BN_CLICKED as usize {
                let _ = PostMessageW(Some(owner), EXECUTE, WPARAM(wp.0 & 0xffff), LPARAM(0));
                let _ = SetFocus(Some(owner));
            }
            LRESULT(0)
        }
        WM_DRAWITEM => {
            let pointer = GetWindowLongPtrW(owner, GWLP_USERDATA) as *const State;
            if !pointer.is_null() && lp.0 != 0 {
                draw::button(&*(lp.0 as *const DRAWITEMSTRUCT), &*pointer);
            }
            LRESULT(1)
        }
        WM_ERASEBKGND => {
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            draw::fill(HDC(wp.0 as *mut _), &rect, 0x18120e);
            LRESULT(1)
        }
        WM_SIZE => {
            layout(hwnd);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            if let Err(error) = font::update(hwnd, wp.0 as u16 as u32) {
                eprintln!("native-tool-panel-font: {error}");
            }
            let rect = &*(lp.0 as *const RECT);
            if let Err(error) = position::fit(hwnd, Some(*rect)) {
                eprintln!("native-tool-panel-position: {error}");
            }
            layout(hwnd);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            font::release(hwnd);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_CLOSE => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            let _ = SetFocus(Some(owner));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

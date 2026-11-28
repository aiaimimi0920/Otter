//! On-demand Win32 tool panel; the system destroys its window/buttons when the owner dies.
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "tool_panel_layout_tests.rs"]
mod layout_tests;
#[path = "edit_strip_popup.rs"]
pub(super) mod popup;
#[path = "tool_panel_position.rs"]
pub(super) mod position;
#[path = "edit_strip.rs"]
mod strip;
#[cfg(test)]
pub(super) use strip::icons_for_test;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) use strip::smoke as pinned_smoke;
pub(super) use strip::{follow, moving_positions};
#[path = "tool_panel_scroll.rs"]
mod scroll;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "tool_panel_smoke.rs"]
mod smoke;
use children::layout;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) use smoke::run as smoke;
pub(super) const EXECUTE: u32 = WM_APP + 90;
fn entries() -> impl Iterator<Item = (usize, &'static str)> {
    super::tool_commands::TOOLS
        .iter()
        .map(|entry| (entry.id, entry.label()))
        .chain(
            super::tool_panel_actions::ACTIONS
                .iter()
                .map(|&(id, label_id)| (id, crate::ui_text::text(label_id))),
        )
}

pub(super) unsafe fn translate(state: &State, message: &MSG) -> bool {
    if state.click.active() {
        return false;
    }
    if super::pin_settings::translate(state, message) {
        return true;
    }
    if keyboard::translate_property(state, message) {
        return true;
    }
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
    if neuro_mini_ui::is_toolbar(panel)
        && super::edit_mode::shortcut(
            owner,
            state,
            message.message,
            message.wParam.0,
            message.lParam.0 & (1 << 30) != 0,
        )
    {
        return true;
    }
    let handled = IsDialogMessageW(panel, message).as_bool();
    if handled && !neuro_mini_ui::is_toolbar(panel) {
        scroll::reveal_focus(panel);
    }
    handled
}

#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
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

pub(super) unsafe fn create(owner: HWND, state: &State, visible: bool) -> Result<Window> {
    if state.canvas.borrow().pinned {
        return strip::create(owner, state, visible);
    }
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
    // AdjustWindowRectEx APIs exclude standard scrollbars; reserve space explicitly so ordinary workspaces do not clip the last column.
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
            &windows::core::HSTRING::from(crate::ui_text::text("tool-panel-title")),
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
    crate::desktop::pin_click_through::panels::bind(panel.0, owner, state, false)?;
    state.tool_panel.set(panel.0);
    // Ordinary workspaces retain advance preparation; hidden pins create BUTTON only on first use.
    if visible || !state.canvas.borrow().pinned {
        prepare(panel.0, state)?;
    }
    if visible {
        let _ = ShowWindow(panel.0, SW_SHOWNOACTIVATE);
    }
    Ok(panel)
}
pub(super) unsafe fn refresh_language(state: &State) {
    let panel = state.tool_panel.get();
    if !IsWindow(Some(panel)).as_bool() {
        return;
    }
    let title = if neuro_mini_ui::is_toolbar(panel) {
        "edit-strip-title"
    } else {
        "tool-panel-title"
    };
    let _ = SetWindowTextW(
        panel,
        &windows::core::HSTRING::from(crate::ui_text::text(title)),
    );
    if !neuro_mini_ui::is_toolbar(panel) && content::ready(panel) {
        for entry in super::tool_commands::TOOLS {
            if let Ok(button) = GetDlgItem(Some(panel), entry.id as i32) {
                let _ = SetWindowTextW(button, &windows::core::HSTRING::from(entry.caption(state)));
            }
        }
        layout(panel);
    }
    invalidate(state);
}
pub(super) unsafe fn invalidate(state: &State) {
    if neuro_mini_ui::is_toolbar(state.tool_panel.get()) {
        strip::refresh(state);
        return;
    }
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
// Compare after command/timer processing, not before, so the last asynchronous delivery immediately reaches the panel too.
pub(super) unsafe fn refresh(owner: HWND) {
    if !IsWindow(Some(owner)).as_bool() {
        return;
    }
    let pointer = GetWindowLongPtrW(owner, GWLP_USERDATA) as *const State;
    if pointer.is_null() {
        return;
    }
    let state = &*pointer;
    if neuro_mini_ui::is_toolbar(state.tool_panel.get()) {
        strip::refresh(state);
        return;
    }
    if !IsWindow(Some(state.tool_panel.get())).as_bool() || !content::ready(state.tool_panel.get())
    {
        return;
    }
    // Reentrant messages may arrive while outer UI operations still hold Canvas; post-return checks supply the refresh.
    let Ok(canvas) = state.canvas.try_borrow() else {
        return;
    };
    let tool = canvas.tool;
    let style = canvas.editor.style();
    let Some(view) = Availability::read(state, &canvas).map(Availability::presentation) else {
        return;
    };
    drop(canvas);
    let mask = entries().enumerate().fold(0u64, |mask, (index, (id, _))| {
        mask | ((view.allows(id) as u64) << index)
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
        if neuro_mini_ui::is_toolbar(state.tool_panel.get()) {
            if lp.0 == neuro_mini_ui::PANEL {
                if let Err(e) = super::pin_settings::execute(hwnd, state, wp.0 as u16) {
                    eprintln!("native-pin-settings: {e}");
                }
            } else if lp.0 == neuro_mini_ui::PANEL_CLOSE {
                super::pin_settings::close(state);
            } else if lp.0 == neuro_mini_ui::PROPERTY {
                if let Err(e) = super::edit_properties::execute(hwnd, state, wp.0 as u16) {
                    eprintln!("native-edit-properties: {e}");
                }
            } else if wp.0 == 0 && lp.0 == neuro_mini_ui::PREPARED {
                strip::refresh(state);
            } else if wp.0 == 0 && lp.0 == neuro_mini_ui::CLOSE {
                strip::close(hwnd, state);
            } else {
                strip::execute(hwnd, state, wp.0, lp.0 == 1);
            }
            return Some(LRESULT(0));
        }
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
            // Other documents' modal/capture state can change without messages here; recalculate admission before interaction.
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

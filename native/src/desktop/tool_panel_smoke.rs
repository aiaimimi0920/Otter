//! Hidden-HWND smoke checks for tool panels/effect buttons; compiled only in Debug or explicit smoke-test.
use super::{entries, prepare, refresh, scroll, verify_captions, EXECUTE};
use crate::desktop::State;
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{Input::KeyboardAndMouse::IsWindowEnabled, WindowsAndMessaging::*},
    },
};

pub(in crate::desktop) unsafe fn run(owner: HWND, state: &State) -> Result<()> {
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
        for command in [crate::desktop::context_menu::WIDTH_UP, 0x4F, 0x46] {
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
        if crate::desktop::tool_panel_actions::label(0x46, state).as_deref() != Some("填充 开") {
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
        Some(WPARAM(crate::desktop::context_menu::WIDTH_UP)),
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

unsafe fn effect_controls_smoke(owner: HWND, state: &State) -> Result<()> {
    use crate::editor::Tool;
    let original = state.canvas.borrow().tool;
    let has_image = state.canvas.borrow().image.is_some();
    let result = (|| {
        for tool in [Tool::Blur, Tool::Mosaic, Tool::Select] {
            // Check actual HWND availability refresh only; do not click modal-input buttons or start blur tasks.
            state.canvas.borrow_mut().tool = tool;
            SendMessageW(owner, WM_NULL, None, None);
            for (command, enabled) in [
                (
                    crate::desktop::blur_default::COMMAND,
                    has_image && tool == Tool::Blur,
                ),
                (
                    crate::desktop::numeric_style::MOSAIC,
                    has_image && tool == Tool::Mosaic,
                ),
                (crate::desktop::blur_numeric::COMMAND, false),
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

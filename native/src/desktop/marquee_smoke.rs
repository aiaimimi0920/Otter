//! 隐藏 HWND 的框选消息链；只修改当前线程键盘状态，测试后恢复文档及选择。
use super::State;
use crate::{
    editor::{Editor, Tool},
    viewport::Point,
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        UI::{HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

struct Keyboard([u8; 256]);
impl Keyboard {
    unsafe fn new() -> Result<Self> {
        let mut original = [0; 256];
        GetKeyboardState(&mut original)?;
        Ok(Self(original))
    }
    unsafe fn shift(&self, down: bool) -> Result<()> {
        let mut keys = self.0;
        for key in [
            VK_CONTROL,
            VK_LCONTROL,
            VK_RCONTROL,
            VK_MENU,
            VK_LMENU,
            VK_RMENU,
            VK_SHIFT,
            VK_LSHIFT,
            VK_RSHIFT,
        ] {
            keys[key.0 as usize] = 0;
        }
        if down {
            keys[VK_SHIFT.0 as usize] = 0x80;
        }
        SetKeyboardState(&keys)
    }
}
impl Drop for Keyboard {
    fn drop(&mut self) {
        unsafe {
            let _ = SetKeyboardState(&self.0);
        }
    }
}
fn require(value: bool, message: &'static str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::new(E_FAIL, message))
    }
}
unsafe fn pointer(hwnd: HWND, state: &State, message: u32, x: f32, y: f32) {
    let view = state.canvas.borrow().viewport;
    let dpi = GetDpiForWindow(hwnd).max(1) as f32 / 96.0;
    let x = ((view.origin.x + x * view.scale) * dpi).round() as i16;
    let y = ((view.origin.y + y * view.scale) * dpi).round() as i16;
    let value = x as u16 as u32 | ((y as u16 as u32) << 16);
    let _ = SendMessageW(hwnd, message, Some(WPARAM(0)), Some(LPARAM(value as isize)));
}
unsafe fn exercise(hwnd: HWND, state: &State, keys: &Keyboard) -> Result<()> {
    keys.shift(false)?;
    let revision = state.canvas.borrow().editor.revision();
    pointer(hwnd, state, WM_LBUTTONDOWN, 1.0, 1.0);
    require(
        GetCapture() == hwnd && state.canvas.borrow().marquee.is_some(),
        "Marquee did not capture pointer",
    )?;
    pointer(hwnd, state, WM_MOUSEMOVE, 15.0, 15.0);
    let _ = SendMessageW(hwnd, WM_PAINT, None, None);
    pointer(hwnd, state, WM_LBUTTONUP, 15.0, 15.0);
    require(
        state.canvas.borrow().selected_indices() == [0],
        "Marquee message selection mismatch",
    )?;
    require(GetCapture() != hwnd, "Marquee did not release capture")?;

    keys.shift(true)?;
    pointer(hwnd, state, WM_LBUTTONDOWN, 2.0, 28.0);
    require(
        GetCapture() == hwnd && state.canvas.borrow().marquee.is_some(),
        "Shift marquee did not capture pointer",
    )?;
    pointer(hwnd, state, WM_MOUSEMOVE, 30.0, 18.0);
    let view = state.canvas.borrow().viewport;
    let _ = SendMessageW(
        hwnd,
        WM_MOUSEWHEEL,
        Some(WPARAM(120 << 16)),
        Some(LPARAM(0)),
    );
    require(
        state.canvas.borrow().viewport == view,
        "Marquee wheel changed viewport",
    )?;
    pointer(hwnd, state, WM_LBUTTONUP, 30.0, 18.0);
    require(
        state.canvas.borrow().selected_indices() == [0, 1],
        "Shift marquee did not append selection",
    )?;
    require(
        state.canvas.borrow().editor.revision() == revision,
        "Marquee created edit history",
    )?;

    pointer(hwnd, state, WM_LBUTTONDOWN, 1.0, 1.0);
    pointer(hwnd, state, WM_MOUSEMOVE, 30.0, 18.0);
    let _ = SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
    require(
        state.canvas.borrow().marquee.is_none() && GetCapture() != hwnd,
        "Marquee escape failed",
    )?;
    require(
        state.canvas.borrow().selected_indices() == [0, 1],
        "Marquee escape lost original selection",
    )?;
    keys.shift(false)?;
    let _ = SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_DELETE.0 as usize)), None);
    require(
        state.canvas.borrow().editor.elements().is_empty(),
        "Marquee group delete failed",
    )?;
    require(
        state.canvas.borrow_mut().editor.undo(),
        "Marquee group undo failed",
    )?;
    require(
        state.canvas.borrow().editor.elements().len() == 2,
        "Marquee group undo was partial",
    )?;
    Ok(())
}

pub(super) unsafe fn run(hwnd: HWND, state: &State) -> Result<()> {
    let keys = Keyboard::new()?;
    let (original, tool, selected, group) = {
        let mut canvas = state.canvas.borrow_mut();
        let mut fixture = Editor::with_layers(canvas.editor.layers().unwrap().clone()).unwrap();
        for point in [Point { x: 8.0, y: 8.0 }, Point { x: 24.0, y: 24.0 }] {
            fixture.begin(Tool::Brush, point).unwrap();
            fixture.commit();
        }
        let tool = canvas.tool;
        let selected = canvas.selected;
        let group = canvas.multi_selection.clone();
        canvas.select_tool(Tool::Select);
        (
            std::mem::replace(&mut canvas.editor, fixture),
            tool,
            selected,
            group,
        )
    };
    let result = exercise(hwnd, state, &keys);
    {
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        canvas.select_tool(tool);
        canvas.editor = original;
        canvas.selected = selected;
        canvas.multi_selection = group;
        canvas.discard_device();
    }
    if GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
    result?;
    let original_keys = keys.0;
    drop(keys);
    let mut restored = [0; 256];
    GetKeyboardState(&mut restored)?;
    require(
        restored == original_keys,
        "Marquee keyboard state not restored",
    )?;
    println!("native-marquee-smoke: pointer=true shift=true capture=true escape=true wheel=true undo=true");
    Ok(())
}

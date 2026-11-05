//! 隐藏窗口翻转链路验证：真实定时器消息、像素、撤销及 Esc；不注入系统输入。
use super::{flip_input, State};
use crate::{erase_layers::Layers, raster_flip::Axis};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, GetKeyboardState, SetKeyboardState, VK_CONTROL, VK_LCONTROL, VK_RCONTROL,
};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

// 仅改变当前测试线程的输入状态表，不产生系统输入；任何提前返回均恢复原表。
struct KeyboardState([u8; 256]);
impl KeyboardState {
    unsafe fn control(down: bool) -> Result<Self> {
        let mut original = [0; 256];
        GetKeyboardState(&mut original)?;
        let mut state = original;
        for key in [VK_CONTROL, VK_LCONTROL, VK_RCONTROL] {
            state[key.0 as usize] = if down { 0x80 } else { 0 };
        }
        SetKeyboardState(&state)?;
        Ok(Self(original))
    }
}
impl Drop for KeyboardState {
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
unsafe fn finish(hwnd: HWND, state: &State) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while state.flip_job.borrow().is_busy() {
        require(
            std::time::Instant::now() < deadline,
            "Flip smoke timer timeout",
        )?;
        require(
            IsWindow(Some(hwnd)).as_bool(),
            "Flip smoke window destroyed",
        )?;
        let mut message = MSG::default();
        // 只处理本窗口消息，避免消费调用线程的退出消息或其他窗口输入。
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
fn mirrored(source: &Layers, actual: &Layers, axis: Axis) -> bool {
    if source.size != actual.size {
        return false;
    }
    let (width, height) = source.size;
    // 独立坐标公式，不用被测翻转函数生成预期值。
    let check = |before: &[u8], after: &[u8]| {
        if before.len() != after.len() {
            return false;
        }
        (0..height).all(|y| {
            (0..width).all(|x| {
                let (sx, sy) = match axis {
                    Axis::Horizontal => (width - 1 - x, y),
                    Axis::Vertical => (x, height - 1 - y),
                };
                let to = ((y * width + x) * 4) as usize;
                let from = ((sy * width + sx) * 4) as usize;
                after[to..to + 4] == before[from..from + 4]
            })
        })
    };
    check(&source.base, &actual.base)
        && match (&source.annotations, &actual.annotations) {
            (None, None) => true,
            (Some(before), Some(after)) => check(before, after),
            _ => false,
        }
}
pub(super) unsafe fn run(hwnd: HWND, state: &State) -> Result<()> {
    for (command, axis) in [
        (flip_input::HORIZONTAL, Axis::Horizontal),
        (flip_input::VERTICAL, Axis::Vertical),
    ] {
        let source = state.canvas.borrow().editor.layers().unwrap().clone();
        let revision = state.canvas.borrow().editor.revision();
        flip_input::start(hwnd, state, command);
        require(
            state.flip_job.borrow().is_busy(),
            "Flip smoke did not start worker",
        )?;
        require(
            state.canvas.borrow().editor.revision() == revision,
            "Flip smoke committed before polling",
        )?;
        super::smoke_timing::run("flip-worker-wait", || finish(hwnd, state))?;
        {
            let canvas = state.canvas.borrow();
            require(
                canvas.editor.revision() == revision + 1,
                "Flip smoke revision mismatch",
            )?;
            require(
                mirrored(&source, canvas.editor.layers().unwrap(), axis),
                "Flip smoke pixel mismatch",
            )?;
            require(
                canvas.image.as_ref().unwrap().pixels[..]
                    == canvas.editor.layers().unwrap().base[..],
                "Flip smoke stale canvas image",
            )?;
        }
        super::smoke_timing::run("flip-paint", || {
            let _ = SendMessageW(hwnd, WM_PAINT, None, None);
            Ok::<(), Error>(())
        })?;
        {
            let mut canvas = state.canvas.borrow_mut();
            require(canvas.editor.undo(), "Flip smoke undo failed")?;
            canvas.sync_raster_source();
            require(
                canvas.editor.layers().unwrap().same_storage(&source),
                "Flip smoke undo source mismatch",
            )?;
            require(canvas.editor.redo(), "Flip smoke redo failed")?;
            canvas.sync_raster_source();
            require(
                mirrored(&source, canvas.editor.layers().unwrap(), axis),
                "Flip smoke redo pixels mismatch",
            )?;
            require(canvas.editor.undo(), "Flip smoke restore failed")?;
            canvas.sync_raster_source();
        }
        super::smoke_timing::run("flip-hit-region", || super::hit_region::update(hwnd, state))?;
    }
    for control in [false, true] {
        let source = state.canvas.borrow().editor.layers().unwrap().clone();
        let revision = state.canvas.borrow().editor.revision();
        flip_input::start(hwnd, state, flip_input::HORIZONTAL);
        require(
            state.flip_job.borrow().is_busy(),
            "Cancel smoke did not start worker",
        )?;
        let mut before = [0; 256];
        GetKeyboardState(&mut before)?;
        {
            let _keyboard = KeyboardState::control(control)?;
            require(
                (GetKeyState(VK_CONTROL.0 as i32) < 0) == control,
                "Smoke Ctrl state mismatch",
            )?;
            SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(0x1B)), Some(LPARAM(0)));
        }
        let mut after = [0; 256];
        GetKeyboardState(&mut after)?;
        require(
            before == after,
            "Cancel smoke did not restore keyboard state",
        )?;
        super::smoke_timing::run("flip-cancel-wait", || finish(hwnd, state))?;
        require(
            state.canvas.borrow().editor.revision() == revision,
            "Cancelled flip changed document",
        )?;
        require(
            state
                .canvas
                .borrow()
                .editor
                .layers()
                .unwrap()
                .same_storage(&source),
            "Cancelled flip changed source",
        )?;
    }
    println!("native-cancel-smoke: escape=true ctrl-escape=true");
    println!("native-flip-smoke: horizontal=true vertical=true timer=true undo=true redo=true escape=true");
    Ok(())
}

//! 两个隐藏宿主的真实消息与关闭隔离；不注册全局输入，不访问桌面或用户数据。
use super::{document_window::DocumentWindow, State};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub(super) unsafe fn run(instance: HINSTANCE, first: &DocumentWindow) -> Result<()> {
    let (image, pinned, stamp) = {
        let canvas = first.state.canvas.borrow();
        (
            canvas.image.clone(),
            canvas.pinned,
            canvas.editor.document_stamp(),
        )
    };
    let second = DocumentWindow::create(instance, State::new(image, pinned)?, false)?;
    {
        let mut canvas = second.state.canvas.borrow_mut();
        if stamp.matches(&canvas.editor) {
            return Err(Error::new(E_FAIL, "Two HWNDs share document identity"));
        }
        if canvas.image.is_some() {
            use crate::{editor::Tool, viewport::Point};
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
            if canvas.editor.elements().len() != 1 {
                return Err(Error::new(E_FAIL, "Secondary document edit failed"));
            }
        }
    }
    SendMessageW(second.window.0, WM_CLOSE, None, None);
    if IsWindow(Some(second.window.0)).as_bool()
        || IsWindow(Some(second.panel.0)).as_bool()
        || !IsWindow(Some(first.window.0)).as_bool()
        || drain_quit()?
    {
        return Err(Error::new(
            E_FAIL,
            "Closing one document terminated other hosts",
        ));
    }
    drop(second);
    let before = first.state.canvas.borrow().paints;
    SendMessageW(first.window.0, WM_NULL, None, None);
    SendMessageW(first.window.0, WM_PAINT, None, None);
    let canvas = first.state.canvas.borrow();
    if !stamp.matches(&canvas.editor)
        || canvas.paints != before + 1
        || first.state.error.borrow().is_some()
    {
        return Err(Error::new(E_FAIL, "Remaining document no longer responds"));
    }
    println!("native-multi-host-smoke: independent=true close-one=true remaining-paint=true");
    Ok(())
}

pub(super) unsafe fn verify_last_exit() -> Result<()> {
    if !drain_quit()? {
        return Err(Error::new(
            E_FAIL,
            "Last document did not request clean exit",
        ));
    }
    println!("native-multi-host-exit-smoke: last-window=true");
    Ok(())
}

unsafe fn drain_quit() -> Result<bool> {
    // WM_QUIT 是低优先级生成消息；只过滤它会让其他排队消息阻止其出现。
    // 有界排空当前测试线程消息，不使用阻塞等待或全局输入。
    let mut message = MSG::default();
    for _ in 0..1024 {
        if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            return Ok(false);
        }
        if message.message == WM_QUIT {
            return if message.wParam.0 == 0 {
                Ok(true)
            } else {
                Err(Error::new(E_FAIL, "Window callback requested failure exit"))
            };
        }
        if IsWindow(Some(message.hwnd)).as_bool() {
            DispatchMessageW(&message);
        }
    }
    Err(Error::new(
        E_FAIL,
        "Window message queue did not settle within smoke budget",
    ))
}

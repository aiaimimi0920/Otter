//! 单个宿主的资源所有者：窗口集合可移动 owner，但 HWND 保存的 State 地址保持不变。
use super::{app_icon, tool_panel, State, Window};
#[path = "parameter_panel.rs"]
pub(super) mod parameter_panel;
#[path = "document_restore_smoke.rs"]
mod restore_smoke;
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

pub(super) struct DocumentWindow {
    // Rust 字段按声明顺序析构。子面板/宿主必须先于借用的图标和回调状态释放。
    pub parameters: Option<parameter_panel::Panel>,
    pub panel: Window,
    pub window: Window,
    pub icons: app_icon::Icons,
    pub state: Box<State>,
}

impl DocumentWindow {
    pub unsafe fn is_alive(&self) -> bool {
        IsWindow(Some(self.window.0)).as_bool()
            && GetWindowLongPtrW(self.window.0, GWLP_USERDATA)
                == (&*self.state as *const State) as isize
    }
    pub fn observe_priority(&self) -> Result<()> {
        if self.state.canvas.borrow().pinned && self.state.priority_watcher.borrow().is_none() {
            *self.state.priority_watcher.borrow_mut() =
                Some(super::window_priority::Watcher::install(self.window.0)?);
        }
        Ok(())
    }
    pub unsafe fn smoke_relocation(self) -> Result<Self> {
        let pointer = (&*self.state as *const State) as isize;
        let mut owners = Vec::with_capacity(1);
        owners.push(self);
        // 模拟未来窗口集合扩容。owner 可搬移，传给 Win32 的状态地址不能随之改变。
        owners.reserve_exact(8);
        let moved = owners.pop().unwrap();
        if (&*moved.state as *const State) as isize != pointer
            || GetWindowLongPtrW(moved.window.0, GWLP_USERDATA) != pointer
            || GetWindowLongPtrW(moved.panel.0, GWLP_USERDATA) != moved.window.0 .0 as isize
        {
            return Err(Error::new(
                E_FAIL,
                "Document owner relocation invalidated window state",
            ));
        }
        SendMessageW(moved.window.0, WM_NULL, None, None);
        moved.icons.verify(moved.window.0)?;
        moved.icons.verify(moved.panel.0)?;
        restore_smoke::verify(moved.window.0)?;
        println!("native-document-owner-smoke: relocated=true state-stable=true icons=true");
        Ok(moved)
    }

    pub unsafe fn create(instance: HINSTANCE, state: State, visible_panel: bool) -> Result<Self> {
        Self::create_inner(instance, state, visible_panel, None)
    }

    /// 恢复事务传入已完成 DPI 转换的物理框；允许无图 Art，创建期间仍保持隐藏。
    pub unsafe fn create_at(
        instance: HINSTANCE,
        state: State,
        frame: crate::pin_geometry::PinRect,
    ) -> Result<Self> {
        if !state.canvas.borrow().pinned
            || frame.width == 0
            || frame.height == 0
            || frame.width > 8192
            || frame.height > 8192
            || u64::from(frame.width) * u64::from(frame.height) > 16_777_216
            || i64::from(frame.x) + i64::from(frame.width) > i64::from(i32::MAX)
            || i64::from(frame.y) + i64::from(frame.height) > i64::from(i32::MAX)
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid restored physical window frame",
            ));
        }
        Self::create_inner(instance, state, false, Some(frame))
    }

    unsafe fn create_inner(
        instance: HINSTANCE,
        state: State,
        visible_panel: bool,
        frame: Option<crate::pin_geometry::PinRect>,
    ) -> Result<Self> {
        // 先固定地址，再注册 HWND。构造失败时局部变量逆序释放，也满足相同生命周期约束。
        let state = Box::new(state);
        let icons = super::smoke_timing::run("document-icons", app_icon::Icons::new)?;
        let pinned = state.canvas.borrow().pinned;
        let (width, height) = if let Some(frame) = frame {
            (frame.width as i32, frame.height as i32)
        } else if pinned {
            let canvas = state.canvas.borrow();
            let image = canvas
                .image
                .as_ref()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Pin requires image"))?;
            let (image_width, image_height) = canvas
                .editor
                .annotation_frame()
                .map_or((image.width as f32, image.height as f32), |frame| {
                    (frame.width as f32, frame.height as f32)
                });
            let scale = if state.capture_origin.is_some() {
                1.0
            } else {
                (1200.0 / image_width).min(800.0 / image_height).min(1.0)
            };
            (
                (image_width * scale).round().max(1.0) as i32,
                (image_height * scale).round().max(1.0) as i32,
            )
        } else {
            (1000, 720)
        };
        let window = Window(super::smoke_timing::run("document-host", || {
            CreateWindowExW(
                if pinned {
                    WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_TOPMOST
                } else {
                    WINDOW_EX_STYLE::default()
                },
                w!("Otter.Native.Workspace"),
                w!("Otter Native"),
                if pinned {
                    WS_POPUP
                } else {
                    WS_OVERLAPPEDWINDOW
                },
                if let Some(frame) = frame {
                    frame.x
                } else if pinned {
                    state.capture_origin.map_or(100, |origin| origin.x)
                } else {
                    CW_USEDEFAULT
                },
                if let Some(frame) = frame {
                    frame.y
                } else if pinned {
                    state.capture_origin.map_or(100, |origin| origin.y)
                } else {
                    CW_USEDEFAULT
                },
                width,
                height,
                None,
                None,
                Some(instance),
                Some((&*state as *const State).cast()),
            )
        })?);
        if pinned && state.canvas.borrow().image.is_some() {
            let mut canvas = state.canvas.borrow_mut();
            canvas.viewport.origin = crate::viewport::Point::default();
            canvas.viewport.scale = width as f32 * 96.0
                / GetDpiForWindow(window.0).max(1) as f32
                / canvas.image.as_ref().unwrap().width as f32;
        }
        let panel = super::smoke_timing::run("document-panel", || {
            tool_panel::create(window.0, &state, visible_panel)
        })?;
        if pinned && state.canvas.borrow().editor.annotation_frame().is_some() {
            super::pin_window::fit(window.0, &state);
        }
        icons.attach(window.0);
        icons.attach(panel.0);
        if frame.is_some() {
            super::pin_window::fit(window.0, &state);
        }
        Ok(Self {
            parameters: None,
            panel,
            window,
            icons,
            state,
        })
    }
}

impl Drop for DocumentWindow {
    fn drop(&mut self) {
        // RAII 退出路径在任何 HWND 析构前停止订阅；用户关闭路径由 WM_DESTROY 同步停止。
        self.state.priority_watcher.borrow_mut().take();
        unsafe {
            // HWND 可被系统复用；已失去本 owner 身份的句柄绝不能再由旧 Window 析构销毁。
            if !self.is_alive() {
                self.window.0 = HWND::default();
                self.panel.0 = HWND::default();
            }
        }
    }
}

//! Resource owner for one host: window collections may move owners, but the State address stored in HWND remains stable.
use super::{app_icon, tool_panel, State, Window};
#[path = "parameter_panel.rs"]
pub(super) mod parameter_panel;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "document_restore_smoke.rs"]
mod restore_smoke;
use windows::{
    core::{w, Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub(super) struct DocumentWindow {
    // Rust drops fields in declaration order. Release child panels/hosts before borrowed icons and callback state.
    pub parameters: Option<parameter_panel::Panel>,
    pub panel: Window,
    pub window: Window,
    pub icons: app_icon::Icons,
    pub state: Box<State>,
}

fn initial_size(width: f32, height: f32, captured: bool) -> (i32, i32) {
    if captured {
        // Keep captures 1:1; shrink only HWND when exceeding presentation budgets, preserving source images/coordinates.
        let (width, height) = (f64::from(width), f64::from(height));
        let scale = (8192.0 / width.max(height))
            .min((16_777_216.0 / (width * height)).sqrt())
            .min(1.0);
        (
            (width * scale).floor().max(1.0) as i32,
            (height * scale).floor().max(1.0) as i32,
        )
    } else {
        let scale = (1200.0 / width).min(800.0 / height).min(1.0);
        (
            (width * scale).round().max(1.0) as i32,
            (height * scale).round().max(1.0) as i32,
        )
    }
}

impl DocumentWindow {
    /// Empty documents carry only commands, tray, and hotkeys; explicit images/restored imageless Art still display normally.
    pub unsafe fn show_content(&self, command: SHOW_WINDOW_CMD) {
        let (pinned, empty) = {
            let canvas = self.state.canvas.borrow();
            (canvas.pinned, !canvas.pinned && canvas.image.is_none())
        };
        if empty || self.state.group_hidden.get() || self.state.shake.hidden.get() {
            return;
        }
        let _ = ShowWindow(self.window.0, command);
        if !pinned {
            let _ = ShowWindow(self.panel.0, SW_SHOWNOACTIVATE);
        }
    }

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
    #[cfg(any(test, debug_assertions, feature = "smoke-test"))]
    pub unsafe fn smoke_relocation(self) -> Result<Self> {
        let pointer = (&*self.state as *const State) as isize;
        let mut owners = Vec::with_capacity(1);
        owners.push(self);
        // Simulate future window-collection growth. Owners may move, but state addresses passed to Win32 must remain stable.
        owners.reserve_exact(8);
        let moved = owners.pop().unwrap();
        if (&*moved.state as *const State) as isize != pointer || !moved.is_alive() {
            return Err(Error::new(
                E_FAIL,
                "Document owner relocation invalidated window state",
            ));
        }
        // Pin edit-strip GWLP_USERDATA stores Neuro-mini private state, not host HWND.
        let panel = moved.panel.0;
        let host = moved.window.0;
        let panel_owner = if moved.state.canvas.borrow().pinned {
            neuro_mini_ui::owner(panel) == Some(host)
        } else {
            GetWindowLongPtrW(panel, GWLP_USERDATA) == host.0 as isize
        };
        if !panel_owner || moved.state.tool_panel.get() != panel {
            return Err(Error::new(
                E_FAIL,
                "Document owner relocation invalidated panel ownership",
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

    /// Restoration transactions supply physical bounds after DPI conversion; allow imageless Art and remain hidden during creation.
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
        // Fix the address before registering HWND. On construction failure, reverse local destruction order satisfies the same lifetime constraints.
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
            initial_size(image_width, image_height, state.capture_origin.is_some())
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
            // Fit using actual client dimensions on both axes; one-pixel rounding for extreme aspect ratios must not crop the other axis.
            super::pin_window::fit(window.0, &state);
        }
        let panel = super::smoke_timing::run("document-panel", || {
            tool_panel::create(window.0, &state, visible_panel)
        })?;
        icons.attach(window.0);
        icons.attach(panel.0);
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
        // RAII exit stops subscriptions before any HWND destruction; user closure stops synchronously in WM_DESTROY.
        self.state.priority_watcher.borrow_mut().take();
        unsafe {
            // Systems may reuse HWNDs; old Window destructors must never destroy handles that have lost this owner's identity.
            if !self.is_alive() {
                self.window.0 = HWND::default();
                self.panel.0 = HWND::default();
            }
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "document_window_tests.rs"]
mod tests;

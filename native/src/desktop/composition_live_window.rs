//! Live 专用 HWND；不复用可编辑钉图 State，不启动捕获，不拥有应用消息循环。
use super::{
    host::{Host, Update},
    notification, Texture,
};
use crate::live_frames::Consumer;
#[path = "composition_live_art.rs"]
mod art;
#[path = "composition_live_freeze_geometry.rs"]
mod freeze_geometry;
#[path = "composition_live_groups.rs"]
pub(in crate::desktop) mod groups;
#[path = "composition_live_window_input.rs"]
mod input;
#[path = "composition_live_local.rs"]
pub(in crate::desktop) mod local;
#[path = "composition_live_menu.rs"]
mod menu;
#[path = "composition_live_placement.rs"]
mod placement;
#[path = "composition_live_publication.rs"]
pub(in crate::desktop) mod publication;
use std::{
    cell::{Cell, RefCell},
    sync::OnceLock,
};
use windows::{
    core::{w, Error, Result, HRESULT},
    Win32::{
        Foundation::*,
        Graphics::{Direct2D::Common::D2D_SIZE_U, Gdi::*},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

struct State {
    chrome: crate::desktop::application::chrome::Binding,
    session_end_reason: Cell<bool>,
    publication: publication::Control,
    groups: RefCell<groups::Control>,
    group_locked: Cell<bool>,
    group_hidden: Cell<bool>,
    group_was_visible: Cell<bool>,
    recycle_requested: Cell<bool>,
    recycle_on_close: Cell<bool>,
    freeze_requested: Cell<bool>,
    snapshot_requested: Cell<bool>,
    link_requested: Cell<bool>,
    art_requested: Cell<bool>,
    opacity: Cell<f32>,
    drag: Cell<Option<local::drag::Drag>>,
    drag_starting: Cell<bool>,
    drag_revision: Cell<u64>,
    menu_open: Cell<bool>,
    unlink_requested: Cell<bool>,
    host: RefCell<Option<Host>>,
    error: RefCell<Option<Error>>,
    closed: Cell<bool>,
    deferred: Cell<bool>,
    input: RefCell<input::Control>,
}
impl State {
    // Win32/GPU 调用可能重入；不跨调用持有 RefCell 借用，关闭期间不重新挂回 owner。
    fn with_host<T>(&self, operation: impl FnOnce(&mut Host) -> Result<T>) -> Result<Option<T>> {
        let Some(mut host) = self.host.borrow_mut().take() else {
            self.deferred.set(true);
            return Ok(None);
        };
        let result = operation(&mut host);
        if !self.closed.get() {
            *self.host.borrow_mut() = Some(host);
        }
        result.map(Some)
    }
    fn stop(&self) {
        self.art_requested.set(false);
        self.drag.set(None);
        self.drag_starting.set(false);
        self.input.borrow_mut().cancel();
        self.closed.set(true);
        let host = self.host.borrow_mut().take();
        drop(host);
    }
}

pub struct LiveWindow {
    unit_id: crate::workspace_id::UnitId,
    hwnd: HWND,
    state: Box<State>,
}
impl LiveWindow {
    pub(in crate::desktop) fn bind_workspace_chrome(
        &self,
        binding: std::rc::Weak<crate::desktop::application::chrome::Ui>,
    ) {
        *self.state.chrome.borrow_mut() = binding;
    }
    pub(in crate::desktop) fn enable_recycle_close(&self) {
        self.state.recycle_on_close.set(true);
    }
    pub fn take_recycle_request(&self) -> bool {
        self.state.recycle_requested.replace(false)
    }
    /// 应用已准备冻结记录；失败且 HWND 仍属于本 owner 时不得发布回收结果。
    pub(in crate::desktop) fn close_for_recycle(&self) -> Result<()> {
        if !self.is_alive() {
            return Err(Error::new(E_ABORT, "Live recycle window already closed"));
        }
        let result = unsafe { DestroyWindow(self.hwnd) };
        if self.owns_hwnd() {
            return Err(result
                .err()
                .unwrap_or_else(|| Error::new(E_FAIL, "Live recycle window remained open")));
        }
        Ok(())
    }
    pub fn unit_id(&self) -> &crate::workspace_id::UnitId {
        &self.unit_id
    }
    pub fn take_freeze_request(&self) -> bool {
        self.state.freeze_requested.replace(false)
    }
    pub fn take_snapshot_request(&self) -> bool {
        self.state.snapshot_requested.replace(false)
    }
    pub fn take_link_request(&self) -> bool {
        self.state.link_requested.replace(false)
    }
    pub fn opacity(&self) -> f32 {
        self.state.opacity.get()
    }
    pub(in crate::desktop) fn persistence_state(&self) -> Option<(RECT, f32)> {
        let mut rect = RECT::default();
        (self.is_alive() && unsafe { GetWindowRect(self.hwnd, &mut rect) }.is_ok())
            .then(|| (rect, self.opacity()))
    }
    pub(in crate::desktop) fn exit_idle(&self) -> bool {
        self.persistence_idle()
            && !self.state.snapshot_requested.get()
            && !self.state.link_requested.get()
            && !self.state.unlink_requested.get()
            && self.state.groups.borrow().request.is_none()
    }
    pub(in crate::desktop) fn persistence_idle(&self) -> bool {
        self.is_alive()
            && self.state.drag.get().is_none()
            && !self.state.drag_starting.get()
            && !self.state.menu_open.get()
            && !self.state.freeze_requested.get()
            && !self.state.recycle_requested.get()
            && !self.state.art_requested.get()
    }
    pub fn take_unlink_request(&self) -> bool {
        self.state.unlink_requested.replace(false)
    }
    #[cfg(test)]
    pub(in crate::desktop) fn hwnd_for_test(&self) -> HWND {
        self.hwnd
    }
    #[cfg(test)]
    pub(in crate::desktop) fn frame_for_test(
        &self,
    ) -> Option<std::sync::Arc<crate::live_frames::Frame<Texture>>> {
        self.state.host.borrow().as_ref()?.frame_for_test()
    }
    /// 创建隐藏窗口并设置系统捕获排除后才绑定源；调用方负责 DPI 上下文与显式用户授权。
    pub fn new(consumer: Consumer<Texture>, width: u32, height: u32) -> Result<Self> {
        super::super::validate_size(D2D_SIZE_U { width, height })?;
        unsafe {
            register()?;
            let unit_id = crate::desktop::document_state::new_unit_id()?;
            let state = Box::new(State {
                chrome: Default::default(),
                session_end_reason: Cell::new(false),
                publication: publication::Control::default(),
                groups: RefCell::default(),
                group_locked: Cell::new(false),
                group_hidden: Cell::new(false),
                group_was_visible: Cell::new(false),
                recycle_requested: Cell::new(false),
                recycle_on_close: Cell::new(false),
                freeze_requested: Cell::new(false),
                snapshot_requested: Cell::new(false),
                link_requested: Cell::new(false),
                art_requested: Cell::new(false),
                opacity: Cell::new(1.0),
                drag: Cell::new(None),
                drag_starting: Cell::new(false),
                drag_revision: Cell::new(0),
                menu_open: Cell::new(false),
                unlink_requested: Cell::new(false),
                host: RefCell::new(None),
                error: RefCell::new(None),
                closed: Cell::new(false),
                deferred: Cell::new(false),
                input: RefCell::new(input::Control::default()),
            });
            let hwnd = CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW,
                w!("Otter.Native.Live"),
                w!("Otter Live"),
                WS_POPUP | WS_SYSMENU,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width as i32,
                height as i32,
                None,
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                Some((&*state as *const State).cast()),
            )?;
            let window = Self {
                hwnd,
                state,
                unit_id,
            };
            input::install(hwnd)?;
            art::install(hwnd)?;
            crate::desktop::application::chrome::ports::install_live(hwnd)?;
            publication::install(hwnd)?;
            window.state.groups.borrow_mut().menu = Some(groups::install(hwnd)?);
            AppendMenuW(
                GetSystemMenu(hwnd, false),
                MF_STRING,
                0x1f90,
                w!("断开 Live 连线…"),
            )?;
            AppendMenuW(
                GetSystemMenu(hwnd, false),
                MF_STRING,
                0x1fa0,
                w!("从 Live 创建连线…"),
            )?;
            AppendMenuW(
                GetSystemMenu(hwnd, false),
                MF_STRING,
                0x1fb0,
                w!("刷新连线快照并通知下游"),
            )?;
            AppendMenuW(
                GetSystemMenu(hwnd, false),
                MF_STRING,
                0x1fe0,
                w!("停止 Live 并保留当前画面"),
            )?;
            AppendMenuW(
                GetSystemMenu(hwnd, false),
                MF_STRING,
                0x1fd0,
                w!("关闭 Live 并移入回收站"),
            )?;
            AppendMenuW(
                GetSystemMenu(hwnd, false),
                MF_STRING,
                0x1fc0,
                w!("放弃 Live（不保留画面）"),
            )?;
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)?;
            *window.state.host.borrow_mut() = Some(Host::bind(hwnd, consumer)?);
            Ok(window)
        }
    }
    pub fn is_alive(&self) -> bool {
        !self.state.closed.get() && self.owns_hwnd()
    }
    fn owns_hwnd(&self) -> bool {
        unsafe {
            IsWindow(Some(self.hwnd)).as_bool()
                && GetWindowLongPtrW(self.hwnd, GWLP_USERDATA)
                    == (&*self.state as *const State) as isize
        }
    }
    pub fn show(&self) -> Result<()> {
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Live window is closed"));
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
    pub fn take_error(&self) -> Option<Error> {
        self.state.error.borrow_mut().take()
    }
    /// 仅移交预算 owner，不在窗口回调内读回 GPU；重入占用时返回 None，不使用旧缓存。
    pub fn snapshot_frame(
        &self,
    ) -> Result<Option<std::sync::Arc<crate::live_frames::Frame<Texture>>>> {
        if !self.is_alive() {
            return Ok(None);
        }
        self.state
            .with_host(Host::snapshot_frame)
            .map(Option::flatten)
    }
    /// 供应用已有事件补偿遗漏通知；无后续事件时不承诺关闭必达。
    pub fn refresh(&self) -> Result<()> {
        if !self.is_alive() {
            return Ok(());
        }
        unsafe {
            handle_update(self.hwnd, self.state.with_host(Host::refresh)?)?;
            reschedule(self.hwnd, &self.state);
            Ok(())
        }
    }
}
impl Drop for LiveWindow {
    fn drop(&mut self) {
        let alive = self.owns_hwnd();
        self.state.stop();
        if alive {
            unsafe {
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}

unsafe fn register() -> Result<()> {
    static REGISTERED: OnceLock<std::result::Result<(), HRESULT>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| {
            let result = (|| -> Result<()> {
                let class = WNDCLASSW {
                    style: CS_DBLCLKS,
                    lpfnWndProc: Some(procedure),
                    hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                    hCursor: LoadCursorW(None, IDC_ARROW)?,
                    lpszClassName: w!("Otter.Native.Live"),
                    ..Default::default()
                };
                if RegisterClassW(&class) == 0 {
                    return Err(Error::from_win32());
                }
                Ok(())
            })();
            result.map_err(|error| error.code())
        })
        .map_err(Error::from_hresult)
}
unsafe fn handle_update(hwnd: HWND, update: Option<Update>) -> Result<()> {
    match update {
        Some(Update::Closed) => DestroyWindow(hwnd),
        Some(Update::Frame) => {
            if !InvalidateRect(Some(hwnd), None, false).as_bool() {
                return Err(Error::from_win32());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
unsafe fn reschedule(hwnd: HWND, state: &State) -> bool {
    // 重入通知已从系统队列消费，但 Host 暂被外层绘制借走；外层 EndPaint 后补一个失效区。
    if !state.closed.get() && state.host.borrow().is_some() && state.deferred.replace(false) {
        return InvalidateRect(Some(hwnd), None, false).as_bool();
    }
    false
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
    let Some(state) = pointer.as_ref() else {
        return DefWindowProcW(hwnd, message, wp, lp);
    };
    crate::desktop::application::chrome::geometry(&state.chrome, hwnd, message);
    if let Some(result) = crate::desktop::session_end::dispatch_active(
        hwnd,
        &state.session_end_reason,
        message,
        wp,
        lp,
    ) {
        return result;
    }
    let result = if let Some(result) = input::handle(hwnd, state, message, wp, lp) {
        result
    } else {
        match message {
            WM_SYSCOMMAND
                if [
                    publication::PUBLISH,
                    publication::PUBLISH_CONTROL,
                    publication::PUBLISH_OBSERVE,
                    publication::STOP,
                ]
                .contains(&(wp.0 & 0xfff0)) =>
            {
                publication::queue(state, wp.0 & 0xfff0);
                Ok(())
            }
            WM_SYSCOMMAND if groups::handles(wp.0 & 0xfff0) => {
                groups::queue(state, wp.0 & 0xfff0);
                Ok(())
            }
            WM_SYSCOMMAND
                if state.group_locked.get()
                    && matches!((wp.0 & 0xfff0) as u32, SC_MOVE | SC_SIZE) =>
            {
                Ok(())
            }
            // 显式放弃保留退出通道：无首帧或旧工作区来源也不能把窗口困住。
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1f90 => {
                if !state.group_locked.get() && !state.group_hidden.get() {
                    state.unlink_requested.set(true);
                }
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fa0 => {
                if !state.group_locked.get() && !state.group_hidden.get() {
                    state.link_requested.set(true);
                }
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fb0 => {
                if !state.group_locked.get() && !state.group_hidden.get() {
                    state.snapshot_requested.set(true);
                }
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fc0 => DestroyWindow(hwnd),
            WM_CLOSE if state.recycle_on_close.get() => {
                state.recycle_requested.set(true);
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fd0 => {
                state.recycle_requested.set(true);
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fe0 => {
                state.freeze_requested.set(true);
                Ok(())
            }
            notification::MESSAGE => state
                .with_host(|host| host.receive(message, wp, lp))
                .and_then(|update| handle_update(hwnd, update)),
            WM_PAINT => paint(hwnd, state),
            WM_SIZE => {
                if wp.0 == SIZE_MINIMIZED as usize {
                    let _ = state.with_host(|host| {
                        host.suspend();
                        Ok(())
                    });
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
                Ok(())
            }
            WM_ERASEBKGND => return LRESULT(1),
            WM_DESTROY => {
                state.stop();
                Ok(())
            }
            WM_NCDESTROY => {
                state.stop();
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                return DefWindowProcW(hwnd, message, wp, lp);
            }
            _ => return DefWindowProcW(hwnd, message, wp, lp),
        }
    };
    if let Err(error) = result {
        *state.error.borrow_mut() = Some(error);
        state.stop();
        if GetWindowLongPtrW(hwnd, GWLP_USERDATA) == pointer as isize {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(hwnd);
        }
    }
    reschedule(hwnd, state);
    LRESULT(0)
}
unsafe fn paint(hwnd: HWND, state: &State) -> Result<()> {
    let mut paint = PAINTSTRUCT::default();
    let _ = BeginPaint(hwnd, &mut paint);
    let result = (|| {
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client)?;
        // 最小化窗口可能仍返回非零 client rect，不能据此重新创建交换链。
        if IsIconic(hwnd).as_bool() {
            client = RECT::default();
        }
        state.with_host(|host| {
            host.set_opacity(state.opacity.get())?;
            host.set_chrome_visible(crate::desktop::application::chrome::live_frame(
                &state.chrome,
            ));
            host.paint(D2D_SIZE_U {
                width: (client.right - client.left).max(0) as u32,
                height: (client.bottom - client.top).max(0) as u32,
            })?;
            Ok(if host.is_closed() {
                Update::Closed
            } else {
                Update::Idle
            })
        })
    })();
    let _ = EndPaint(hwnd, &paint);
    result.and_then(|update| handle_update(hwnd, update))
}

#[cfg(test)]
#[path = "composition_live_window_tests.rs"]
mod tests;

//! 声明式 Surface 的窗口 owner；隐藏创建，按需绘制，关闭时先释放原生呈现资源。
use super::surface_host::Host;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
#[path = "surface_window_accessibility.rs"]
mod accessibility_bridge;
#[path = "surface_window_cursor.rs"]
mod cursor;
#[path = "surface_window_edit.rs"]
mod edit;
#[path = "surface_window_focus_notifications.rs"]
mod focus_notifications;
#[path = "surface_window_gesture.rs"]
mod gesture;
#[path = "surface_window_javascript_focus.rs"]
mod javascript_focus;
#[path = "surface_window_javascript_keyboard.rs"]
mod javascript_keyboard;
#[path = "surface_window_javascript_pointer.rs"]
mod javascript_pointer;
#[path = "surface_window_keyboard.rs"]
mod keyboard;
#[path = "surface_window_notifications.rs"]
mod notifications;
#[path = "surface_window_pointer.rs"]
mod pointer;
#[path = "surface_window_repeat.rs"]
mod repeat;
#[path = "surface_window_scene.rs"]
mod scene;
#[path = "surface_window_scroll.rs"]
mod scroll;
use super::surface_scroll_repeat as scroll_repeat;
#[cfg(test)]
#[path = "surface_window_scroll_tests.rs"]
pub(in crate::desktop) mod scroll_tests;
#[path = "surface_window_scrollbar.rs"]
mod scrollbar;
#[path = "surface_window_session_end.rs"]
mod session_end;
#[path = "surface_window_state_lifetime.rs"]
mod state_lifetime;
#[path = "surface_window_structure_notifications.rs"]
mod structure_notifications;
#[path = "surface_window_switch.rs"]
mod switch;
#[cfg(test)]
#[path = "surface_switch_window_tests.rs"]
mod switch_tests;
#[path = "surface_uia_root.rs"]
mod uia;
use crate::surface_scene::Scene;
pub(crate) use pointer::Activation;
pub use pointer::Click;
#[path = "surface_window_deferred.rs"]
mod deferred_click;
pub(crate) use deferred_click::DeferredClick;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::OnceLock,
};
use windows::{
    core::{w, Error, Result, HRESULT},
    Win32::{
        Foundation::*, Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};
struct State {
    session_end_reason: Cell<bool>,
    structure_notifications: structure_notifications::Pending,
    focus_notifications: focus_notifications::Pending,
    notifications: RefCell<notifications::Pending>,
    accessibility_key: Cell<usize>,
    uia: RefCell<Option<uia::Root>>,
    accessibility: RefCell<Option<crate::surface_accessibility::Owner>>,
    host: RefCell<Option<Host>>,
    error: RefCell<Option<Error>>,
    closed: Cell<bool>,
    deferred: Cell<bool>,
    viewport_epoch: Cell<u64>,
    focus_epoch: Cell<u64>,
    javascript_keyboard: Cell<bool>,
    javascript_pointer: RefCell<javascript_pointer::Pointer>,
    pointer_cancel_pending: Cell<bool>,
    pointer_releasing: Cell<bool>,
    pointer: RefCell<pointer::Pointer>,
    keyboard: RefCell<keyboard::Keyboard>,
    local_edit: Cell<bool>,
    scroll_drag: RefCell<Option<crate::surface_scroll::Drag>>,
    scroll_repeat: RefCell<Option<scroll_repeat::Repeat>>,
    pan: RefCell<Option<gesture::Pan>>,
}
pub struct SurfaceWindow {
    hwnd: HWND,
    state: Rc<State>,
}
impl SurfaceWindow {
    pub fn new(scene: Scene, width: u32, height: u32) -> Result<Self> {
        Self::with_images(
            scene,
            width,
            height,
            crate::surface_presentation::Images::new(),
        )
    }
    /// 显式交付当前场景像素；不从节点 URL/文件路径读取，也不代替上层来源授权。
    pub fn with_images(
        scene: Scene,
        width: u32,
        height: u32,
        images: crate::surface_presentation::Images,
    ) -> Result<Self> {
        if !(64..=8192).contains(&width) || !(64..=8192).contains(&height) {
            return Err(Error::new(E_INVALIDARG, "Invalid Surface window size"));
        }
        unsafe {
            register()?;
            let state = Rc::new(State {
                session_end_reason: Cell::new(false),
                structure_notifications: Default::default(),
                focus_notifications: Default::default(),
                notifications: RefCell::new(Default::default()),
                accessibility_key: Cell::new(0),
                uia: RefCell::new(None),
                accessibility: RefCell::new(Some(crate::surface_accessibility::Owner::new(&scene))),
                host: RefCell::new(None),
                error: RefCell::new(None),
                closed: Cell::new(false),
                deferred: Cell::new(false),
                viewport_epoch: Cell::new(0),
                focus_epoch: Cell::new(0),
                javascript_keyboard: Cell::new(false),
                javascript_pointer: RefCell::new(Default::default()),
                pointer_cancel_pending: Cell::new(false),
                pointer_releasing: Cell::new(false),
                pointer: RefCell::new(pointer::Pointer::default()),
                keyboard: RefCell::new(keyboard::Keyboard::default()),
                local_edit: Cell::new(false),
                scroll_drag: RefCell::new(None),
                scroll_repeat: RefCell::new(None),
                pan: RefCell::new(None),
            });
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("Otter.Native.Surface"),
                w!("Otter Surface"),
                WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width as i32,
                height as i32,
                None,
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                Some((&*state as *const State).cast()),
            )?;
            let window = Self { hwnd, state };
            gesture::configure(hwnd)?;
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)?;
            *window.state.host.borrow_mut() = Some(Host::bind_with_images(hwnd, scene, images)?);
            accessibility_bridge::register(hwnd, &window.state);
            Ok(window)
        }
    }
    fn owns_hwnd(&self) -> bool {
        unsafe {
            IsWindow(Some(self.hwnd)).as_bool()
                && GetWindowLongPtrW(self.hwnd, GWLP_USERDATA)
                    == (&*self.state as *const State) as isize
        }
    }
    pub fn is_alive(&self) -> bool {
        !self.state.closed.get() && self.owns_hwnd()
    }
    pub(super) fn dialog_owner(&self) -> Result<HWND> {
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window closed"));
        }
        Ok(self.hwnd)
    }
    pub fn show(&self) -> Result<()> {
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window closed"));
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
        self.invalidate()
    }
    fn invalidate(&self) -> Result<()> {
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window closed"));
        }
        if !unsafe { InvalidateRect(Some(self.hwnd), None, false) }.as_bool() {
            return Err(Error::from_win32());
        }
        Ok(())
    }
    pub fn take_error(&self) -> Option<Error> {
        self.state.error.borrow_mut().take()
    }
    /// 声明式节点描述句柄；场景替换撤销旧身份，UIA 控制模式仍经窗口权限门禁。
    pub fn accessibility_node(&self, id: &str) -> Option<crate::surface_accessibility::Handle> {
        if !self.is_alive() {
            return None;
        }
        self.state.accessibility.borrow().as_ref()?.node(id)
    }
    /// 有效窗口的客户区像素命中；仅返回身份，调用方仍须验证授权和生命周期。
    pub fn hit_test(&self, client_pixels: [i32; 2]) -> Result<Option<String>> {
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window closed"));
        }
        let mut hit = None;
        let ran = self.state.run(|host| {
            hit = host.hit_test(client_pixels)?.map(str::to_owned);
            Ok(())
        })?;
        if !ran || !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window busy or closed"));
        }
        Ok(hit)
    }
    pub fn close(&self) -> Result<()> {
        self.state.stop();
        if self.owns_hwnd() {
            unsafe {
                DestroyWindow(self.hwnd)?;
            }
        }
        Ok(())
    }
    /// 只准备有界事件，不发送；返回前复核窗口，重入关闭不得泄漏旧场景动作。
    pub fn click_events(
        &self,
        client_pixels: [i32; 2],
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
        modifiers: crate::surface_click_event::Modifiers,
    ) -> Result<Vec<crate::surface_click_event::ClickEvent>> {
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window closed"));
        }
        let mut events = Vec::new();
        let ran = self.state.run(|host| {
            events = host.click_events(client_pixels, binding, revision, modifiers)?;
            Ok(())
        })?;
        if !ran || !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window busy or closed"));
        }
        Ok(events)
    }
}
impl Drop for SurfaceWindow {
    fn drop(&mut self) {
        self.state.stop();
        if self.owns_hwnd() {
            unsafe {
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                let _ = DestroyWindow(self.hwnd);
                uia::destroyed(self.hwnd);
            }
        }
    }
}
unsafe fn register() -> Result<()> {
    static REGISTERED: OnceLock<std::result::Result<(), HRESULT>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| {
            (|| -> Result<()> {
                if RegisterClassW(&WNDCLASSW {
                    style: CS_DBLCLKS,
                    lpfnWndProc: Some(procedure),
                    hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                    hCursor: LoadCursorW(None, IDC_ARROW)?,
                    lpszClassName: w!("Otter.Native.Surface"),
                    ..Default::default()
                }) == 0
                {
                    return Err(Error::from_win32());
                }
                Ok(())
            })()
            .map_err(|e| e.code())
        })
        .map_err(Error::from_hresult)
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lp.0 as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
    if pointer.is_null() {
        return DefWindowProcW(hwnd, message, wp, lp);
    }
    // userdata 指向仍由窗口 owner 持有的 Rc 数据。当前回调独立保活，防止同步重入
    // 释放 SurfaceWindow 后，外层消息继续访问 State；from_raw 消费刚增加的引用。
    Rc::increment_strong_count(pointer);
    let owner = Rc::from_raw(pointer);
    let state = &*owner;
    if let Some(result) = state.session_message(hwnd, message, wp, lp) {
        return result;
    }
    javascript_focus::observe(hwnd, state, message, wp, lp);
    if message == structure_notifications::MESSAGE {
        structure_notifications::flush(state);
        return LRESULT(0);
    }
    if message == focus_notifications::MESSAGE {
        focus_notifications::flush(hwnd, state);
        return LRESULT(0);
    }
    if matches!(
        message,
        WM_SETFOCUS | WM_KILLFOCUS | WM_SHOWWINDOW | WM_ENABLE
    ) {
        if message == WM_KILLFOCUS {
            focus_notifications::lost(state);
        }
        focus_notifications::schedule(state);
    }
    if message == notifications::MESSAGE {
        notifications::flush(hwnd, state);
        return LRESULT(0);
    }
    if let Some(result) = edit::message(hwnd, state, message, wp, lp) {
        return result;
    }
    match message {
        WM_GETOBJECT if lp.0 as i32 == windows::Win32::UI::Accessibility::UiaRootObjectId => {
            if state.closed.get() {
                return LRESULT(0);
            }
            if state.uia.borrow().is_none() {
                if state.host.borrow().is_none() {
                    return LRESULT(0);
                }
                match uia::Root::new(hwnd) {
                    Ok(root) => {
                        if let Err(error) = root.set_scene(
                            state
                                .accessibility
                                .borrow()
                                .as_ref()
                                .and_then(|owner| owner.root()),
                        ) {
                            *state.error.borrow_mut() = Some(error);
                            return LRESULT(0);
                        }
                        *state.uia.borrow_mut() = Some(root);
                    }
                    Err(error) => {
                        *state.error.borrow_mut() = Some(error);
                        return LRESULT(0);
                    }
                }
            }
            // 不跨 UIA 调用持有 RefCell；系统可同步重入并销毁窗口。
            let provider = state.uia.borrow().as_ref().map(uia::Root::provider);
            return windows::Win32::UI::Accessibility::UiaReturnRawElementProvider(
                hwnd,
                wp,
                lp,
                provider.as_ref(),
            );
        }
        WM_GESTURE => {
            if gesture::message(hwnd, state, lp) {
                return LRESULT(0);
            }
            return DefWindowProcW(hwnd, message, wp, lp);
        }
        WM_TIMER if repeat::timer(hwnd, state, wp.0, std::time::Instant::now()) => {
            return LRESULT(0)
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            if scroll::message(hwnd, state, message, wp, lp) {
                return LRESULT(0);
            }
            return DefWindowProcW(hwnd, message, wp, lp);
        }
        WM_SETCURSOR if cursor::set(hwnd, state, lp) => return LRESULT(1),
        WM_KEYDOWN | WM_KEYUP | WM_CHAR => {
            if keyboard::message(hwnd, state, message, wp, lp) {
                return LRESULT(0);
            }
            return DefWindowProcW(hwnd, message, wp, lp);
        }
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_LBUTTONUP | WM_MOUSEMOVE | WM_MOUSELEAVE
        | WM_CAPTURECHANGED | WM_CANCELMODE | WM_KILLFOCUS => {
            if scrollbar::message(hwnd, state, message, lp) {
                return LRESULT(0);
            }
            if javascript_pointer::message(hwnd, state, message, wp, lp) {
                return LRESULT(0);
            }
            javascript_pointer::hover_message(hwnd, state, message, wp, lp);
            if pointer::message(hwnd, state, message, lp) {
                return LRESULT(0);
            }
            return DefWindowProcW(hwnd, message, wp, lp);
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            BeginPaint(hwnd, &mut paint);
            // 绘制可能同步重入窗口消息，不能跨 D2D 调用持有 RefCell 借用。
            let focus = state.keyboard.borrow().indicator().map(str::to_owned);
            let hover = state.pointer.borrow().hovered().map(str::to_owned);
            let pressed = state
                .keyboard
                .borrow()
                .pressed()
                .map(str::to_owned)
                .or_else(|| state.pointer.borrow().pressed_node().map(str::to_owned));
            let result = state.run(|host| {
                host.paint_with_visual(crate::surface_button_visual::State {
                    focus: focus.as_deref(),
                    hover: hover.as_deref(),
                    pressed: pressed.as_deref(),
                })
            });
            let _ = EndPaint(hwnd, &paint);
            if let Err(error) = result {
                *state.error.borrow_mut() = Some(error);
            }
            if !state.closed.get() && state.host.borrow().is_some() && state.deferred.replace(false)
            {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
        }
        WM_ACTIVATEAPP if wp.0 == 0 => {
            pointer::invalidate(hwnd, state);
        }
        WM_ACTIVATE if wp.0 as u16 == WA_INACTIVE as u16 => {
            pointer::invalidate(hwnd, state);
        }
        WM_SHOWWINDOW if wp.0 == 0 => {
            pointer::invalidate(hwnd, state);
        }
        WM_SIZE => {
            state
                .viewport_epoch
                .set(state.viewport_epoch.get().saturating_add(1));
            pointer::invalidate(hwnd, state);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        WM_DPICHANGED => {
            state
                .viewport_epoch
                .set(state.viewport_epoch.get().saturating_add(1));
            pointer::invalidate(hwnd, state);
            if lp.0 != 0 {
                let rect = &*(lp.0 as *const RECT);
                // checked_sub 防止畸形建议矩形溢出；不改变激活状态或 Z 序。
                let size = rect
                    .right
                    .checked_sub(rect.left)
                    .zip(rect.bottom.checked_sub(rect.top));
                if let Some((width, height)) =
                    size.filter(|(w, h)| (1..=32768).contains(w) && (1..=32768).contains(h))
                {
                    if let Err(error) = SetWindowPos(
                        hwnd,
                        None,
                        rect.left,
                        rect.top,
                        width,
                        height,
                        SWP_NOACTIVATE | SWP_NOZORDER,
                    ) {
                        *state.error.borrow_mut() = Some(error);
                    }
                }
            }
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_DESTROY => {
            state.stop();
            uia::destroyed(hwnd);
        }
        WM_NCDESTROY => {
            state.stop();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            return DefWindowProcW(hwnd, message, wp, lp);
        }
        _ => return DefWindowProcW(hwnd, message, wp, lp),
    }
    LRESULT(0)
}
#[cfg(test)]
#[path = "surface_window_accessibility_tests.rs"]
mod accessibility_tests;
#[cfg(test)]
#[path = "surface_window_tests.rs"]
mod tests;

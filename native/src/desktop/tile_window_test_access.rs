//! 仅隔离测试使用的真实自有HWND消息入口；不编译进普通产品。
use super::*;
use windows::Win32::{
    Graphics::Direct2D::{Common::D2D_SIZE_U, ID2D1Factory},
    System::{StationsAndDesktops::*, Threading::GetCurrentThreadId},
};
#[link(name = "imm32")]
unsafe extern "system" {
    fn ImmDisableIME(thread: u32) -> i32;
}
struct InputDesktop(HDESK);
impl Drop for InputDesktop {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseDesktop(self.0);
        }
    }
}
fn desktop_name(desktop: HDESK) -> Result<String> {
    let mut buffer = [0u16; 512];
    let mut needed = 0;
    unsafe {
        GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_NAME,
            Some(buffer.as_mut_ptr().cast()),
            1024,
            Some(&mut needed),
        )?;
    }
    if !(2..=1024).contains(&needed) || needed % 2 != 0 {
        return Err(Error::new(E_INVALIDARG, "Private test desktop name"));
    }
    let content = &buffer[..needed as usize / 2];
    if content.last() != Some(&0) {
        return Err(Error::new(E_INVALIDARG, "Private test desktop terminator"));
    }
    String::from_utf16(&content[..content.len() - 1])
        .map_err(|_| Error::new(E_INVALIDARG, "Private test desktop encoding"))
}
/// 私有desktop的DXGI非S_OK不能伪造提交；键盘夹具显式使用原CPU后端的真实提交。
/// 只识别runner专属desktop并再核不是input desktop；本模块整体只编译到测试。
pub(super) fn private_surface(
    factory: &ID2D1Factory,
    size: Size,
) -> Result<Option<crate::desktop::render_surface::RenderSurface>> {
    let thread = unsafe { GetCurrentThreadId() };
    let current = desktop_name(unsafe { GetThreadDesktop(thread)? })?;
    if !current.starts_with("Otter.Background.Tests.") {
        return Ok(None);
    }
    let input = InputDesktop(unsafe {
        OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS)?
    });
    if current.eq_ignore_ascii_case(&desktop_name(input.0)?) {
        return Err(Error::new(
            E_ACCESSDENIED,
            "Test desktop is the input desktop",
        ));
    }
    // 不在不可见desktop等待系统IME；只禁用本测试线程，人工组合边沿不等候选窗验收。
    if unsafe { ImmDisableIME(thread) } == 0 {
        return Err(Error::new(E_FAIL, "Private test thread IME isolation"));
    }
    crate::desktop::render_surface::RenderSurface::software(
        factory,
        D2D_SIZE_U {
            width: size.width,
            height: size.height,
        },
    )
    .map(Some)
}
impl Window {
    pub(crate) fn test_art_slider_message(&self, message: u32, pixel: [i32; 2]) {
        let point = LPARAM((pixel[0] as u16 as isize) | ((pixel[1] as u16 as isize) << 16));
        unsafe {
            SendMessageW(self.hwnd, message, Some(WPARAM(0)), Some(point));
        }
    }
    pub(crate) fn test_art_slider_active(&self) -> bool {
        matches!(&*self.state.pressed.borrow(), actions::Gesture::Slider(_))
    }
    pub(crate) fn test_identification_presented(&self) -> Option<String> {
        self.state
            .displayed_marker
            .borrow()
            .as_ref()
            .map(|m| m.identity.request.clone())
    }
    pub(crate) fn test_art_click(&self, pixel: [i32; 2]) -> bool {
        let before = self.state.shared.lock().actions.test_offered;
        self.test_art_slider_message(WM_LBUTTONDOWN, pixel);
        self.test_art_slider_message(WM_LBUTTONUP, pixel);
        self.state.shared.lock().actions.test_offered > before
    }
    pub(crate) fn test_art_choice(
        &self,
        choice: Option<crate::surface_confirmation::Choice>,
    ) -> bool {
        actions::test_choice(self.hwnd, &self.state, choice)
    }
    pub(crate) fn test_art_select(&self, pixel: [i32; 2], selected: Option<usize>) -> bool {
        actions::test_value(self.hwnd, &self.state, pixel, selected)
    }
    pub(crate) fn test_identification_escape(&self) {
        unsafe {
            SendMessageW(self.hwnd, WM_KEYDOWN, Some(WPARAM(27)), Some(LPARAM(0)));
        }
    }
    pub(crate) fn test_art_scrollbar_to_end(&self, mode: &str) -> bool {
        match mode {
            "drag" => {
                self.test_art_slider_message(WM_LBUTTONDOWN, [635, 100]);
                self.test_art_slider_message(WM_MOUSEMOVE, [635, 700]);
                self.test_art_slider_message(WM_LBUTTONUP, [635, 700]);
            }
            "page" => {
                self.test_art_slider_message(WM_LBUTTONDOWN, [635, 450]);
                self.test_art_slider_message(WM_LBUTTONUP, [635, 450]);
            }
            _ => panic!("unknown scrollbar oracle"),
        }
        assert!(self.state.scrollbar.borrow().gesture_is_idle());
        actions::active(&self.state)
            .and_then(|frame| {
                self.state
                    .renderer
                    .borrow()
                    .as_ref()?
                    .focus_at(&frame, [20, 445])
            })
            .is_some_and(|f| f.node == "send")
    }
}

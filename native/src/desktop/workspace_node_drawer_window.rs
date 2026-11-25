//! Ownerless top-level attached hosts avoid automatic system raising; callbacks register only credentialed bounded intents.
use super::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
const CLASS: windows::core::PCWSTR = w!("Otter.Native.NodeDrawer");
pub(super) struct Window(pub HWND, isize);
impl Window {
    pub unsafe fn new(data: &Rc<Data>) -> Result<Self> {
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        if RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: CLASS,
            lpfnWndProc: Some(procedure),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        }) == 0
            && GetLastError() != ERROR_CLASS_ALREADY_EXISTS
        {
            return Err(Error::from_win32());
        }
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_CONTROLPARENT,
            CLASS,
            &HSTRING::from(crate::ui_text::text("workspace-node-properties")),
            WS_POPUP | WS_CLIPCHILDREN,
            0,
            0,
            20,
            10,
            None,
            None,
            Some(instance),
            Some(Rc::as_ptr(data).cast()),
        )?;
        let owner = Self(hwnd, Rc::as_ptr(data) as isize);
        data.hwnd.set(hwnd);
        SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)?;
        Ok(owner)
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.0)).as_bool()
                && GetWindowLongPtrW(self.0, GWLP_USERDATA) == self.1
            {
                let _ = DestroyWindow(self.0);
            }
        }
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    std::panic::catch_unwind(|| route(hwnd, msg, wp, lp)).unwrap_or(LRESULT(0))
}
unsafe fn route(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lp.0 as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let Some(data) = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Data).as_ref() else {
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    match msg {
        WM_CLOSE => {
            if let Some(ui) = data.chrome.upgrade().filter(|ui| !ui.blocked()) {
                if let Err(error) = data.set_open(false) {
                    ui.fail(error);
                }
                data.pending.set(None);
                let _ = PostMessageW(Some(data.host), WM_NULL, WPARAM(0), LPARAM(0));
            }
            LRESULT(0)
        }
        NOTIFY => {
            let context = (wp.0 >> 32) as u32;
            if data.root.get().0 as isize == lp.0
                && data.context.get() == context
                && context != 0
                && data.chrome.upgrade().is_some_and(|ui| !ui.blocked())
                && GetWindowLongPtrW(data.host, GWLP_USERDATA) == data.marker
                && IsWindowVisible(hwnd).as_bool()
                && data.pending.get().is_none()
                && PostMessageW(Some(data.host), WM_NULL, WPARAM(0), LPARAM(0)).is_ok()
            {
                data.pending.set(Some((context, (wp.0 & 0xffff) as u16)));
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_WINDOWPOSCHANGED => {
            let result = DefWindowProcW(hwnd, msg, wp, lp);
            if let Some(ui) = data.chrome.upgrade() {
                if let Err(error) = window::order_drawing(&ui) {
                    ui.fail(error);
                }
            }
            result
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

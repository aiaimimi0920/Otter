//! Tile的原生编辑层：owned popup在DComp之上，只转发自有EDIT通知，不持Rust会话裸指针。
use super::DISMISS;
use windows::{
    core::{w, Error, Result, HRESULT},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};
const MARKER: windows::core::PCWSTR = w!("Otter.Tile.Text.Owner");
const EPOCH: windows::core::PCWSTR = w!("Otter.Tile.Text.Epoch");
pub(in crate::desktop::tile_window) struct Popup {
    hwnd: HWND,
    marker: Box<u8>,
    owner: HWND,
}
impl Popup {
    pub fn new(owner: HWND) -> Result<Self> {
        static CLASS: std::sync::OnceLock<std::result::Result<(), HRESULT>> =
            std::sync::OnceLock::new();
        CLASS
            .get_or_init(|| {
                (|| -> Result<()> {
                    unsafe {
                        if RegisterClassW(&WNDCLASSW {
                            lpfnWndProc: Some(procedure),
                            hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                            hCursor: LoadCursorW(None, IDC_IBEAM)?,
                            hbrBackground: GetSysColorBrush(COLOR_WINDOW),
                            lpszClassName: w!("Otter.Native.Tile.Edit"),
                            ..Default::default()
                        }) == 0
                        {
                            Err(Error::from_win32())
                        } else {
                            Ok(())
                        }
                    }
                })()
                .map_err(|e| e.code())
            })
            .map_err(Error::from_hresult)?;
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("Otter.Native.Tile.Edit"),
                w!(""),
                WS_POPUP | WS_CLIPCHILDREN,
                0,
                0,
                1,
                1,
                Some(owner),
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                None,
            )?
        };
        let popup = Self {
            hwnd,
            marker: Box::new(0),
            owner,
        };
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, owner.0 as isize);
            if let Err(error) = SetPropW(
                hwnd,
                MARKER,
                Some(HANDLE((&*popup.marker as *const u8).cast_mut().cast())),
            ) {
                let _ = DestroyWindow(hwnd);
                return Err(error);
            }
        }
        Ok(popup)
    }
    pub fn hwnd(&self) -> Result<HWND> {
        if self.alive() {
            Ok(self.hwnd)
        } else {
            Err(Error::new(E_HANDLE, "Tile edit popup retired"))
        }
    }
    pub fn show(&self, owner: HWND, rect: [i32; 4]) -> Result<()> {
        self.hwnd()?;
        if owner != self.owner {
            return Err(Error::new(E_HANDLE, "Tile edit owner changed"));
        }
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let epoch = NEXT
            .fetch_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |v| v.checked_add(1),
            )
            .map_err(|_| Error::new(E_FAIL, "Tile edit epoch exhausted"))?;
        let mut p = POINT {
            x: rect[0],
            y: rect[1],
        };
        unsafe {
            SetPropW(self.hwnd, EPOCH, Some(HANDLE(epoch as *mut _)))?;
            if !ClientToScreen(owner, &mut p).as_bool() {
                return Err(Error::from_win32());
            }
            SetWindowPos(
                self.hwnd,
                None,
                p.x,
                p.y,
                rect[2] - rect[0],
                rect[3] - rect[1],
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
            let _ = ShowWindow(self.hwnd()?, SW_SHOWNOACTIVATE);
        }
        self.hwnd()?;
        Ok(())
    }
    pub fn hide(&self) {
        unsafe {
            if self.alive() {
                let _ = RemovePropW(self.hwnd, EPOCH);
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
    }
    pub fn matches(&self, epoch: WPARAM, popup: LPARAM) -> bool {
        self.alive()
            && epoch.0 != 0
            && popup.0 == self.hwnd.0 as isize
            && unsafe { GetPropW(self.hwnd, EPOCH).0 as usize == epoch.0 }
    }
    fn alive(&self) -> bool {
        unsafe {
            GetPropW(self.hwnd, MARKER).0 == (&*self.marker as *const u8).cast_mut().cast()
                && GetParent(self.hwnd).ok() == Some(self.owner)
        }
    }
    pub fn install(&self, edit: HWND) -> Result<()> {
        self.install_policy(edit, false)
    }
    pub fn install_select(&self, select: HWND) -> Result<()> {
        self.install_policy(select, true)
    }
    fn install_policy(&self, edit: HWND, select: bool) -> Result<()> {
        if unsafe { SetWindowSubclass(edit, Some(key), 0x54455854, usize::from(select) << 1) }
            .as_bool()
        {
            Ok(())
        } else {
            Err(Error::from_win32())
        }
    }
}
impl Drop for Popup {
    fn drop(&mut self) {
        unsafe {
            if self.alive() {
                let _ = RemovePropW(self.hwnd, MARKER);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
unsafe extern "system" fn key(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    composing: usize,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(key), 0x54455854);
    }
    if msg == WM_IME_STARTCOMPOSITION {
        let _ = SetWindowSubclass(hwnd, Some(key), 0x54455854, composing | 1);
    }
    if composing & 1 == 0 && matches!(msg, WM_KEYDOWN | WM_CHAR | WM_KEYUP) && wp.0 == 27 {
        if composing & 4 != 0 {
            if msg == WM_KEYUP || msg == WM_KEYDOWN && lp.0 as usize & (1 << 30) == 0 {
                let _ = SetWindowSubclass(hwnd, Some(key), 0x54455854, composing & !4);
            }
            if msg != WM_KEYDOWN || lp.0 as usize & (1 << 30) != 0 {
                return LRESULT(0);
            }
        }
        // 首个Esc交给真实打开的系统列表；其余边沿仍吞掉，不能再次取消整个编辑层。
        // 只有明确安装为Select的控件才能消费Combo消息，不能向任意EDIT猜测其类型。
        if composing & 2 != 0 && SendMessageW(hwnd, CB_GETDROPPEDSTATE, None, None).0 != 0 {
            if msg == WM_KEYDOWN {
                let _ = SetWindowSubclass(hwnd, Some(key), 0x54455854, composing | 4);
            }
            return DefSubclassProc(hwnd, msg, wp, lp);
        }
        if msg == WM_KEYDOWN && lp.0 as usize & (1 << 30) == 0 {
            if let Ok(parent) = GetParent(hwnd) {
                SendMessageW(parent, WM_CLOSE, None, None);
            }
        }
        return LRESULT(0);
    }
    let result = DefSubclassProc(hwnd, msg, wp, lp);
    if msg == WM_IME_ENDCOMPOSITION && IsWindow(Some(hwnd)).as_bool() {
        let _ = SetWindowSubclass(hwnd, Some(key), 0x54455854, composing & !1);
    }
    result
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let owner = HWND(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut _);
    if !owner.0.is_null() {
        let edit = super::messages::COMPOSITION_START..=super::messages::ENTER_KEY;
        if msg == WM_COMMAND
            || edit.contains(&msg)
            || matches!(msg, WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC)
        {
            return SendMessageW(owner, msg, Some(wp), Some(lp));
        }
        if msg == WM_CLOSE {
            return SendMessageW(
                owner,
                super::CANCEL_EDIT,
                Some(WPARAM(GetPropW(hwnd, EPOCH).0 as usize)),
                Some(LPARAM(hwnd.0 as isize)),
            );
        }
        if msg == WM_ACTIVATE && wp.0 & 0xffff == WA_INACTIVE as usize {
            let next = HWND(lp.0 as *mut _);
            if next != owner && GetAncestor(next, GA_ROOTOWNER) != owner {
                PostMessageW(
                    Some(owner),
                    DISMISS,
                    WPARAM(GetPropW(hwnd, EPOCH).0 as usize),
                    LPARAM(hwnd.0 as isize),
                )
                .ok();
            }
        }
    }
    if msg == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

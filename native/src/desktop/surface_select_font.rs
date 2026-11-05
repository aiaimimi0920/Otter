//! EDIT/Select 共用的 DPI 字体 owner；调用方在默认 NCDESTROY 完成后释放。
use windows::{
    core::{w, Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
};
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}
const KEY: windows::core::PCWSTR = w!("Otter.Surface.Control.Font");
pub(super) struct Font(HFONT);
impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.0 .0));
        }
    }
}
pub(super) unsafe fn take(hwnd: HWND) -> Option<Font> {
    RemovePropW(hwnd, KEY)
        .ok()
        .filter(|h| !h.is_invalid())
        .map(|h| Font(HFONT(h.0)))
}
pub(super) fn update(
    hwnd: HWND,
    dpi: u32,
    style: Option<&crate::surface_presentation::ControlFont>,
) -> Result<()> {
    if !(48..=768).contains(&dpi) {
        return Err(invalid("Select DPI outside supported range"));
    }
    unsafe {
        let mut descriptor = LOGFONTW::default();
        if GetObjectW(
            GetStockObject(DEFAULT_GUI_FONT),
            std::mem::size_of::<LOGFONTW>() as i32,
            Some((&mut descriptor as *mut LOGFONTW).cast()),
        ) == 0
        {
            return Err(Error::from_win32());
        }
        // 从不变的 96 DPI stock 描述计算，避免往返显示器时累计舍入。
        descriptor.lfHeight = ((descriptor.lfHeight as i64 * dpi as i64) / 96) as i32;
        descriptor.lfWidth = ((descriptor.lfWidth as i64 * dpi as i64) / 96) as i32;
        if let Some(style) = style {
            let mut family = [0; 32];
            for (index, unit) in style.family.encode_utf16().enumerate() {
                if index >= family.len() - 1 {
                    return Err(invalid("Select font family exceeds GDI limit"));
                }
                family[index] = unit;
            }
            let height = style.size * dpi as f32 / 96.;
            if style.family.is_empty()
                || style.family.chars().any(char::is_control)
                || !height.is_finite()
                || !(0. ..=32768.).contains(&height)
                || !(100..=900).contains(&style.weight)
            {
                return Err(invalid("Invalid Select resolved font"));
            }
            // GDI 高度为整数像素；0 有系统默认字号语义，不能用于表示微小字号。
            descriptor.lfHeight = -(height.round().max(1.) as i32);
            descriptor.lfWidth = 0;
            descriptor.lfWeight = style.weight as i32;
            descriptor.lfFaceName = family;
        }
        let old = GetPropW(hwnd, KEY);
        let mut previous = LOGFONTW::default();
        if !old.is_invalid()
            && GetObjectW(
                HGDIOBJ(old.0),
                std::mem::size_of::<LOGFONTW>() as i32,
                Some((&mut previous as *mut LOGFONTW).cast()),
            ) != 0
            && previous.lfHeight == descriptor.lfHeight
            && previous.lfWidth == descriptor.lfWidth
            && previous.lfWeight == descriptor.lfWeight
            && previous.lfFaceName == descriptor.lfFaceName
        {
            return Ok(());
        }
        let font = Font(CreateFontIndirectW(&descriptor));
        if font.0.is_invalid() {
            return Err(Error::from_win32());
        }
        SetPropW(hwnd, KEY, Some(HANDLE(font.0 .0)))?;
        let handle = font.0;
        std::mem::forget(font); // WM_NCDESTROY 即使重入也能释放已转交窗口的字体。
        let previous = (!old.is_invalid()).then(|| Font(HFONT(old.0)));
        SendMessageW(
            hwnd,
            WM_SETFONT,
            Some(WPARAM(handle.0 as usize)),
            Some(LPARAM(1)),
        );
        drop(previous); // 控件替换完成前，旧字体仍有效；共享 stock 字体从未归我们所有。
        Ok(())
    }
}

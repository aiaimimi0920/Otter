//! DPI font owner shared by EDIT/Select; callers release after default NCDESTROY finishes.
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
        // Calculate from the immutable 96-DPI stock description, avoiding accumulated rounding across monitor round trips.
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
            // GDI heights are integer pixels; 0 means system-default font size, not a tiny font.
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
        std::mem::forget(font); // WM_NCDESTROY releases the transferred window-owned font even during reentrancy.
        let previous = (!old.is_invalid()).then(|| Font(HFONT(old.0)));
        SendMessageW(
            hwnd,
            WM_SETFONT,
            Some(WPARAM(handle.0 as usize)),
            Some(LPARAM(1)),
        );
        drop(previous); // The old font remains valid until control replacement finishes; shared stock fonts were never ours.
        Ok(())
    }
}

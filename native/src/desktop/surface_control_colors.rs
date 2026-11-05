//! Surface 系统字段共用 Neuro 颜色；借用 DC_BRUSH，不创建或销毁共享 GDI 对象。
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{Accessibility::*, WindowsAndMessaging::*},
};
pub(super) const TEXT: COLORREF = COLORREF(0xeff8f7);
pub(super) const BACKGROUND: COLORREF = COLORREF(0x201711);
pub(super) const HOVER: COLORREF = COLORREF(0x2c2218);
pub(super) const MUTED: COLORREF = COLORREF(0x9f9a92);
pub(super) const FOCUS: COLORREF = COLORREF(0x38ffd9);
pub(super) const FOCUS_SURFACE: COLORREF = COLORREF(0xf1f5f3);
pub(super) const FOCUS_INK: COLORREF = COLORREF(0x2b2520);
pub(super) const FOCUS_MUTED: COLORREF = COLORREF(0x726b65);
pub(super) const LINE: COLORREF = COLORREF(0x3b332e);
pub(super) unsafe fn enabled() -> bool {
    let mut contrast = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    // 无法读取或系统要求高对比度时保留系统绘制，不覆盖用户的辅助功能配色。
    if SystemParametersInfoW(
        SPI_GETHIGHCONTRAST,
        contrast.cbSize,
        Some((&mut contrast as *mut HIGHCONTRASTW).cast()),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    )
    .is_err()
        || contrast.dwFlags.contains(HCF_HIGHCONTRASTON)
    {
        return false;
    }
    true
}
pub(super) unsafe fn paint(dc: HDC) -> Option<LRESULT> {
    if dc.is_invalid() || !enabled() {
        return None;
    }
    SetTextColor(dc, TEXT);
    SetBkColor(dc, BACKGROUND);
    SetDCBrushColor(dc, BACKGROUND);
    Some(LRESULT(GetStockObject(DC_BRUSH).0 as isize))
}

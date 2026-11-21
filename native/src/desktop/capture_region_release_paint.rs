//! 实时选区只画透明键背景、外描边和尺寸；从不复制桌面作为预览。
use super::*;
pub(super) const KEY: COLORREF = COLORREF(0xff00ff);

pub(super) unsafe fn paint(hwnd: HWND, state: &Selection) -> Result<()> {
    let mut paint = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut paint);
    let result = render(hwnd, state, dc, &paint.rcPaint);
    let _ = EndPaint(hwnd, &paint);
    result
}
pub(super) unsafe fn render(hwnd: HWND, state: &Selection, dc: HDC, dirty: &RECT) -> Result<()> {
    if dirty.right <= dirty.left || dirty.bottom <= dirty.top {
        return Ok(());
    }
    if hwnd != state.outline.get() {
        if FillRect(dc, dirty, HBRUSH(GetStockObject(BLACK_BRUSH).0)) == 0 {
            return Err(Error::from_win32());
        }
        return Ok(());
    }
    let bytes = crate::viewport::checked_image_bytes(
        (dirty.right - dirty.left) as u32,
        (dirty.bottom - dirty.top) as u32,
    )
    .ok_or_else(|| Error::new(E_INVALIDARG, "Selection paint exceeds image budget"))?;
    let _lease = crate::pixels::reserve(bytes)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Selection paint budget exhausted"))?;
    let Some(frame) = super::super::super::buffered_paint::Frame::begin(dc, dirty)? else {
        return Ok(());
    };
    SetDCBrushColor(frame.dc, KEY);
    if FillRect(frame.dc, dirty, HBRUSH(GetStockObject(DC_BRUSH).0)) == 0 {
        return Err(Error::from_win32());
    }
    super::super::drawing::decorations(
        frame.dc,
        state.preview(),
        windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).clamp(96, 768),
        false,
        true,
    )?;
    frame.commit()
}

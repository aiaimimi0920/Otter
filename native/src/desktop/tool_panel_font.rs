//! 面板拥有一个 DPI 字体；属性只保存句柄，替换/销毁负责释放，绘制仅借用。
use windows::{
    core::{w, Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
};
const KEY: windows::core::PCWSTR = w!("Otter.ToolPanel.Font");
struct OwnedFont(HFONT);
impl Drop for OwnedFont {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.0 .0));
        }
    }
}
fn height(dpi: u32) -> i32 {
    -(((13 * dpi.max(96) as u64 + 48) / 96).min(i32::MAX as u64) as i32)
}
pub(in crate::desktop) unsafe fn update(hwnd: HWND, dpi: u32) -> Result<()> {
    let old = GetPropW(hwnd, KEY);
    let mut previous = LOGFONTW::default();
    if !old.is_invalid()
        && GetObjectW(
            HGDIOBJ(old.0),
            std::mem::size_of::<LOGFONTW>() as i32,
            Some((&mut previous as *mut LOGFONTW).cast()),
        ) > 0
        && previous.lfHeight == height(dpi)
    {
        return Ok(());
    }
    let mut descriptor = LOGFONTW {
        lfHeight: height(dpi),
        lfWeight: FW_NORMAL.0 as i32,
        lfCharSet: DEFAULT_CHARSET,
        lfQuality: CLEARTYPE_QUALITY,
        ..Default::default()
    };
    for (destination, unit) in descriptor
        .lfFaceName
        .iter_mut()
        .zip("Microsoft YaHei UI".encode_utf16())
    {
        *destination = unit;
    }
    let font = OwnedFont(CreateFontIndirectW(&descriptor));
    if font.0.is_invalid() {
        return Err(Error::from_win32());
    }
    SetPropW(hwnd, KEY, Some(HANDLE(font.0 .0)))?;
    std::mem::forget(font); // 从这里开始由窗口属性 owner 负责释放。
    if !old.is_invalid() {
        let _ = DeleteObject(HGDIOBJ(old.0));
    }
    Ok(())
}
pub(in crate::desktop) unsafe fn object(panel: HWND) -> HGDIOBJ {
    let font = GetPropW(panel, KEY);
    if font.is_invalid() {
        GetStockObject(DEFAULT_GUI_FONT)
    } else {
        HGDIOBJ(font.0)
    }
}
pub(in crate::desktop) unsafe fn release(hwnd: HWND) {
    if let Ok(font) = RemovePropW(hwnd, KEY) {
        let _ = DeleteObject(HGDIOBJ(font.0));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct MemoryDc(HDC, HGDIOBJ);
    impl Drop for MemoryDc {
        fn drop(&mut self) {
            unsafe {
                SelectObject(self.0, self.1);
                let _ = DeleteDC(self.0);
            }
        }
    }
    struct PropertyCleanup(HWND);
    impl Drop for PropertyCleanup {
        fn drop(&mut self) {
            unsafe {
                release(self.0);
            }
        }
    }
    #[test]
    fn actual_gdi_labels_fit_tool_cells_at_supported_dpi_scales() {
        unsafe {
            let window = crate::desktop::Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("font layout fixture"),
                    WS_POPUP,
                    0,
                    0,
                    1,
                    1,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            let _cleanup = PropertyCleanup(window.0);
            let labels: Vec<_> = crate::desktop::tool_commands::TOOLS
                .iter()
                .map(|entry| format!("✓ {}", entry.label.split('\t').next().unwrap()))
                .chain(
                    crate::desktop::tool_panel_actions::ACTIONS
                        .iter()
                        .map(|entry| entry.1.to_owned()),
                )
                .chain([
                    "粗细 + 256.00".into(),
                    "透明度 100%".into(),
                    "填充 开".into(),
                ])
                .collect();
            for dpi in [96, 144, 192] {
                update(window.0, dpi).unwrap();
                let dc = CreateCompatibleDC(None);
                assert!(!dc.is_invalid());
                let _dc = MemoryDc(dc, SelectObject(dc, object(window.0)));
                let rect = super::super::geometry::button(0, 19, dpi);
                for label in &labels {
                    let mut size = SIZE::default();
                    let text: Vec<_> = label.encode_utf16().collect();
                    GetTextExtentPoint32W(dc, &text, &mut size).ok().unwrap();
                    assert!(
                        size.cx <= rect.right - rect.left - 4,
                        "DPI={dpi} label={label} width={}",
                        size.cx
                    );
                    assert!(
                        size.cy <= rect.bottom - rect.top - 4,
                        "DPI={dpi} label={label} height={}",
                        size.cy
                    );
                }
            }
        }
    }
    #[test]
    fn dpi_font_reuses_matching_height_and_releases_replaced_handles() {
        unsafe {
            let window = crate::desktop::Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("font fixture"),
                    WS_POPUP,
                    0,
                    0,
                    1,
                    1,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            update(window.0, 96).unwrap();
            let first = object(window.0);
            update(window.0, 96).unwrap();
            assert_eq!(object(window.0), first);
            update(window.0, 192).unwrap();
            let second = object(window.0);
            assert_ne!(second, first);
            let mut descriptor = LOGFONTW::default();
            assert_eq!(
                GetObjectW(
                    first,
                    std::mem::size_of::<LOGFONTW>() as i32,
                    Some((&mut descriptor as *mut LOGFONTW).cast())
                ),
                0
            );
            assert!(
                GetObjectW(
                    second,
                    std::mem::size_of::<LOGFONTW>() as i32,
                    Some((&mut descriptor as *mut LOGFONTW).cast())
                ) > 0
            );
            assert_eq!(descriptor.lfHeight, -26);
            release(window.0);
            assert!(GetPropW(window.0, KEY).is_invalid());
            assert_eq!(
                GetObjectW(
                    second,
                    std::mem::size_of::<LOGFONTW>() as i32,
                    Some((&mut descriptor as *mut LOGFONTW).cast())
                ),
                0
            );
            release(window.0); // 清理幂等，不释放系统默认字体。
        }
    }
    #[test]
    fn font_height_scales_and_rounds_without_overflow() {
        assert_eq!(
            [0, 96, 144, 192, 240].map(height),
            [-13, -13, -20, -26, -33]
        );
        assert!(height(u32::MAX) < 0);
    }
}

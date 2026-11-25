//! Panels own one DPI font; properties store only handles, replacement/destruction releases them, and drawing borrows.
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
    std::mem::forget(font); // The window-property owner is responsible for release from here.
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
            let mut labels = Vec::new();
            for language in crate::ui_text::DisplayLanguage::ALL {
                labels.extend(crate::desktop::tool_commands::TOOLS.iter().map(|entry| {
                    let label = crate::ui_text::text_for(language, entry.label_id);
                    format!("✓ {}", label.split('\t').next().unwrap())
                }));
                labels.extend(
                    crate::desktop::tool_panel_actions::ACTIONS
                        .iter()
                        .map(|entry| crate::ui_text::text_for(language, entry.1).to_owned()),
                );
                labels.push(crate::ui_text::format_for(
                    language,
                    "tool-action-width-up-value",
                    &[("width", "256.00".into())],
                ));
                labels.push(crate::ui_text::format_for(
                    language,
                    "tool-action-opacity-value",
                    &[("opacity", "100".into())],
                ));
                labels.push(crate::ui_text::text_for(language, "tool-action-fill-on").to_owned());
            }
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
            release(window.0); // Cleanup is idempotent and never releases the system-default font.
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

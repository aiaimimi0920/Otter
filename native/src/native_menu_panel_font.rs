//! 每个面板拥有两种 DPI 字体；先恢复绘制 DC，再由面板生命周期释放字体。
use super::*;
struct Font(HFONT);
impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.0 .0));
        }
    }
}
pub(super) struct Fonts {
    normal: Font,
    bold: Font,
}
pub(super) fn scaled(value: i32, dpi: u32) -> i32 {
    ((i64::from(value) * i64::from(dpi.clamp(48, 768)) + 48) / 96) as i32
}
fn create(dpi: u32, weight: FONT_WEIGHT) -> Result<Font> {
    let mut descriptor = LOGFONTW {
        lfHeight: -scaled(13, dpi),
        lfWeight: weight.0 as i32,
        lfCharSet: DEFAULT_CHARSET,
        lfQuality: CLEARTYPE_QUALITY,
        ..Default::default()
    };
    for (target, value) in descriptor
        .lfFaceName
        .iter_mut()
        .zip("Microsoft YaHei UI".encode_utf16())
    {
        *target = value;
    }
    let font = unsafe { CreateFontIndirectW(&descriptor) };
    if font.is_invalid() {
        Err(Error::from_win32())
    } else {
        Ok(Font(font))
    }
}
impl Fonts {
    pub(super) fn new(dpi: u32) -> Result<Self> {
        Ok(Self {
            normal: create(dpi, FW_NORMAL)?,
            bold: create(dpi, FW_BOLD)?,
        })
    }
    pub(super) fn object(&self, bold: bool) -> HGDIOBJ {
        HGDIOBJ(if bold {
            self.bold.0 .0
        } else {
            self.normal.0 .0
        })
    }
}

struct MeasureDc(HWND, HDC, i32);
impl Drop for MeasureDc {
    fn drop(&mut self) {
        unsafe {
            let _ = RestoreDC(self.1, self.2);
            ReleaseDC(Some(self.0), self.1);
        }
    }
}
pub(super) fn width(owner: HWND, rows: &[Row]) -> Result<i32> {
    if rows.len() > 1024
        || rows
            .iter()
            .map(|row| row.label.encode_utf16().count())
            .sum::<usize>()
            > 128 * 1024
    {
        return Err(invalid());
    }
    let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(owner) }.clamp(48, 768);
    let fonts = Fonts::new(dpi)?;
    let dc = unsafe { GetDC(Some(owner)) };
    if dc.is_invalid() {
        return Err(Error::from_win32());
    }
    let saved = unsafe { SaveDC(dc) };
    if saved == 0 {
        unsafe {
            ReleaseDC(Some(owner), dc);
        }
        return Err(Error::from_win32());
    }
    let _dc = MeasureDc(owner, dc, saved);
    let mut label_width = 0;
    let mut shortcut_width = 0;
    for row in rows.iter().filter(|row| !row.separator) {
        let (label, shortcut) = row.label.split_once('\t').unwrap_or((&row.label, ""));
        for (value, bold, flags, target) in [
            (label, row.default, DT_LEFT, &mut label_width),
            (shortcut, false, DT_NOPREFIX, &mut shortcut_width),
        ] {
            let mut rect = RECT::default();
            let mut units: Vec<u16> = value.encode_utf16().collect();
            unsafe {
                SelectObject(dc, fonts.object(bold));
                if !units.is_empty()
                    && DrawTextW(
                        dc,
                        &mut units,
                        &mut rect,
                        DT_CALCRECT | DT_SINGLELINE | flags,
                    ) == 0
                {
                    return Err(Error::new(E_FAIL, "Cannot measure native menu label"));
                }
            }
            *target = (*target).max(rect.right - rect.left);
        }
    }
    Ok((label_width + shortcut_width + scaled(72, dpi)).max(scaled(160, dpi)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_fonts_follow_dpi_weight_and_release_owned_handles() {
        for dpi in [96, 144, 192, 288] {
            let fonts = Fonts::new(dpi).unwrap();
            let handles = [fonts.object(false), fonts.object(true)];
            for (index, handle) in handles.iter().enumerate() {
                let mut descriptor = LOGFONTW::default();
                assert!(
                    unsafe {
                        GetObjectW(
                            *handle,
                            std::mem::size_of::<LOGFONTW>() as i32,
                            Some((&mut descriptor as *mut LOGFONTW).cast()),
                        )
                    } > 0
                );
                assert_eq!(descriptor.lfHeight, -scaled(13, dpi));
                assert_eq!(descriptor.lfWeight, if index == 0 { 400 } else { 700 });
            }
            drop(fonts);
            for handle in handles {
                // NULL 查询只返回类型所需大小，不能证明句柄仍存活；必须实际读取。
                let mut descriptor = LOGFONTW::default();
                assert_eq!(
                    unsafe {
                        GetObjectW(
                            handle,
                            std::mem::size_of::<LOGFONTW>() as i32,
                            Some((&mut descriptor as *mut LOGFONTW).cast()),
                        )
                    },
                    0
                );
            }
        }
    }
}

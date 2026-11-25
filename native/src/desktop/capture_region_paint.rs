//! 冻结图与选区装饰先离屏完成再提交；复用 main 的信号黄外描边和尺寸标签。
use super::{Mode, Selection};
use crate::desktop::{buffered_paint, image::Image};
use windows::{
    core::{w, Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::HiDpi::GetDpiForWindow},
};

const SIGNAL: COLORREF = COLORREF(0x38ffd9); // main: #d9ff38
const PANEL: COLORREF = COLORREF(0x18120e); // main: #0e1218

pub(super) unsafe fn paint(hwnd: HWND, state: &Selection<'_>, dc: HDC, dirty: &RECT) -> Result<()> {
    if dirty.right <= dirty.left || dirty.bottom <= dirty.top {
        return Ok(());
    }
    let _lease = crate::pixels::reserve(state.image.pixels.len())
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Capture paint buffer budget exhausted"))?;
    let Some(frame) = buffered_paint::Frame::begin(dc, dirty)? else {
        return Ok(());
    };
    render(
        frame.dc,
        state.image,
        state.preview_rect(),
        GetDpiForWindow(hwnd).clamp(96, 768),
        state.mode != Mode::Quick,
    )?;
    frame.commit()
}

unsafe fn render(
    dc: HDC,
    image: &Image,
    selection: Option<RECT>,
    dpi: u32,
    handles: bool,
) -> Result<()> {
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: image.width as i32,
            biHeight: -(image.height as i32),
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    if SetDIBitsToDevice(
        dc,
        0,
        0,
        image.width,
        image.height,
        0,
        0,
        0,
        image.height,
        image.pixels.as_ptr().cast(),
        &info,
        DIB_RGB_COLORS,
    ) == 0
    {
        return Err(Error::new(E_FAIL, "Cannot draw frozen capture"));
    }
    decorations(dc, selection, dpi, handles, false)
}

pub(super) unsafe fn decorations(
    dc: HDC,
    selection: Option<RECT>,
    dpi: u32,
    handles: bool,
    opaque_label: bool,
) -> Result<()> {
    if let Some(rect) = selection {
        let scale = |value: i32| (value * dpi as i32 + 48) / 96;
        let width = scale(2);
        // 实线完全在量取区域外，不改变截取尺寸或覆盖选区内的边缘像素。
        for edge in edges(rect, width) {
            fill(dc, &edge, SIGNAL)?;
        }
        if handles {
            let half = scale(3);
            for point in crate::desktop::capture_selection::handles(rect) {
                let handle = RECT {
                    left: point.x - half,
                    top: point.y - half,
                    right: point.x + half + 1,
                    bottom: point.y + half + 1,
                };
                if FrameRect(dc, &handle, HBRUSH(GetStockObject(WHITE_BRUSH).0)) == 0 {
                    return Err(Error::from_win32());
                }
            }
        }
        label(dc, rect, dpi, opaque_label)?;
    }
    Ok(())
}

fn edges(rect: RECT, width: i32) -> [RECT; 4] {
    [
        RECT {
            left: rect.left - width,
            top: rect.top - width,
            right: rect.right + width,
            bottom: rect.top,
        },
        RECT {
            left: rect.left - width,
            top: rect.bottom,
            right: rect.right + width,
            bottom: rect.bottom + width,
        },
        RECT {
            left: rect.left - width,
            right: rect.left,
            ..rect
        },
        RECT {
            left: rect.right,
            right: rect.right + width,
            ..rect
        },
    ]
}
unsafe fn fill(dc: HDC, rect: &RECT, color: COLORREF) -> Result<()> {
    SetDCBrushColor(dc, color);
    if FillRect(dc, rect, HBRUSH(GetStockObject(DC_BRUSH).0)) == 0 {
        return Err(Error::from_win32());
    }
    Ok(())
}
struct Font(HFONT);
impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}
struct DcState(HDC, i32);
impl Drop for DcState {
    fn drop(&mut self) {
        unsafe {
            let _ = RestoreDC(self.0, self.1);
        }
    }
}

fn label_text(rect: RECT) -> String {
    format!("{} x {}", rect.right - rect.left, rect.bottom - rect.top)
}
unsafe fn label(dc: HDC, rect: RECT, dpi: u32, opaque: bool) -> Result<()> {
    let scale = |value: i32| (value * dpi as i32 + 48) / 96;
    let font = Font(CreateFontW(
        -scale(11),
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        OUT_DEFAULT_PRECIS,
        CLIP_DEFAULT_PRECIS,
        ANTIALIASED_QUALITY,
        (FIXED_PITCH.0 | FF_MODERN.0) as u32,
        w!("Consolas"),
    ));
    if font.0.is_invalid() {
        return Err(Error::from_win32());
    }
    let saved = SaveDC(dc);
    if saved == 0 {
        return Err(Error::from_win32());
    }
    let _saved = DcState(dc, saved); // 先恢复 DC，之后才释放字体。
    if SelectObject(dc, font.0.into()).is_invalid() {
        return Err(Error::from_win32());
    }
    let mut text: Vec<u16> = label_text(rect).encode_utf16().collect();
    let mut measured = RECT::default();
    if DrawTextW(
        dc,
        &mut text,
        &mut measured,
        DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
    ) == 0
    {
        return Err(Error::from_win32());
    }
    let panel = RECT {
        left: rect.left,
        top: rect.top - scale(28),
        right: rect.left + measured.right + scale(18),
        bottom: rect.top - scale(28) + scale(26),
    };
    // 一像素色块缩放混合，只改变标签背景，不修改冻结图或重新采集桌面。
    let tint = (!opaque)
        .then(|| {
            crate::software_dib::Bitmap::new(
                windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
                    width: 1,
                    height: 1,
                },
            )
        })
        .transpose()?;
    if let Some(tint) = &tint {
        blend(dc, tint.dc(), panel, PANEL, 245)?;
    } else {
        fill(dc, &panel, PANEL)?;
    }
    let line = scale(1);
    let inner = RECT {
        left: panel.left + line,
        top: panel.top + line,
        right: panel.right - line,
        bottom: panel.bottom - line,
    };
    for edge in edges(inner, line) {
        if let Some(tint) = &tint {
            blend(dc, tint.dc(), edge, SIGNAL, 97)?;
        } else {
            fill(dc, &edge, SIGNAL)?;
        }
    }
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, SIGNAL);
    let mut content = RECT {
        left: panel.left + scale(9),
        top: panel.top + scale(5),
        right: panel.right - scale(9),
        bottom: panel.bottom - scale(5),
    };
    if DrawTextW(
        dc,
        &mut text,
        &mut content,
        DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
    ) == 0
    {
        return Err(Error::from_win32());
    }
    Ok(())
}
unsafe fn blend(dc: HDC, source: HDC, rect: RECT, color: COLORREF, alpha: u8) -> Result<()> {
    fill(
        source,
        &RECT {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
        color,
    )?;
    AlphaBlend(
        dc,
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
        source,
        0,
        0,
        1,
        1,
        BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: alpha,
            AlphaFormat: 0,
        },
    )
    .ok()
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_region_paint_tests.rs"]
mod tests;

//! Complete frozen-image and selection-decoration drawing offscreen before submission; reuse main's signal-yellow outer stroke and size label.
use super::{Mode, Selection};
use crate::desktop::{buffered_paint, image::Image};
use windows::{
    core::{w, Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::HiDpi::GetDpiForWindow},
};

const SIGNAL: COLORREF = COLORREF(0x38ffd9); // main: #d9ff38
const PANEL: COLORREF = COLORREF(0x18120e); // main: #0e1218

pub(super) fn decoration_bounds(rect: RECT, dpi: u32) -> RECT {
    let scale = |value: i32| (value * dpi as i32 + 48) / 96;
    let pad = scale(6);
    RECT {
        left: rect.left.saturating_sub(pad),
        top: rect.top.saturating_sub(scale(32)),
        right: rect
            .right
            .saturating_add(pad)
            .max(rect.left.saturating_add(scale(320))),
        bottom: rect.bottom.saturating_add(pad),
    }
}

pub(super) unsafe fn invalidate_selection_change(
    hwnd: HWND,
    before: Option<RECT>,
    after: Option<RECT>,
    dpi: u32,
) {
    match (
        before.map(|r| decoration_bounds(r, dpi)),
        after.map(|r| decoration_bounds(r, dpi)),
    ) {
        (Some(a), Some(b)) => {
            let _ = InvalidateRect(Some(hwnd), Some(&a), false);
            let _ = InvalidateRect(Some(hwnd), Some(&b), false);
        }
        (Some(r), None) | (None, Some(r)) => {
            let _ = InvalidateRect(Some(hwnd), Some(&r), false);
        }
        (None, None) => {}
    }
}

pub(super) unsafe fn paint(hwnd: HWND, state: &Selection<'_>, dc: HDC, dirty: &RECT) -> Result<()> {
    if dirty.right <= dirty.left || dirty.bottom <= dirty.top {
        return Ok(());
    }
    // Performance audit optimization (Tier 2.5):
    // Reserve pixel budget only for the `dirty` update rectangle rather than the entire 4K/dual-4K frozen desktop
    // (`31.6–66.4 MiB`), matching `buffered_paint::Frame::begin(dc, dirty)`'s actual offscreen surface size.
    let dirty_bytes = crate::viewport::checked_image_bytes(
        (dirty.right - dirty.left) as u32,
        (dirty.bottom - dirty.top) as u32,
    )
    .ok_or_else(|| Error::new(E_INVALIDARG, "Capture dirty region exceeds image budget"))?;
    let _lease = crate::pixels::reserve(dirty_bytes)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Capture paint buffer budget exhausted"))?;
    let Some(frame) = buffered_paint::Frame::begin(dc, dirty)? else {
        return Ok(());
    };
    render_clipped(
        frame.dc,
        state.image,
        Some(*dirty),
        state.preview_rect(),
        GetDpiForWindow(hwnd).clamp(96, 768),
        state.mode != Mode::Quick,
    )?;
    frame.commit()
}

#[cfg_attr(not(test), allow(dead_code))]
unsafe fn render(
    dc: HDC,
    image: &Image,
    selection: Option<RECT>,
    dpi: u32,
    handles: bool,
) -> Result<()> {
    render_clipped(dc, image, None, selection, dpi, handles)
}

unsafe fn render_clipped(
    dc: HDC,
    image: &Image,
    dirty: Option<RECT>,
    selection: Option<RECT>,
    dpi: u32,
    handles: bool,
) -> Result<()> {
    let height = image.height as i32;
    let width = image.width as i32;
    let (top, bottom) = match dirty {
        Some(rect) => (rect.top.clamp(0, height), rect.bottom.clamp(0, height)),
        None => (0, height),
    };
    if top < bottom && width > 0 {
        let rows = (bottom - top) as u32;
        let stride = image.width as usize * 4;
        let slice = &image.pixels[top as usize * stride..bottom as usize * stride];
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -(rows as i32),
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        // Performance audit optimization (Tier 2.5):
        // Upload only the `[top..bottom]` scanline band intersecting `dirty` instead of pushing all 2,160+ rows of the
        // frozen desktop on every pointer move.
        if SetDIBitsToDevice(
            dc,
            0,
            top,
            image.width,
            rows,
            0,
            0,
            0,
            rows,
            slice.as_ptr().cast(),
            &info,
            DIB_RGB_COLORS,
        ) == 0
        {
            return Err(Error::new(E_FAIL, "Cannot draw frozen capture"));
        }
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
        // The solid stroke lies entirely outside the measured region, preserving capture dimensions and edge pixels inside the selection.
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

std::thread_local! {
    static CACHED_FONT: std::cell::RefCell<Option<(u32, Font)>> = const { std::cell::RefCell::new(None) };
    static CACHED_TINT: std::cell::RefCell<Option<crate::software_dib::Bitmap>> = const { std::cell::RefCell::new(None) };
}

pub(super) fn clear_cached_resources() {
    CACHED_FONT.with(|slot| {
        slot.borrow_mut().take();
    });
    CACHED_TINT.with(|slot| {
        slot.borrow_mut().take();
    });
}

fn label_text(rect: RECT) -> String {
    format!(
        "{} x {}  {}",
        rect.right - rect.left,
        rect.bottom - rect.top,
        crate::ui_text::text("capture-size-more")
    )
}
unsafe fn label(dc: HDC, rect: RECT, dpi: u32, opaque: bool) -> Result<()> {
    let scale = |value: i32| (value * dpi as i32 + 48) / 96;
    // Reuse the DPI-matched UI HFONT and 1x1 tint DIB across WM_PAINT frames during a capture session
    // instead of creating and destroying GDI fonts/DIBs/DCs on every 240Hz mouse move; `clear_cached_resources()`
    // releases both handles deterministically when the capture modal exits.
    let font_handle = CACHED_FONT.with(|slot| -> Result<HFONT> {
        let mut guard = slot.borrow_mut();
        if let Some((cached_dpi, font)) = guard.as_ref() {
            if *cached_dpi == dpi {
                return Ok(font.0);
            }
        }
        let created = Font(CreateFontW(
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
            (VARIABLE_PITCH.0 | FF_SWISS.0) as u32,
            w!("Microsoft YaHei UI"),
        ));
        if created.0.is_invalid() {
            return Err(Error::from_win32());
        }
        let handle = created.0;
        *guard = Some((dpi, created));
        Ok(handle)
    })?;
    let saved = SaveDC(dc);
    if saved == 0 {
        return Err(Error::from_win32());
    }
    let _saved = DcState(dc, saved); // Restore the DC before releasing fonts.
    if SelectObject(dc, font_handle.into()).is_invalid() {
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
    // Blend a scaled one-pixel color block into only the label background; do not alter the frozen image or recapture the desktop.
    let tint_dc = if !opaque {
        Some(CACHED_TINT.with(|slot| -> Result<HDC> {
            let mut guard = slot.borrow_mut();
            if guard.is_none() {
                *guard = Some(crate::software_dib::Bitmap::new(
                    windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
                        width: 1,
                        height: 1,
                    },
                )?);
            }
            Ok(guard.as_ref().unwrap().dc())
        })?)
    } else {
        None
    };
    if let Some(tint_dc) = tint_dc {
        blend(dc, tint_dc, panel, PANEL, 245)?;
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
        if let Some(tint_dc) = tint_dc {
            blend(dc, tint_dc, edge, SIGNAL, 97)?;
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

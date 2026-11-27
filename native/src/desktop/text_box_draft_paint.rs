//! Paint glyphs, selection and caret from one annotation layout. A DC target
//! never outlives its paint DC; the model never owns an HWND or GDI resource.
use super::*;
use windows::Win32::Graphics::{
    Direct2D::{Common::*, *},
    Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
};

fn rgba(c: COLORREF) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: (c.0 & 255) as f32 / 255.,
        g: ((c.0 >> 8) & 255) as f32 / 255.,
        b: ((c.0 >> 16) & 255) as f32 / 255.,
        a: 1.,
    }
}
unsafe fn render(v: &View, dc: HDC) -> Result<()> {
    let mut rc = RECT::default();
    GetClientRect(v.cover, &mut rc)?;
    if rc.right <= 0 || rc.bottom <= 0 {
        return Ok(());
    }
    let popup = GetParent(v.edit)?;
    let (ink, bg) = paint::colors(popup);
    SetDCBrushColor(dc, bg);
    FillRect(dc, &rc, HBRUSH(GetStockObject(DC_BRUSH).0));
    let Some(m) = v.frame.borrow().clone() else {
        return Err(Error::new(E_FAIL, "No boxed draft layout"));
    };
    let ([ox, oy], [cx, cy, ch]) = input::geometry(v, &m)?;
    let (a, b) = v.selected(&m);
    let _scratch = crate::viewport::checked_image_bytes(rc.right as u32, rc.bottom as u32)
        .and_then(|n| n.checked_mul(2))
        .and_then(crate::pixels::reserve)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Boxed draft paint budget exhausted"))?;
    let t = v
        .factory
        .CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_IGNORE,
            },
            dpiX: 96.,
            dpiY: 96.,
            ..Default::default()
        })?;
    t.BindDC(dc, &rc)?;
    t.SetTextAntialiasMode(if v.keyed {
        D2D1_TEXT_ANTIALIAS_MODE_ALIASED
    } else {
        D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE
    });
    let brush = t.CreateSolidColorBrush(&rgba(ink), None)?;
    let mut matrix = Default::default();
    t.GetTransform(&mut matrix);
    matrix.M11 = v.scale;
    matrix.M22 = v.scale;
    matrix.M31 = ox * v.scale;
    matrix.M32 = oy * v.scale;
    t.SetTransform(&matrix);
    desktop::render_frame::draw(&t, || {
        t.Clear(Some(&rgba(bg)));
        if a != b {
            brush.SetColor(&rgba(COLORREF(GetSysColor(COLOR_HIGHLIGHT))));
            for r in m.layout.selection_rects(a..b)? {
                t.FillRectangle(
                    &D2D_RECT_F {
                        left: r.left,
                        top: r.top,
                        right: r.right,
                        bottom: r.bottom,
                    },
                    &brush,
                );
            }
        }
        brush.SetColor(&rgba(ink));
        m.layout.draw(&t, &brush, [0., 0.])?;
        if a == b && GetFocus() == v.edit {
            brush.SetColor(&rgba(COLORREF(GetSysColor(COLOR_WINDOWTEXT))));
            t.FillRectangle(
                &D2D_RECT_F {
                    left: cx,
                    top: cy,
                    right: cx + 1. / v.scale,
                    bottom: cy + ch,
                },
                &brush,
            );
        }
        Ok(())
    })
}
pub(super) unsafe fn paint(v: &View, msg: u32, wp: WPARAM) -> LRESULT {
    let mut ps = PAINTSTRUCT::default();
    let dc = if msg == WM_PRINTCLIENT {
        HDC(wp.0 as *mut _)
    } else {
        BeginPaint(v.cover, &mut ps)
    };
    if !dc.is_invalid() {
        if let Err(e) = render(v, dc) {
            eprintln!("native-boxed-text-paint: {e}");
            let mut rect = RECT::default();
            if GetClientRect(v.cover, &mut rect).is_ok() {
                let mut text: Vec<_> = crate::ui_text::format(
                    "action-text-draft-unavailable",
                    &[("error", e.to_string().into())],
                )
                .encode_utf16()
                .collect();
                SetTextColor(dc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
                DrawTextW(dc, &mut text, &mut rect, DT_WORDBREAK | DT_NOPREFIX);
            }
        }
    }
    if msg == WM_PAINT {
        let _ = EndPaint(v.cover, &ps);
    }
    LRESULT(0)
}

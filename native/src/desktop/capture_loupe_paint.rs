//! Paint an immutable grid snapshot. Invalid monitor pixels are visibly distinct from sampled black.
use super::*;
use crate::capture_loupe_layout::scale;
pub(super) struct Picture {
    pub window: Cell<HWND>,
    pub grid: Cell<Grid>,
    pub format: Cell<Format>,
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[test]
fn loupe_gdi_grid_preserves_center_color_and_draws_contrasting_edges() {
    unsafe {
        let owner = Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                20,
                20,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let hud = Hud::new(owner.0).unwrap();
        let hwnd = hud.hwnd();
        let dpi = GetDpiForWindow(hwnd);
        let s = |v| scale(v, dpi);
        let dib = crate::software_dib::Bitmap::new(
            windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U {
                width: s(168) as u32,
                height: s(204) as u32,
            },
        )
        .unwrap();
        for (rgb, ink) in [([250, 250, 250], 0), ([1, 2, 3], 0xffffff)] {
            let mut grid = [None; 225];
            grid[112] = Some(rgb);
            hud.picture.grid.set(grid);
            render(dib.dc(), hwnd, &hud.picture);
            GdiFlush().ok().unwrap();
            let at = |x, y| GetPixel(dib.dc(), s(x), s(y)).0;
            assert_eq!(at(84, 68), u32::from_le_bytes([rgb[0], rgb[1], rgb[2], 0]));
            assert_eq!(at(80, 68), ink);
            assert_eq!(at(79, 68), ink ^ 0xffffff);
            assert_ne!(at(26, 10), at(34, 10));
        }
        drop(owner);
        assert!(hud.hwnd().is_invalid());
    }
}
impl Default for Picture {
    fn default() -> Self {
        Self {
            window: Cell::default(),
            grid: Cell::new([None; 225]),
            format: Cell::default(),
        }
    }
}
const ID: usize = 0x434c50;
pub(super) unsafe fn attach(hwnd: HWND, picture: Rc<Picture>) -> Result<()> {
    let ptr = Rc::into_raw(picture);
    if !SetWindowSubclass(hwnd, Some(procedure), ID, ptr as usize).as_bool() {
        drop(Rc::from_raw(ptr));
        return Err(Error::from_win32());
    }
    Ok(())
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            if !dc.is_invalid() {
                render(dc, hwnd, &*(data as *const Picture));
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_NCDESTROY => {
            let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
            Rc::from_raw(data as *const Picture)
                .window
                .set(HWND::default());
            DefSubclassProc(hwnd, msg, wp, lp)
        }
        _ => DefSubclassProc(hwnd, msg, wp, lp),
    }
}
unsafe fn render(dc: HDC, hwnd: HWND, p: &Picture) {
    let dpi = GetDpiForWindow(hwnd);
    let s = |v| scale(v, dpi);
    let brush = HBRUSH(GetStockObject(DC_BRUSH).0);
    let area = |l, t, r, b| RECT {
        left: s(l),
        top: s(t),
        right: s(r),
        bottom: s(b),
    };
    let fill = |rect: RECT, rgb: u32| {
        SetDCBrushColor(dc, COLORREF(rgb));
        FillRect(dc, &rect, brush);
    };
    let grid = p.grid.get();
    fill(area(0, 0, 168, 204), 0x282421);
    for (i, c) in grid.iter().enumerate() {
        let x = (i % 15) as i32;
        let y = (i / 15) as i32;
        let rgb = c.map_or(
            if (x + y) % 2 == 0 { 0x686868 } else { 0xa0a0a0 },
            |[r, g, b]| u32::from_le_bytes([r, g, b, 0]),
        );
        fill(area(24 + x * 8, 8 + y * 8, 32 + x * 8, 16 + y * 8), rgb);
    }
    let center = grid[112];
    let ink = if center.is_some_and(crate::color_format::use_black_cross) {
        0
    } else {
        0xffffff
    };
    // Double outline leaves the center pixel itself visible.
    for (inset, color) in [(-1, ink ^ 0xffffff), (0, ink)] {
        SetDCBrushColor(dc, COLORREF(color));
        FrameRect(
            dc,
            &area(80 + inset, 64 + inset, 88 - inset, 72 - inset),
            brush,
        );
    }
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(0xffffff));
    let format = p.format.get();
    let header = match format {
        Format::Hex => "capture-loupe-hex",
        Format::Rgb => "capture-loupe-rgb",
        Format::Hsl => "capture-loupe-hsl",
    };
    for (y, text) in [
        (131, crate::ui_text::text(header).to_string()),
        (
            156,
            center.map_or_else(|| "—".into(), |c| format.describe(c)),
        ),
        (181, crate::ui_text::text("capture-loupe-keys").into()),
    ] {
        let mut units: Vec<_> = text.encode_utf16().collect();
        DrawTextW(
            dc,
            &mut units,
            &mut area(1, y, 167, y + 22),
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
}

//! Native EDIT paints over a color key; a separate input-transparent layer carries fractional fill.
use super::*;
use crate::stroke_style::{Color, StrokeStyle};

const INK: PCWSTR = w!("Otter.Annotation.Inline.Ink");
const BACKGROUND: PCWSTR = w!("Otter.Annotation.Inline.Background");
const FILL_ONLY: PCWSTR = w!("Otter.Annotation.Inline.FillOnly");

fn rgb(Color(r, g, b): Color) -> COLORREF {
    COLORREF(u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16)
}
fn alpha(value: f32) -> u8 {
    (value * 255.0).round() as u8
}
pub(super) struct Style {
    ink: COLORREF,
    background: COLORREF,
    ink_alpha: u8,
    fill: Option<(COLORREF, u8)>,
    keyed: bool,
}
impl Style {
    pub(super) unsafe fn new(style: StrokeStyle) -> Self {
        let ink = rgb(style.color());
        let ink_alpha = alpha(style.paint_alpha().stroke() * style.opacity());
        let fill_alpha = alpha(style.paint_alpha().fill() * style.opacity());
        let fill = style
            .fill()
            .filter(|_| fill_alpha > 0)
            .map(|color| (rgb(color), fill_alpha));
        let keyed = ink_alpha != 255 || !matches!(fill, Some((_, 255)));
        let background = if keyed {
            // Avoid keying out either user-selected ink or a system selection/caret color.
            let avoid = [
                ink.0,
                GetSysColor(COLOR_HIGHLIGHT),
                GetSysColor(COLOR_HIGHLIGHTTEXT),
                GetSysColor(COLOR_WINDOWTEXT),
            ];
            COLORREF((0x00fe01..).find(|c| !avoid.contains(c)).unwrap())
        } else {
            fill.unwrap().0
        };
        Self {
            ink,
            background,
            ink_alpha,
            fill,
            keyed,
        }
    }
    pub(super) fn keyed(&self) -> bool {
        self.keyed
    }
    pub(super) unsafe fn install(&self, hwnd: HWND) -> Result<()> {
        // Zero alpha ink is the key itself; keep selection/caret and border visible.
        set_color(
            hwnd,
            INK,
            if self.ink_alpha == 0 {
                self.background
            } else {
                self.ink
            },
        )?;
        set_color(hwnd, BACKGROUND, self.background)?;
        if self.keyed {
            SetLayeredWindowAttributes(
                hwnd,
                self.background,
                if self.ink_alpha == 0 {
                    255
                } else {
                    self.ink_alpha
                },
                LWA_COLORKEY | LWA_ALPHA,
            )?;
        }
        Ok(())
    }
    pub(super) unsafe fn backing(
        &self,
        class: PCWSTR,
        owner: HWND,
    ) -> Result<Option<desktop::Window>> {
        let Some((color, alpha)) = self.fill.filter(|_| self.keyed) else {
            return Ok(None);
        };
        let layer = desktop::Window(CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
            class,
            w!(""),
            WS_POPUP,
            0,
            0,
            1,
            1,
            Some(owner),
            None,
            None,
            None,
        )?);
        set_color(layer.0, BACKGROUND, color)?;
        SetPropW(layer.0, FILL_ONLY, Some(HANDLE(1_usize as *mut _)))?;
        SetLayeredWindowAttributes(layer.0, COLORREF(0), alpha, LWA_ALPHA)?;
        Ok(Some(layer))
    }
}

unsafe fn set_color(hwnd: HWND, key: PCWSTR, value: COLORREF) -> Result<()> {
    SetPropW(hwnd, key, Some(HANDLE((value.0 as usize + 1) as *mut _)))
}
unsafe fn color(hwnd: HWND, key: PCWSTR) -> COLORREF {
    COLORREF((GetPropW(hwnd, key).0 as u32).saturating_sub(1))
}
pub(super) unsafe fn edit(hwnd: HWND, dc: HDC) -> LRESULT {
    let background = color(hwnd, BACKGROUND);
    SetTextColor(dc, color(hwnd, INK));
    SetBkColor(dc, background);
    SetDCBrushColor(dc, background);
    LRESULT(GetStockObject(DC_BRUSH).0 as isize)
}
pub(super) unsafe fn popup(hwnd: HWND) -> LRESULT {
    let mut ps = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut ps);
    let mut rect = RECT::default();
    if GetClientRect(hwnd, &mut rect).is_ok() {
        let brush = HBRUSH(GetStockObject(DC_BRUSH).0);
        SetDCBrushColor(dc, color(hwnd, BACKGROUND));
        FillRect(dc, &rect, brush);
        if GetPropW(hwnd, FILL_ONLY).is_invalid() {
            let signal = neuro_mini_ui::Palette::default().signal;
            SetDCBrushColor(
                dc,
                rgb(Color(
                    (signal >> 16) as u8,
                    (signal >> 8) as u8,
                    signal as u8,
                )),
            );
            FrameRect(dc, &rect, brush);
        }
    }
    let _ = EndPaint(hwnd, &ps);
    LRESULT(0)
}
pub(super) unsafe fn retire(hwnd: HWND) {
    for key in [INK, BACKGROUND, FILL_ONLY] {
        let _ = RemovePropW(hwnd, key);
    }
}

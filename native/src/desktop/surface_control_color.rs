//! HWND 拥有已解析控件颜色；EDIT/Select 共用校验、释放和 GDI alpha 合成。
use crate::surface_color::Color;
use windows::{
    core::{w, Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
const KEY: windows::core::PCWSTR = w!("Otter.Surface.Control.Color");
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct Palette {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}
pub(super) unsafe fn current(hwnd: HWND) -> Palette {
    let pointer = GetPropW(hwnd, KEY).0.cast::<Palette>();
    if pointer.is_null() {
        Palette::default()
    } else {
        *pointer
    }
}
pub(super) unsafe fn clear(hwnd: HWND) {
    if let Ok(value) = RemovePropW(hwnd, KEY) {
        if !value.is_invalid() {
            drop(Box::from_raw(value.0.cast::<Palette>()));
        }
    }
}
pub(super) fn update(hwnd: HWND, value: Palette) -> Result<bool> {
    if [value.foreground, value.background]
        .into_iter()
        .flatten()
        .any(|c| {
            c.0.iter()
                .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
        })
    {
        return Err(Error::new(E_INVALIDARG, "Invalid control color"));
    }
    unsafe {
        if current(hwnd) == value {
            return Ok(false);
        }
        if value == Palette::default() {
            clear(hwnd);
        } else {
            let old = GetPropW(hwnd, KEY);
            let mut owned = Box::new(value);
            SetPropW(
                hwnd,
                KEY,
                Some(HANDLE((&mut *owned as *mut Palette).cast())),
            )?;
            let _ = Box::into_raw(owned);
            if !old.is_invalid() {
                drop(Box::from_raw(old.0.cast::<Palette>()));
            }
        }
    }
    Ok(true)
}
pub(super) fn blend(color: Option<Color>, background: COLORREF) -> COLORREF {
    let Some(Color(rgba)) = color else {
        return background;
    };
    let mut value = 0;
    for (i, channel) in rgba[..3].iter().enumerate() {
        let base = ((background.0 >> (i * 8)) & 255) as f32;
        value |= ((channel * 255. * rgba[3] + base * (1. - rgba[3])).round() as u32) << (i * 8);
    }
    COLORREF(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_control_color_alpha_preserves_background_and_channel_order() {
        let base = COLORREF(0x563412);
        assert_eq!(blend(None, base), base);
        assert_eq!(blend(Some(Color([1., 0., 0., 0.])), base), base);
        assert_eq!(blend(Some(Color([1., 0., 0., 1.])), base), COLORREF(255));
        assert_eq!(
            blend(Some(Color([1., 0., 0., 0.5])), base),
            COLORREF(0x2b1a89)
        );
    }
    #[test]
    fn surface_control_color_invalid_values_are_rejected_before_hwnd_access() {
        for value in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            let color = Some(Color([value, 0., 0., 1.]));
            assert!(update(
                HWND::default(),
                Palette {
                    foreground: color,
                    background: None
                }
            )
            .is_err());
            assert!(update(
                HWND::default(),
                Palette {
                    foreground: None,
                    background: color
                }
            )
            .is_err());
        }
    }
}

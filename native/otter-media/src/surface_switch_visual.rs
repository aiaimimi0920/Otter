//! Switch 的 Neuro 派生状态色；只决定画刷颜色，不修改草稿或授予交互权限。
use crate::surface_control_colors::{ACTIVE, HOVER, LINE, MUTED, ON_ACTIVE, TRACK};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
pub(super) struct Palette {
    pub track: D2D1_COLOR_F,
    pub border: D2D1_COLOR_F,
    pub thumb: D2D1_COLOR_F,
}
fn mix(a: D2D1_COLOR_F, b: D2D1_COLOR_F, amount: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: a.r * amount + b.r * (1. - amount),
        g: a.g * amount + b.g * (1. - amount),
        b: a.b * amount + b.b * (1. - amount),
        a: 1.,
    }
}
pub(super) fn palette(checked: bool, disabled: bool, hover: bool, pressed: bool) -> Palette {
    if disabled {
        return Palette {
            track: TRACK,
            border: MUTED,
            thumb: mix(MUTED, TRACK, 0.6),
        };
    }
    let mut track = if checked {
        ACTIVE
    } else if hover || pressed {
        HOVER
    } else {
        TRACK
    };
    if pressed {
        track = mix(track, D2D1_COLOR_F::default(), 0.82);
    } else if checked && hover {
        track = mix(
            ACTIVE,
            D2D1_COLOR_F {
                r: 1.,
                g: 1.,
                b: 1.,
                a: 1.,
            },
            0.86,
        );
    }
    Palette {
        track,
        border: if checked {
            track
        } else if hover || pressed {
            MUTED
        } else {
            LINE
        },
        thumb: if checked { ON_ACTIVE } else { MUTED },
    }
}

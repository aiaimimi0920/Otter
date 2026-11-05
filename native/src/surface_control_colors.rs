//! 原生 Surface 控件的 Neuro 语义颜色；动态主题 owner 接入前的默认深色基线。
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
pub(crate) const TRACK: D2D1_COLOR_F = rgb(17, 23, 32);
pub(crate) const HOVER: D2D1_COLOR_F = rgb(24, 34, 44);
pub(crate) const ACTIVE: D2D1_COLOR_F = rgb(217, 255, 56);
pub(crate) const MUTED: D2D1_COLOR_F = rgb(146, 154, 159);
pub(crate) const LINE: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 1.,
    g: 1.,
    b: 1.,
    a: 0.12,
};
// Neuro on-yellow = signalYellow 14% + black，不引入第二种强调色。
pub(crate) const ON_ACTIVE: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 217. / 255. * 0.14,
    g: 0.14,
    b: 56. / 255. * 0.14,
    a: 1.,
};
const fn rgb(r: u8, g: u8, b: u8) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: r as f32 / 255.,
        g: g as f32 / 255.,
        b: b as f32 / 255.,
        a: 1.,
    }
}

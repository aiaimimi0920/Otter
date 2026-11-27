//! All picker formats describe the same sampled 8-bit sRGB value.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Format {
    #[default]
    Hex,
    Rgb,
    Hsl,
}
impl Format {
    pub fn next(self) -> Self {
        match self {
            Self::Hex => Self::Rgb,
            Self::Rgb => Self::Hsl,
            Self::Hsl => Self::Hex,
        }
    }
    pub fn describe(self, [r, g, b]: [u8; 3]) -> String {
        match self {
            Self::Hex => format!("#{r:02X}{g:02X}{b:02X}"),
            Self::Rgb => format!("rgb({r}, {g}, {b})"),
            Self::Hsl => {
                let [r, g, b] = [r, g, b].map(|c| f64::from(c) / 255.0);
                let hi = r.max(g).max(b);
                let lo = r.min(g).min(b);
                let d = hi - lo;
                let l = (hi + lo) * 0.5;
                let (h, s) = if d == 0.0 {
                    (0.0, 0.0)
                } else {
                    let h = if hi == r {
                        ((g - b) / d).rem_euclid(6.0)
                    } else if hi == g {
                        (b - r) / d + 2.0
                    } else {
                        (r - g) / d + 4.0
                    };
                    (60.0 * h, d / (1.0 - (2.0 * l - 1.0).abs()))
                };
                format!(
                    "hsl({}, {}%, {}%)",
                    h.round() as u16 % 360,
                    (100.0 * s).round(),
                    (100.0 * l).round()
                )
            }
        }
    }
}
pub fn use_black_cross(rgb: [u8; 3]) -> bool {
    let [r, g, b] = rgb.map(|c| {
        let v = f64::from(c) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    });
    let luminance = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (luminance + 0.05) / 0.05 >= 1.05 / (luminance + 0.05)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats_and_perceptual_contrast() {
        for (rgb, hsl) in [
            ([0, 0, 0], "hsl(0, 0%, 0%)"),
            ([255; 3], "hsl(0, 0%, 100%)"),
            ([255, 0, 128], "hsl(330, 100%, 50%)"),
            ([0, 255, 0], "hsl(120, 100%, 50%)"),
        ] {
            assert_eq!(Format::Hsl.describe(rgb), hsl);
        }
        assert_eq!(Format::Hex.describe([1, 10, 255]), "#010AFF");
        assert_eq!(Format::Rgb.describe([1, 10, 255]), "rgb(1, 10, 255)");
        assert!(use_black_cross([0, 255, 0]));
        assert!(!use_black_cross([0, 0, 255]));
    }
}

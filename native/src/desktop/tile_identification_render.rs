//! Draw temporary identification within existing Tile output; one DirectWrite format, no child windows/timers/extra pixel surfaces.
use crate::tile_identification::Marker;
use std::time::Instant;
use windows::{
    core::{w, Result},
    Win32::Graphics::{
        Direct2D::{Common::*, *},
        DirectWrite::*,
    },
};

pub(super) struct Painter {
    factory: IDWriteFactory,
    format: IDWriteTextFormat,
}
pub(in crate::desktop::tile_window) fn close_rect(size: [f32; 2]) -> D2D_RECT_F {
    let [w, h] = size;
    D2D_RECT_F {
        left: w * 0.1,
        top: h * 0.8,
        right: w * 0.9,
        bottom: h * 0.95,
    }
}
fn escaped(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_control()
            || matches!(ch, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            output.extend(ch.escape_unicode());
        } else {
            output.push(ch);
        }
    }
    output
}
pub(super) fn text(marker: &Marker, now: Instant) -> String {
    let endpoint = &marker.endpoint;
    let seconds = marker
        .until
        .saturating_duration_since(now)
        .as_millis()
        .div_ceil(1000);
    crate::ui_text::format(
        "integration-tile-identification",
        &[
            (
                "name",
                escaped(endpoint.display.as_ref().map_or("", |d| &d.name)).into(),
            ),
            ("width", endpoint.pixel_size.width.into()),
            ("height", endpoint.pixel_size.height.into()),
            ("device", escaped(&endpoint.device_id).into()),
            ("output", escaped(&endpoint.output_id).into()),
            ("seconds", seconds.to_string().into()),
        ],
    )
}
impl Painter {
    pub fn new() -> Result<Self> {
        unsafe {
            let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let format = factory.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                18.,
                w!("zh-cn"),
            )?;
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
            Ok(Self { factory, format })
        }
    }
    pub unsafe fn draw(
        &self,
        target: &ID2D1RenderTarget,
        marker: &Marker,
        now: Instant,
    ) -> Result<()> {
        let size = target.GetSize();
        target.Clear(Some(&D2D1_COLOR_F {
            r: 0.04,
            g: 0.05,
            b: 0.06,
            a: 1.,
        }));
        let brush = target.CreateSolidColorBrush(
            &D2D1_COLOR_F {
                r: 0.85,
                g: 0.9,
                b: 0.3,
                a: 1.,
            },
            None,
        )?;
        let rect = close_rect([size.width, size.height]);
        target.FillRectangle(&rect, &brush);
        brush.SetColor(&D2D1_COLOR_F {
            r: 0.9,
            g: 0.92,
            b: 0.94,
            a: 1.,
        });
        let content = text(marker, now).encode_utf16().collect::<Vec<_>>();
        let height = size.height * 0.72;
        let layout =
            self.factory
                .CreateTextLayout(&content, &self.format, size.width * 0.8, height)?;
        let mut metrics = DWRITE_TEXT_METRICS::default();
        layout.GetMetrics(&mut metrics)?;
        if metrics.height > height {
            // Retain long physical names/stable IDs in full; reduce font size instead of truncating crucial identity.
            layout.SetFontSize(
                (18. * height / metrics.height).max(0.1),
                DWRITE_TEXT_RANGE {
                    startPosition: 0,
                    length: content.len() as u32,
                },
            )?;
        }
        target.DrawTextLayout(
            D2D_POINT_2F {
                x: size.width * 0.1,
                y: size.height * 0.06,
            },
            &layout,
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
        );
        brush.SetColor(&D2D1_COLOR_F {
            r: 0.04,
            g: 0.05,
            b: 0.06,
            a: 1.,
        });
        target.DrawText(
            &crate::ui_text::text("integration-close-identification")
                .encode_utf16()
                .collect::<Vec<_>>(),
            &self.format,
            &rect,
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
        Ok(())
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "tile_identification_pixel_tests.rs"]
mod tests;

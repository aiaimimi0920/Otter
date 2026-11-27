//! Native Surface text measurement and painting share DirectWrite layouts held by render owners, with no global cache.
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::E_INVALIDARG,
        Graphics::{
            Direct2D::{
                Common::D2D_POINT_2F, ID2D1RenderTarget, ID2D1SolidColorBrush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            },
            DirectWrite::*,
        },
    },
};
#[path = "surface_text_selection.rs"]
mod selection;
pub use selection::TextHit;
#[path = "surface_text_families.rs"]
mod families;
#[path = "surface_text_overflow.rs"]
mod overflow;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whitespace {
    Normal,
    Nowrap,
    Pre,
    PreWrap,
}
#[derive(Clone, Copy, Debug)]
pub enum Align {
    Start,
    Center,
    End,
}
#[derive(Clone, Copy)]
pub struct Style<'a> {
    pub family: &'a str,
    pub size: f32,
    pub weight: u16,
    pub whitespace: Whitespace,
    pub align: Align,
    pub line_height: Option<f32>,
}
const MAX_CACHED_FORMATS: usize = 64;

#[derive(Clone, PartialEq, Eq)]
struct FormatKey {
    family: String,
    size_bits: u32,
    weight: u16,
    no_wrap: bool,
    align: u8,
}

pub struct Engine {
    factory: IDWriteFactory,
    families: std::cell::RefCell<families::Families>,
    formats: std::cell::RefCell<std::collections::VecDeque<(FormatKey, IDWriteTextFormat)>>,
}
#[derive(Clone)]
pub struct Layout {
    native: IDWriteTextLayout,
    metrics: DWRITE_TEXT_METRICS,
    baseline: f32,
    // DirectWrite indexes refer to normalized UTF-16, never original props.text; cloned layouts share content.
    text: std::rc::Rc<[u16]>,
}
#[cfg(test)]
thread_local! {
    static MEASURE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(crate) fn measure_calls() -> usize {
    MEASURE_CALLS.get()
}
fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Invalid Surface text layout")
}
fn content(text: &str, whitespace: Whitespace) -> Result<Vec<u16>> {
    if text.len() > 256 * 1024 || text.encode_utf16().count() > 65536 || text.contains('\0') {
        return Err(invalid());
    }
    let preserve = matches!(whitespace, Whitespace::Pre | Whitespace::PreWrap);
    let mut output = Vec::with_capacity(text.len().min(65536));
    let mut space = false;
    let mut chars = text.chars().peekable();
    while let Some(mut ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            ch = '\n';
        }
        if !preserve && matches!(ch, ' ' | '\t' | '\n' | '\u{000c}') {
            if !space {
                output.push(u16::from(b' '));
            }
            space = true;
        } else {
            space = false;
            let mut buffer = [0; 2];
            output.extend_from_slice(ch.encode_utf16(&mut buffer));
        }
    }
    Ok(output)
}
impl Engine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            factory: unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? },
            families: Default::default(),
            formats: Default::default(),
        })
    }
    /// Return one installed native family shared by text and native fields; release the cache with Engine.
    pub fn resolve_family(&self, source: &str) -> Result<std::rc::Rc<str>> {
        self.families.borrow_mut().resolve(&self.factory, source)
    }
    pub fn layout(&self, text: &str, style: Style<'_>, width: f32, height: f32) -> Result<Layout> {
        #[cfg(test)]
        MEASURE_CALLS.set(MEASURE_CALLS.get() + 1);
        if style.family.is_empty()
            || style.family.len() > 256
            || style.family.chars().any(char::is_control)
            || !style.size.is_finite()
            || style.size <= 0.
            || style.size > 8192.
            || !(100..=900).contains(&style.weight)
            || style
                .line_height
                .is_some_and(|v| !v.is_finite() || !(0. ..=32768.).contains(&v))
            || [width, height]
                .iter()
                .any(|v| !v.is_finite() || *v < 0. || *v > 32768.)
        {
            return Err(invalid());
        }
        let text = content(text, style.whitespace)?;
        unsafe {
            let no_wrap = matches!(style.whitespace, Whitespace::Nowrap | Whitespace::Pre);
            let align_code = match style.align {
                Align::Start => 0,
                Align::Center => 1,
                Align::End => 2,
            };
            let key = FormatKey {
                family: style.family.to_owned(),
                size_bits: style.size.to_bits(),
                weight: style.weight,
                no_wrap,
                align: align_code,
            };
            let format = if let Some(hit) = {
                let mut cache = self.formats.borrow_mut();
                cache.iter().position(|(k, _)| k == &key).map(|idx| {
                    let entry = cache.remove(idx).unwrap();
                    let fmt = entry.1.clone();
                    cache.push_back(entry);
                    fmt
                })
            } {
                hit
            } else {
                let created = self.factory.CreateTextFormat(
                    &HSTRING::from(style.family),
                    None,
                    DWRITE_FONT_WEIGHT(i32::from(style.weight)),
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    style.size,
                    w!("zh-CN"),
                )?;
                created.SetWordWrapping(if no_wrap {
                    DWRITE_WORD_WRAPPING_NO_WRAP
                } else {
                    DWRITE_WORD_WRAPPING_WRAP
                })?;
                created.SetTextAlignment(match style.align {
                    Align::Start => DWRITE_TEXT_ALIGNMENT_LEADING,
                    Align::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
                    Align::End => DWRITE_TEXT_ALIGNMENT_TRAILING,
                })?;
                let mut cache = self.formats.borrow_mut();
                while cache.len() >= MAX_CACHED_FORMATS {
                    cache.pop_front();
                }
                cache.push_back((key, created.clone()));
                created
            };
            let native = self
                .factory
                .CreateTextLayout(&text, &format, width, height)?;
            let mut metrics = DWRITE_TEXT_METRICS::default();
            native.GetMetrics(&mut metrics)?;
            if metrics.lineCount == 0 || metrics.lineCount > 65537 {
                return Err(invalid());
            }
            let mut lines = vec![DWRITE_LINE_METRICS::default(); metrics.lineCount as usize];
            let mut count = 0;
            native.GetLineMetrics(Some(&mut lines), &mut count)?;
            let mut baseline = lines[0].baseline;
            if let Some(height) = style.line_height {
                // Split extra line spacing evenly while preserving actual font ascent/descent rather than guessing baselines with fixed percentages.
                baseline += (height - lines[0].height) / 2.;
                if !baseline.is_finite() || baseline < 0. {
                    return Err(invalid());
                }
                native.SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, height, baseline)?;
                native.GetMetrics(&mut metrics)?;
            }
            if [
                metrics.width,
                metrics.widthIncludingTrailingWhitespace,
                metrics.height,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
            {
                return Err(invalid());
            }
            Ok(Layout {
                native,
                metrics,
                baseline,
                text: text.into(),
            })
        }
    }
}
impl Layout {
    /// Reuse cluster hit testing on an annotation's actual layout. Do not
    /// apply Surface whitespace, CSS line height or font-family resolution.
    pub(crate) fn from_annotation(native: IDWriteTextLayout, text: Vec<u16>) -> Result<Self> {
        let mut metrics = DWRITE_TEXT_METRICS::default();
        unsafe { native.GetMetrics(&mut metrics)? };
        if text.len() > 65536 || metrics.lineCount == 0 || metrics.lineCount > 65537 {
            return Err(invalid());
        }
        let mut lines = vec![DWRITE_LINE_METRICS::default(); metrics.lineCount as usize];
        let mut count = 0;
        unsafe { native.GetLineMetrics(Some(&mut lines), &mut count)? };
        if [
            metrics.width,
            metrics.widthIncludingTrailingWhitespace,
            metrics.height,
            lines[0].baseline,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
        {
            return Err(invalid());
        }
        Ok(Self {
            native,
            metrics,
            baseline: lines[0].baseline,
            text: text.into(),
        })
    }
    pub(crate) fn insertion_geometry(&self, pos: u32) -> Result<[f32; 3]> {
        self.validate_range(pos..pos)?;
        let (mut x, mut y, mut h) = (0., 0., DWRITE_HIT_TEST_METRICS::default());
        unsafe {
            self.native
                .HitTestTextPosition(pos, false, &mut x, &mut y, &mut h)?;
        }
        if [x, y, h.height].iter().any(|v| !v.is_finite()) || h.height < 0. {
            return Err(invalid());
        }
        Ok([x, y, h.height])
    }
    #[cfg(test)]
    pub(crate) fn text_for_test(&self) -> String {
        String::from_utf16(&self.text).unwrap()
    }
    pub(crate) fn same_text(&self, other: &Self) -> bool {
        self.text == other.text
    }
    pub(crate) fn allocated_width(&self) -> f32 {
        self.metrics.layoutWidth
    }
    pub fn baseline(&self) -> f32 {
        self.baseline
    }
    pub fn size(&self) -> [f32; 2] {
        [
            self.metrics.widthIncludingTrailingWhitespace,
            self.metrics.height,
        ]
    }
    pub fn line_count(&self) -> u32 {
        self.metrics.lineCount
    }
    /// Callers own BeginDraw/EndDraw and target-thread handling; shared layouts keep measurement and painting consistent.
    pub fn draw(
        &self,
        target: &ID2D1RenderTarget,
        brush: &ID2D1SolidColorBrush,
        origin: [f32; 2],
    ) -> Result<()> {
        if origin
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 16_777_216.)
        {
            return Err(invalid());
        }
        unsafe {
            target.DrawTextLayout(
                D2D_POINT_2F {
                    x: origin[0],
                    y: origin[1],
                },
                &self.native,
                brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "surface_text_overflow_tests.rs"]
mod overflow_tests;
#[cfg(test)]
#[path = "surface_text_tests.rs"]
mod tests;

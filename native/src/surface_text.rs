//! Surface 原生文字测量和绘制共用 DirectWrite 布局；由渲染 owner 持有，无全局缓存。
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
pub struct Engine {
    factory: IDWriteFactory,
    families: std::cell::RefCell<families::Families>,
}
#[derive(Clone)]
pub struct Layout {
    native: IDWriteTextLayout,
    metrics: DWRITE_TEXT_METRICS,
    baseline: f32,
    // DirectWrite 索引对应规范化 UTF-16，不得索引原始 props.text；克隆布局共享内容。
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
        })
    }
    /// 返回已安装的单一 native family，供文字和原生字段共用；缓存随 Engine 释放。
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
            let format = self.factory.CreateTextFormat(
                &HSTRING::from(style.family),
                None,
                DWRITE_FONT_WEIGHT(i32::from(style.weight)),
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                style.size,
                w!("zh-CN"),
            )?;
            format.SetWordWrapping(
                if matches!(style.whitespace, Whitespace::Nowrap | Whitespace::Pre) {
                    DWRITE_WORD_WRAPPING_NO_WRAP
                } else {
                    DWRITE_WORD_WRAPPING_WRAP
                },
            )?;
            format.SetTextAlignment(match style.align {
                Align::Start => DWRITE_TEXT_ALIGNMENT_LEADING,
                Align::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
                Align::End => DWRITE_TEXT_ALIGNMENT_TRAILING,
            })?;
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
                // 平分额外行距，保留字体实际 ascent/descent，而不是固定百分比猜测基线。
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
    /// 调用方负责 BeginDraw/EndDraw 和目标线程；共享布局确保测量与绘制一致。
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

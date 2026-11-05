//! 原生文字布局与绘制共用 DirectWrite；布局局部持有，不引入计时器或全局字体缓存。
use crate::{
    editor::Stroke,
    text_annotation::TextAnnotation,
    viewport::{Point, Viewport},
};
use windows::{
    core::{w, Result, HSTRING},
    Win32::Graphics::{
        Direct2D::{Common::*, *},
        DirectWrite::*,
    },
};

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
) -> Result<()> {
    let Some(text) = &stroke.text else {
        return Ok(());
    };
    let Some(start) = stroke.points.first() else {
        return Ok(());
    };
    let Some(end) = stroke.points.get(1) else {
        return Ok(());
    };
    let layout = layout(
        text,
        viewport.scale,
        (end.x - start.x).abs() * viewport.scale,
        (end.y - start.y).abs() * viewport.scale,
    )?;
    target.DrawTextLayout(
        D2D_POINT_2F {
            x: viewport.origin.x + (start.x + 4.0) * viewport.scale,
            y: viewport.origin.y + (start.y + 4.0) * viewport.scale,
        },
        &layout,
        brush,
        D2D1_DRAW_TEXT_OPTIONS_CLIP,
    );
    Ok(())
}

pub(super) unsafe fn measure(text: &TextAnnotation) -> Result<Point> {
    let layout = layout(text, 1.0, 1_000_000.0, 1_000_000.0)?;
    let mut metrics = DWRITE_TEXT_METRICS::default();
    layout.GetMetrics(&mut metrics)?;
    Ok(Point {
        x: metrics.widthIncludingTrailingWhitespace.ceil() + 8.0,
        y: metrics.height.ceil() + 8.0,
    })
}

/// 与绘制相同的字体和布局设置；只返回数值，COM 对象不跨线程。
pub fn measure_flip_metrics(text: &TextAnnotation) -> Result<crate::editor::TextFlipMetrics> {
    // 所有 COM 接口、UTF-16 缓冲和输出数组均局部拥有，原生调用不保留借用。
    unsafe {
        let layout = layout(text, 1.0, 1_000_000.0, 1_000_000.0)?;
        let mut metrics = DWRITE_TEXT_METRICS::default();
        layout.GetMetrics(&mut metrics)?;
        let mut count = 0;
        // 空缓冲探测通常返回 E_NOT_SUFFICIENT_BUFFER，但仍写入所需行数。
        let probe = layout.GetLineMetrics(None, &mut count);
        if count == 0 {
            probe?;
        }
        if count == 0 || count as usize > crate::text_annotation::MAX_TEXT_UNITS + 1 {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Invalid text line count",
            ));
        }
        let mut lines = vec![DWRITE_LINE_METRICS::default(); count as usize];
        layout.GetLineMetrics(Some(&mut lines), &mut count)?;
        let baseline = lines[0].baseline + 4.0;
        if !metrics.widthIncludingTrailingWhitespace.is_finite()
            || metrics.widthIncludingTrailingWhitespace < 0.0
            || !baseline.is_finite()
            || baseline < 0.0
        {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Invalid text flip metrics",
            ));
        }
        Ok(crate::editor::TextFlipMetrics {
            advance_width: metrics.widthIncludingTrailingWhitespace,
            left_inset: 4.0,
            baseline_from_top: baseline,
        })
    }
}
pub(super) unsafe fn layout(
    text: &TextAnnotation,
    scale: f32,
    width: f32,
    height: f32,
) -> Result<IDWriteTextLayout> {
    let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
    let format = factory.CreateTextFormat(
        // PCWSTR字体名不能表达NUL；显示使用有效前缀/系统fallback，模型仍保留原值。
        &HSTRING::from(
            text.family()
                .split('\0')
                .next()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or("Segoe UI"),
        ),
        None,
        if text.bold() {
            DWRITE_FONT_WEIGHT_BOLD
        } else {
            DWRITE_FONT_WEIGHT_MEDIUM
        },
        DWRITE_FONT_STYLE_NORMAL,
        DWRITE_FONT_STRETCH_NORMAL,
        text.font_size() * scale,
        w!("zh-CN"),
    )?;
    format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
    let content: Vec<u16> = text.content().encode_utf16().collect();
    factory.CreateTextLayout(
        &content,
        &format,
        (width - 8.0 * scale).max(1.0),
        (height - 8.0 * scale).max(1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_directwrite_metrics_feed_baseline_flip_without_rounding_layout_size() {
        use crate::{crop_geometry::Rect, editor::Editor, raster_flip::Axis};
        for content in ["Otter", "中文标注", "Otter   ", "第一行\r\n第二行"] {
            let text = TextAnnotation::new(content, "Segoe UI", 18.0, false).unwrap();
            let (size, metrics) = unsafe {
                (
                    measure(&text).unwrap(),
                    measure_flip_metrics(&text).unwrap(),
                )
            };
            assert_eq!(size.x, metrics.advance_width.ceil() + 8.0);
            assert!(metrics.baseline_from_top > 4.0 && metrics.baseline_from_top < size.y);
            let mut editor = Editor::default();
            editor
                .insert_text(Point { x: 20.0, y: 30.0 }, size, text)
                .unwrap();
            let source = &editor.elements()[0];
            let frame = Rect {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 200.0,
            };
            let vertical = source
                .flipped_with_text_metrics(frame, Axis::Vertical, Some(metrics))
                .unwrap();
            let expected = 200.0 - (30.0 + metrics.baseline_from_top) + 18.0;
            assert!((vertical.points[0].y + metrics.baseline_from_top - expected).abs() < 0.0001);
            let horizontal = source
                .flipped_with_text_metrics(frame, Axis::Horizontal, Some(metrics))
                .unwrap();
            assert!(
                (horizontal.points[0].x + 4.0 - (400.0 - 24.0 - metrics.advance_width)).abs()
                    < 0.0001
            );
            assert_eq!(horizontal.text, source.text);
            assert_eq!(vertical.text, source.text);
        }
        {
            let plain = TextAnnotation::new("Otter", "Segoe UI", 18.0, false).unwrap();
            let spaces = TextAnnotation::new("Otter   ", "Segoe UI", 18.0, false).unwrap();
            let multi = TextAnnotation::new("Otter\r\nSecond", "Segoe UI", 18.0, false).unwrap();
            let a = measure_flip_metrics(&plain).unwrap();
            let b = measure_flip_metrics(&spaces).unwrap();
            let c = measure_flip_metrics(&multi).unwrap();
            assert!(b.advance_width > a.advance_width);
            assert_eq!(a.baseline_from_top, b.baseline_from_top);
            assert_eq!(a.baseline_from_top, c.baseline_from_top);
        }
    }
    #[test]
    fn automatic_measurement_includes_multiline_height_and_trailing_spaces() {
        let single = TextAnnotation::new("你好 Otter", "微软雅黑", 16.0, false).unwrap();
        let multi = TextAnnotation::new("你好 Otter\r\n第二行", "微软雅黑", 16.0, false).unwrap();
        let spaces = TextAnnotation::new("你好 Otter   ", "微软雅黑", 16.0, false).unwrap();
        unsafe {
            let a = measure(&single).unwrap();
            let b = measure(&multi).unwrap();
            let c = measure(&spaces).unwrap();
            assert!(a.x > 8.0 && a.y > 8.0);
            assert!(b.y > a.y);
            assert!(c.x > a.x);
            assert_eq!(c.y, a.y);
        }
    }
}

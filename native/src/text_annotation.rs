//! 文字标注的有界不可变内容；快照和复制共享字符串，编辑替换内容而非原地改写。
use std::sync::Arc;
#[path = "text_content.rs"]
mod content;

pub const MAX_TEXT_UNITS: usize = 65_536;
// 序号半径 999 的基线字号为 round(999 * 1.15) = 1149。
const MAX_FONT_SIZE: f32 = 1149.0;
#[derive(Clone, Debug, PartialEq)]
pub struct TextAnnotation {
    content: Arc<content::Content>,
    font: TextFont,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TextFont {
    family: Arc<str>,
    font_size: f32,
    bold: bool,
}
impl Default for TextFont {
    fn default() -> Self {
        Self::new("微软雅黑", 16.0, false).unwrap()
    }
}
impl TextFont {
    pub(crate) fn restored(family: &str, font_size: f32, bold: bool) -> Option<Self> {
        if family.encode_utf16().count() > 128
            || family.contains(['\r', '\n'])
            || !font_size.is_finite()
            || !(1.0..=1_000_000.0).contains(&font_size)
        {
            return None;
        }
        Some(Self {
            family: Arc::from(family),
            font_size,
            bold,
        })
    }
    pub fn new(family: &str, font_size: f32, bold: bool) -> Option<Self> {
        if family.trim().is_empty()
            || family.len() > 192
            || family.encode_utf16().count() > 64
            || family.contains(['\0', '\r', '\n'])
            || !font_size.is_finite()
            || !(1.0..=MAX_FONT_SIZE).contains(&font_size)
        {
            return None;
        }
        Some(Self {
            family: Arc::from(family),
            font_size,
            bold,
        })
    }
    pub fn family(&self) -> &str {
        &self.family
    }
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
    pub fn bold(&self) -> bool {
        self.bold
    }
}
impl TextAnnotation {
    /// 恢复保留原文，包含空白/NUL/CRLF；原生EDIT不可表达的值不得经控件截断提交。
    pub(crate) fn restored(content: &str, family: &str, size: f32, bold: bool) -> Option<Self> {
        if content.len() > MAX_TEXT_UNITS * 3 || content.encode_utf16().count() > MAX_TEXT_UNITS {
            return None;
        }
        Some(Self {
            content: Arc::new(content::Content::restored(content)?),
            font: TextFont::restored(family, size, bold)?,
        })
    }
    pub fn new(content: &str, family: &str, font_size: f32, bold: bool) -> Option<Self> {
        // 先检查 UTF-8 字节上界，再计 UTF-16 单元；避免对任意大输入先分配副本。
        if content.len() > MAX_TEXT_UNITS * 3
            || content.encode_utf16().count() > MAX_TEXT_UNITS
            || content.trim().is_empty()
            || content.contains('\0')
        {
            return None;
        }
        Some(Self {
            // 原生 EDIT 使用 CRLF，模型使用 LF，避免打开后原样确认生成伪修改。
            content: Arc::new(content::Content::new(content)?),
            font: TextFont::new(family, font_size, bold)?,
        })
    }
    pub fn content(&self) -> &str {
        &self.content
    }
    pub fn family(&self) -> &str {
        self.font.family()
    }
    pub fn font_size(&self) -> f32 {
        self.font.font_size()
    }
    pub fn bold(&self) -> bool {
        self.font.bold()
    }
    pub fn with_font_size(&self, size: f32) -> Option<Self> {
        if !size.is_finite() || !(1.0..=1_000_000.0).contains(&size) {
            return None;
        }
        Some(Self {
            font: TextFont {
                font_size: size,
                ..self.font.clone()
            },
            ..self.clone()
        })
    }
    pub fn font(&self) -> &TextFont {
        &self.font
    }
    pub fn with_font(&self, font: TextFont) -> Self {
        Self {
            font,
            ..self.clone()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_history_and_snapshot_retain_the_same_budget_owner() {
        use crate::{editor::Editor, viewport::Point};
        let text = TextAnnotation::new(&"x".repeat(5000), "Arial", 16.0, false).unwrap();
        let weak = Arc::downgrade(&text.content);
        let mut editor = Editor::default();
        editor
            .insert_text(
                Point::default(),
                Point {
                    x: 50000.0,
                    y: 24.0,
                },
                text,
            )
            .unwrap();
        let snapshot = editor.export_snapshot();
        assert!(editor.undo());
        assert!(weak.upgrade().is_some());
        drop(editor);
        assert!(weak.upgrade().is_some());
        drop(snapshot);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn unicode_limits_are_in_utf16_units_and_clones_share_content() {
        let text = TextAnnotation::new("你好\nOtter 🦦", "微软雅黑", 16.0, false).unwrap();
        let cloned = text.clone();
        assert!(Arc::ptr_eq(&text.content, &cloned.content));
        assert!(
            TextAnnotation::new(&"🦦".repeat(MAX_TEXT_UNITS / 2), "Segoe UI", 16.0, false)
                .is_some()
        );
        assert!(TextAnnotation::new(
            &"🦦".repeat(MAX_TEXT_UNITS / 2 + 1),
            "Segoe UI",
            16.0,
            false
        )
        .is_none());
        for input in ["", " \r\n", "a\0b"] {
            assert!(TextAnnotation::new(input, "Segoe UI", 16.0, false).is_none());
        }
        assert!(text.with_font_size(f32::NAN).is_none());
        assert!(text.with_font_size(1_000_001.0).is_none());
    }
}

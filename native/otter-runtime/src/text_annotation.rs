//! Bounded immutable text-annotation content; snapshots/copies share strings, and edits replace content rather than mutate it in place.
use std::sync::Arc;
#[path = "text_content.rs"]
pub mod content;

pub const MAX_TEXT_UNITS: usize = 65_536;
// The baseline font size for sequence radius 999 is round(999 * 1.15) = 1149.
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
    pub fn restored(family: &str, font_size: f32, bold: bool) -> Option<Self> {
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
    /// Restoration preserves original text, including whitespace/NUL/CRLF; values native EDIT cannot represent must not be truncated by the control and committed.
    pub fn restored(content: &str, family: &str, size: f32, bold: bool) -> Option<Self> {
        if content.len() > MAX_TEXT_UNITS * 3 || content.encode_utf16().count() > MAX_TEXT_UNITS {
            return None;
        }
        Some(Self {
            content: Arc::new(content::Content::restored(content)?),
            font: TextFont::restored(family, size, bold)?,
        })
    }
    pub fn new(content: &str, family: &str, font_size: f32, bold: bool) -> Option<Self> {
        // Check the UTF-8 byte limit before counting UTF-16 units, avoiding an initial copy of arbitrarily large input.
        if content.len() > MAX_TEXT_UNITS * 3
            || content.encode_utf16().count() > MAX_TEXT_UNITS
            || content.trim().is_empty()
            || content.contains('\0')
        {
            return None;
        }
        Some(Self {
            // Native EDIT uses CRLF while the model uses LF, preventing false edits when reopening and confirming unchanged content.
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
        Some(self.with_font(TextFont {
            font_size: size,
            ..self.font.clone()
        }))
    }
    pub fn font(&self) -> &TextFont {
        &self.font
    }
    pub fn with_font(&self, font: TextFont) -> Self {
        Self {
            content: Arc::clone(&self.content),
            font,
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
    fn changing_font_reuses_only_the_unchanged_owners() {
        let original = TextAnnotation::new("Otter", "Arial", 16.0, true).unwrap();
        let resized = original.with_font_size(30.0).unwrap();
        assert!(Arc::ptr_eq(&original.content, &resized.content));
        assert!(Arc::ptr_eq(&original.font.family, &resized.font.family));
        assert_eq!(resized.font_size(), 30.0);
        assert!(resized.bold());
        let replacement = TextFont::new("Segoe UI", 22.0, false).unwrap();
        let family = Arc::downgrade(&replacement.family);
        let changed = original.with_font(replacement);
        assert!(Arc::ptr_eq(&original.content, &changed.content));
        assert!(Arc::ptr_eq(
            &changed.font.family,
            &family.upgrade().unwrap()
        ));
        assert_eq!(changed.font_size(), 22.0);
        assert!(!changed.bold());
        assert_eq!(original.font_size(), 16.0);
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

//! 菜单关闭且 MenuTarget 通过后交付历史内容；不改写系统剪贴板或替换当前图片。
use super::super::State;
use crate::{
    clipboard_history::{Content, Entry},
    text_annotation::{TextAnnotation, TextFont},
    viewport::Point,
};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect},
};

pub(in crate::desktop) unsafe fn deliver(hwnd: HWND, state: &State, entry: Entry) -> Result<()> {
    match entry.content {
        Content::Bitmap { encoded, .. } => super::super::open_job::start_destination(
            hwnd,
            state,
            super::super::open_job::Input::Encoded(encoded),
            true,
        ),
        Content::Text(content) => {
            if !import_text(state, &content)? {
                return Ok(());
            }
            super::super::hit_region::update(hwnd, state)?;
            let _ = InvalidateRect(Some(hwnd), None, false);
            Ok(())
        }
    }
}

fn import_text(state: &State, content: &str) -> Result<bool> {
    let (font, screen) = {
        let canvas = state.canvas.borrow();
        let display = state.resolved_display.borrow();
        // 准入必须基于正在显示的快照；底层文档可能为空或尺寸不同。
        let point = match display.as_ref() {
            Some(frame) => frame
                .editor
                .as_deref()
                .and_then(|editor| editor.annotation_frame())
                .map_or(
                    Point {
                        x: frame.image.width as f32 / 2.0,
                        y: frame.image.height as f32 / 2.0,
                    },
                    |rect| Point {
                        x: rect.x as f32 + rect.width as f32 / 2.0,
                        y: rect.y as f32 + rect.height as f32 / 2.0,
                    },
                ),
            None => center(&canvas)?,
        };
        (
            canvas.tool_profiles.text_font.clone(),
            Point {
                x: canvas.viewport.origin.x + point.x * canvas.viewport.scale,
                y: canvas.viewport.origin.y + point.y * canvas.viewport.scale,
            },
        )
    };
    let lines = prepare(content, &font)?;
    // 空内容不触发显示接管，避免产生没有文字的编辑事务。
    if lines.is_empty() || !super::super::display_edit::prepare_text_at(state, screen) {
        return Ok(false);
    }
    let mut canvas = state.canvas.borrow_mut();
    let entries = centered(lines, center(&canvas)?);
    canvas.cancel_gesture();
    canvas
        .editor
        .insert_text_batch(&entries)
        .map_err(|error| Error::new(E_FAIL, format!("History text import rejected: {error:?}")))?;
    Ok(true)
}

fn center(canvas: &super::super::canvas::Canvas) -> Result<Point> {
    let image = canvas
        .image
        .as_ref()
        .ok_or_else(|| Error::new(E_FAIL, "请先打开图片，再导入历史文字。"))?;
    Ok(canvas.editor.annotation_frame().map_or(
        Point {
            x: image.width as f32 / 2.0,
            y: image.height as f32 / 2.0,
        },
        |rect| Point {
            x: rect.x as f32 + rect.width as f32 / 2.0,
            y: rect.y as f32 + rect.height as f32 / 2.0,
        },
    ))
}

fn centered(
    lines: Vec<(f32, Point, TextAnnotation)>,
    center: Point,
) -> Vec<(Point, Point, TextAnnotation)> {
    let height = lines
        .last()
        .map_or(0.0, |(offset, size, _)| offset + size.y);
    lines
        .into_iter()
        .map(|(offset, size, text)| {
            (
                Point {
                    x: center.x - size.x / 2.0,
                    y: center.y - height / 2.0 + offset,
                },
                size,
                text,
            )
        })
        .collect()
}

fn prepare(content: &str, font: &TextFont) -> Result<Vec<(f32, Point, TextAnnotation)>> {
    if content.len() > crate::clipboard_history::MAX_TEXT_BYTES || content.contains('\0') {
        return Err(Error::new(
            E_INVALIDARG,
            "History text exceeds import budget",
        ));
    }
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.split('\n').count() > 256 {
        return Err(Error::new(E_INVALIDARG, "History text exceeds line budget"));
    }
    let mut output = Vec::new();
    let mut offset = 0.0;
    for line in normalized.split('\n') {
        if line.trim().is_empty() {
            offset += font.font_size() * 1.25;
            continue;
        }
        // 一行保持一个对象；正文、历史及快照受模型共享预算约束，不截断或拆段。
        let text = TextAnnotation::new(line, font.family(), font.font_size(), font.bold())
            .ok_or_else(|| Error::new(E_INVALIDARG, "历史文字超出内容预算或格式无效。"))?;
        let size = unsafe { super::super::annotation_text::measure(&text)? };
        output.push((offset, size, text));
        offset += size.y.max(font.font_size()) * 1.25;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_line_and_the_whole_batch_are_centered_with_blank_line_spacing() {
        let text = TextAnnotation::new("fixture", "Arial", 16.0, false).unwrap();
        let rows = centered(
            vec![
                (0.0, Point { x: 20.0, y: 10.0 }, text.clone()),
                (20.0, Point { x: 40.0, y: 10.0 }, text),
            ],
            Point { x: 100.0, y: 80.0 },
        );
        assert_eq!(rows[0].0, Point { x: 90.0, y: 65.0 });
        assert_eq!(rows[1].0, Point { x: 80.0, y: 85.0 });
        assert_eq!((rows[0].0.y + rows[1].0.y + rows[1].1.y) / 2.0, 80.0);
    }
    #[test]
    fn history_lines_preserve_content_spacing_and_reject_without_partial_result() {
        let font = TextFont::default();
        let lines = prepare(" first\r\n\r\n中文😀\rlast ", &font).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].2.content(), " first");
        assert_eq!(lines[1].2.content(), "中文😀");
        assert_eq!(lines[2].2.content(), "last ");
        assert!(lines[1].0 > lines[0].1.y * 1.25);
        let long = "a".repeat(65536);
        let prepared = prepare(&long, &font).unwrap();
        assert_eq!(prepared.len(), 1);
        assert_eq!(prepared[0].2.content(), long);
        assert!(prepare(&format!("{long}a"), &font).is_err());
        assert!(prepare(&"a\n".repeat(256), &font).is_err());
        assert!(prepare("a\0b", &font).is_err());
    }
}

#[cfg(test)]
#[path = "popup_history_adoption_tests.rs"]
mod adoption_tests;

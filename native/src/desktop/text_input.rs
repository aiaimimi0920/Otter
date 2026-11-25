//! 文字输入事务；就地草稿不改文档，提交前核验文档身份与版本。
use super::canvas::Canvas;
use super::State;
use crate::{editor::Tool, text_annotation::TextAnnotation, viewport::Point};
use windows::{
    core::{w, HSTRING},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
#[path = "text_inline.rs"]
pub(super) mod inline;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "text_input_tests.rs"]
mod tests;

pub(super) struct Pending {
    index: Option<usize>,
    point: Point,
    pixels: crate::pixels::Pixels,
    stamp: crate::editor::DocumentStamp,
    text: Option<TextAnnotation>,
    tool: Tool,
    font: crate::text_annotation::TextFont,
    style: crate::stroke_style::StrokeStyle,
}
impl Pending {
    fn at_point(canvas: &mut Canvas, screen: Point) -> windows::core::Result<Option<Self>> {
        let Some(point) = canvas.image_point(screen, false) else {
            return Ok(None);
        };
        let index = unsafe {
            super::selection_hit::pick(
                &canvas.factory,
                &canvas.editor,
                point,
                5.0 / canvas.viewport.scale,
            )
        }?;
        let Some(index) = index.filter(|index| {
            let tool = canvas.editor.elements()[*index].tool;
            canvas.can_select_annotation(*index)
                && (tool == Tool::Text || (canvas.tool == Tool::Select && tool == Tool::Serial))
        }) else {
            return Ok(None);
        };
        canvas.selected = Some((index, canvas.editor.revision()));
        canvas.multi_selection = None;
        Ok(Self::selected(canvas))
    }
    pub(super) fn selected(canvas: &Canvas) -> Option<Self> {
        let index = canvas.selected_index()?;
        let stroke = &canvas.editor.elements()[index];
        let text = stroke.text.clone()?;
        if !matches!(stroke.tool, Tool::Text | Tool::Serial) {
            return None;
        }
        Some(Self {
            index: Some(index),
            point: stroke.points[0],
            pixels: canvas.image.as_ref()?.pixels.clone(),
            stamp: canvas.editor.document_stamp(),
            font: text.font().clone(),
            style: stroke.style(),
            text: Some(text),
            tool: canvas.tool,
        })
    }
    pub(super) fn commit(
        &self,
        canvas: &mut Canvas,
        text: TextAnnotation,
        size: Point,
    ) -> windows::core::Result<bool> {
        if !self.stamp.matches(&canvas.editor)
            || canvas.tool != self.tool
            || !canvas
                .image
                .as_ref()
                .is_some_and(|image| image.pixels.same_storage(&self.pixels))
        {
            return Ok(false);
        }
        if let Some(index) = self.index {
            // 没有内容/字体变更时不因重新测量改动手动调整过的文字框。
            if self.text.as_ref() == Some(&text) {
                return Ok(false);
            }
            let changed = canvas
                .editor
                .replace_text(index, size, text)
                .map_err(edit_error)?;
            if changed {
                canvas.selected = Some((index, canvas.editor.revision()));
            }
            Ok(changed)
        } else {
            let index = canvas
                .editor
                .insert_text_with_style(self.point, size, text, self.style)
                .map_err(edit_error)?;
            if let Some(index) = index {
                canvas.select_created_annotation(index);
            }
            Ok(index.is_some())
        }
    }
    pub(super) fn text(&self) -> Option<&TextAnnotation> {
        self.text.as_ref()
    }
}
fn edit_error(error: crate::editor::EditError) -> windows::core::Error {
    windows::core::Error::new(E_FAIL, format!("Text edit rejected: {error:?}"))
}
pub(super) unsafe fn edit_selected(hwnd: HWND, state: &State) {
    if !super::display_edit::prepare(state) {
        return;
    }
    let pending = {
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        Pending::selected(&canvas)
    };
    if let Some(pending) = pending {
        run(hwnd, state, pending);
    }
}
pub(super) unsafe fn edit_at(hwnd: HWND, state: &State, point: Point) {
    if !super::display_edit::prepare_text_at(state, point) {
        return;
    }
    let pending = {
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        Pending::at_point(&mut canvas, point)
    };
    match pending {
        Ok(Some(pending)) => run(hwnd, state, pending),
        Ok(None) => {}
        Err(error) => eprintln!("native-text-hit: {error}"),
    }
}
pub(super) unsafe fn insert(hwnd: HWND, state: &State, point: Point) {
    if !super::display_edit::prepare_text_at(state, point) {
        return;
    }
    let existing = {
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        Pending::at_point(&mut canvas, point)
    };
    match existing {
        Ok(Some(pending)) => {
            run(hwnd, state, pending);
            return;
        }
        Err(error) => {
            eprintln!("native-text-hit: {error}");
            return;
        }
        Ok(None) => {}
    }
    let pending = {
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        canvas.image_point(point, false).and_then(|point| {
            canvas.image.as_ref().map(|image| Pending {
                index: None,
                point,
                pixels: image.pixels.clone(),
                stamp: canvas.editor.document_stamp(),
                text: None,
                tool: Tool::Text,
                font: canvas.tool_profiles.text_font.clone(),
                style: canvas.editor.style(),
            })
        })
    };
    let Some(pending) = pending else {
        return;
    };
    run(hwnd, state, pending);
}
unsafe fn run(hwnd: HWND, state: &State, pending: Pending) {
    if let Err(error) = inline::begin(hwnd, state, pending) {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(format!("无法开始文字输入。\n{error}")),
            w!("Otter"),
            MB_OK | MB_ICONERROR,
        );
    }
}
unsafe fn commit_content(
    state: &State,
    pending: &Pending,
    content: &str,
) -> windows::core::Result<()> {
    if pending.text.as_ref().is_some_and(|text| {
        text.content().replace("\r\n", "\n").replace('\r', "\n")
            == content.replace("\r\n", "\n").replace('\r', "\n")
    }) {
        return Ok(());
    }
    let constructor = if pending.text.is_some() {
        TextAnnotation::restored
    } else {
        TextAnnotation::new
    };
    let Some(text) = constructor(
        content,
        pending.font.family(),
        pending.font.font_size(),
        pending.font.bold(),
    ) else {
        return Ok(());
    };
    let size = super::annotation_text::measure(&text)?;
    pending.commit(&mut state.canvas.borrow_mut(), text, size)?;
    Ok(())
}

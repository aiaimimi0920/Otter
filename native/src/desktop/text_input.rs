//! 文字输入事务；模态期间不借用 Canvas，提交前核验文档身份与版本。
use super::canvas::Canvas;
use super::State;
use crate::{editor::Tool, text_annotation::TextAnnotation, viewport::Point};
use windows::{
    core::{w, HSTRING},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};
#[cfg(test)]
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
            tool == Tool::Text || (canvas.tool == Tool::Select && tool == Tool::Serial)
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
            Ok(canvas
                .editor
                .insert_text(self.point, size, text)
                .map_err(edit_error)?
                .is_some())
        }
    }
    pub(super) fn text(&self) -> Option<&TextAnnotation> {
        self.text.as_ref()
    }
}
fn edit_error(error: crate::editor::EditError) -> windows::core::Error {
    windows::core::Error::new(E_FAIL, format!("Text edit rejected: {error:?}"))
}
pub(super) unsafe fn append_edit_menu(
    menu: HMENU,
    id: usize,
    canvas: &Canvas,
) -> windows::core::Result<()> {
    let enabled = canvas.selected_index().is_some_and(|index| {
        matches!(
            canvas.editor.elements()[index].tool,
            Tool::Text | Tool::Serial
        )
    });
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        id,
        w!("编辑选中文字…\tF2"),
    )?;
    super::text_font::append_menu(menu, canvas)
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
            })
        })
    };
    let Some(pending) = pending else {
        return;
    };
    run(hwnd, state, pending);
}
unsafe fn run(hwnd: HWND, state: &State, pending: Pending) {
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let initial = pending
        .text
        .as_ref()
        .map_or(String::new(), |text| text.content().to_owned());
    if initial.contains('\0') {
        MessageBoxW(
            Some(hwnd),
            w!("此文字含有输入控件无法表达的字符，原内容已保留。"),
            w!("Otter"),
            MB_OK | MB_ICONERROR,
        );
        return;
    }
    let Some(content) = super::shape_dialog::show_text(hwnd, initial) else {
        return;
    };
    if !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    if pending.text.as_ref().is_some_and(|text| {
        text.content().replace("\r\n", "\n").replace('\r', "\n")
            == content.replace("\r\n", "\n").replace('\r', "\n")
    }) {
        return;
    }
    let constructor = if pending.text.is_some() {
        TextAnnotation::restored
    } else {
        TextAnnotation::new
    };
    let Some(text) = constructor(
        &content,
        pending.font.family(),
        pending.font.font_size(),
        pending.font.bold(),
    ) else {
        return;
    };
    let result = (|| -> windows::core::Result<()> {
        let size = super::annotation_text::measure(&text)?;
        let mut canvas = state.canvas.borrow_mut();
        pending.commit(&mut canvas, text, size)?;
        Ok(())
    })();
    if let Err(error) = result {
        MessageBoxW(
            Some(hwnd),
            &HSTRING::from(format!("无法保存文字。\n{error}")),
            w!("Otter"),
            MB_OK | MB_ICONERROR,
        );
    }
    if IsWindow(Some(hwnd)).as_bool() {
        if let Err(error) = super::hit_region::update(hwnd, state) {
            eprintln!("native-text-region: {error}");
        }
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

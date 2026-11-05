//! 单选引用绑定文档版本；历史切换后旧索引失效，不能误删或修改另一个标注。
use super::canvas::Canvas;
use crate::{
    editor::{Stroke, Tool},
    viewport::Viewport,
};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

impl Canvas {
    pub fn duplicate_selected(&mut self) -> bool {
        self.cancel_gesture();
        let Some(index) = self.selected_index() else {
            return false;
        };
        match self.editor.duplicate(index) {
            Ok(Some(index)) => {
                self.selected = Some((index, self.editor.revision()));
                true
            }
            Ok(None) => false,
            Err(error) => {
                eprintln!("native-duplicate: {error:?}");
                false
            }
        }
    }
    pub fn rotate_selected(&mut self, degrees: f32) -> bool {
        self.cancel_gesture();
        let indices = self.selected_indices();
        if indices.len() > 1 {
            let Some(pivot) = crate::group_bounds::center(
                indices.iter().map(|index| &self.editor.elements()[*index]),
            ) else {
                return false;
            };
            return match self
                .editor
                .rotate_many_mode(&indices, pivot, degrees, self.transform_own)
            {
                Ok(true) => {
                    let revision = self.editor.revision();
                    self.selected = Some((indices[0], revision));
                    self.multi_selection = Some((indices, revision));
                    true
                }
                Ok(false) => false,
                Err(error) => {
                    eprintln!("native-group-rotate: {error:?}");
                    false
                }
            };
        }
        let Some(index) = self.selected_index() else {
            return false;
        };
        let stroke = &self.editor.elements()[index];
        let result = if matches!(stroke.tool, Tool::Blur | Tool::Mosaic) {
            let pivot = crate::rotation::center(&stroke.points);
            self.editor.rotate_many_around(&[index], pivot, degrees)
        } else {
            self.editor.rotate(index, degrees)
        };
        match result {
            Ok(true) => {
                self.selected = Some((index, self.editor.revision()));
                true
            }
            Ok(false) => false,
            Err(error) => {
                eprintln!("native-rotate: {error:?}");
                false
            }
        }
    }
    #[cfg(test)]
    pub fn resize_selected(&mut self, factor: f32) -> bool {
        self.resize_selected_axes(crate::viewport::Point {
            x: factor,
            y: factor,
        })
    }
    fn resize_group_selection(&mut self, factors: crate::viewport::Point) -> bool {
        let indices = self.selected_indices();
        if indices.len() > 1 {
            self.cancel_gesture();
            let Some(pivot) = crate::group_bounds::center(
                indices.iter().map(|index| &self.editor.elements()[*index]),
            ) else {
                return false;
            };
            let prepared = self.editor.prepare_resize_many_mode(
                &indices,
                pivot,
                factors,
                self.transform_own,
                |text| {
                    unsafe { super::annotation_text::measure(text) }.map_err(|error| {
                        eprintln!("native-group-text-layout: {error}");
                        crate::editor::EditError::InvalidPoint
                    })
                },
            );
            let result = prepared.and_then(|plan| match plan {
                Some(plan) => self.editor.commit_resize(plan),
                None => Ok(false),
            });
            return match result {
                Ok(true) => {
                    let revision = self.editor.revision();
                    self.selected = Some((indices[0], revision));
                    self.multi_selection = Some((indices, revision));
                    true
                }
                Ok(false) => false,
                Err(error) => {
                    eprintln!("native-group-resize: {error:?}");
                    false
                }
            };
        }
        false
    }
    pub fn resize_selected_axes(&mut self, factors: crate::viewport::Point) -> bool {
        if !factors.x.is_finite()
            || !factors.y.is_finite()
            || factors.x <= 0.0
            || factors.y <= 0.0
            || (factors.x == 1.0 && factors.y == 1.0)
        {
            return false;
        }
        self.cancel_gesture();
        if self
            .selected_group()
            .is_some_and(|indices| indices.len() > 1)
        {
            return self.resize_group_selection(factors);
        }
        let Some(index) = self.selected_index() else {
            return false;
        };
        // 文字不能直接拉伸包围框；按目标字号重新排版，失败时不改文档。
        let result = if self.editor.elements()[index].tool == Tool::Text {
            let Some(text) = self.editor.elements()[index]
                .text
                .as_ref()
                .and_then(|text| {
                    text.with_font_size(text.font_size() * ((factors.x + factors.y) / 2.0))
                })
            else {
                return false;
            };
            let size = match unsafe { super::annotation_text::measure(&text) } {
                Ok(size) => size,
                Err(error) => {
                    eprintln!("native-resize-text: {error}; document unchanged");
                    return false;
                }
            };
            self.editor.resize_text_axes(index, factors, size)
        } else {
            self.editor.resize_axes(index, factors)
        };
        match result {
            Ok(true) => {
                self.selected = Some((index, self.editor.revision()));
                true
            }
            Ok(false) => false,
            Err(error) => {
                eprintln!("native-resize: {error:?}; document unchanged");
                false
            }
        }
    }
    pub fn selected_index(&self) -> Option<usize> {
        if self
            .selected_group()
            .is_some_and(|indices| indices.len() > 1)
        {
            return None;
        }
        self.selected
            .filter(|(index, revision)| {
                *revision == self.editor.revision() && *index < self.editor.elements().len()
            })
            .map(|(index, _)| index)
    }
    pub fn delete_target(&mut self) -> bool {
        self.cancel_gesture();
        let changed = if self.tool == Tool::Select {
            let indices = self.selected_indices();
            self.editor.remove_many(&indices).unwrap_or_else(|error| {
                eprintln!("native-delete: {error:?}");
                false
            })
        } else {
            self.editor.remove_last()
        };
        if changed {
            self.selected = None;
            self.multi_selection = None;
        }
        changed
    }
    pub fn restyle_target(&mut self) -> bool {
        self.cancel_gesture();
        if self.tool != Tool::Select {
            return self.editor.restyle_last();
        }
        let indices = self.selected_indices();
        let changed = match self.editor.restyle_many(&indices, self.editor.style()) {
            Ok(changed) => changed,
            Err(error) => {
                eprintln!("native-group-style: {error:?}");
                false
            }
        };
        if changed {
            let revision = self.editor.revision();
            self.selected = indices.first().map(|index| (*index, revision));
            self.multi_selection = Some((indices, revision));
        }
        changed
    }
}

pub(super) unsafe fn draw_outline(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
) {
    let mut rect = D2D_RECT_F {
        left: f32::INFINITY,
        top: f32::INFINITY,
        right: f32::NEG_INFINITY,
        bottom: f32::NEG_INFINITY,
    };
    for point in &stroke.points {
        rect.left = rect.left.min(point.x);
        rect.right = rect.right.max(point.x);
        rect.top = rect.top.min(point.y);
        rect.bottom = rect.bottom.max(point.y);
    }
    let padding = if stroke.tool == Tool::Bezier && stroke.arrow {
        (stroke.width * 6.0).max(24.0) + (stroke.width * 5.0).max(16.0) / 2.0
    } else if stroke.has_straight_arrow() {
        (stroke.width * 5.0).max(16.0) / 2.0
    } else {
        stroke.width / 2.0
    };
    let rotated = crate::rotation::bounds(
        (
            rect.left - padding,
            rect.top - padding,
            rect.right + padding,
            rect.bottom + padding,
        ),
        crate::rotation::center(&stroke.points),
        stroke.rotation,
    );
    rect = D2D_RECT_F {
        left: rotated.0 + padding,
        top: rotated.1 + padding,
        right: rotated.2 - padding,
        bottom: rotated.3 - padding,
    };
    rect.left = viewport.origin.x + (rect.left - padding) * viewport.scale - 3.0;
    rect.right = viewport.origin.x + (rect.right + padding) * viewport.scale + 3.0;
    rect.top = viewport.origin.y + (rect.top - padding) * viewport.scale - 3.0;
    rect.bottom = viewport.origin.y + (rect.bottom + padding) * viewport.scale + 3.0;
    brush.SetColor(&D2D1_COLOR_F {
        r: 0.1,
        g: 0.8,
        b: 1.0,
        a: 1.0,
    });
    target.DrawRectangle(&rect, brush, 1.0, None);
}

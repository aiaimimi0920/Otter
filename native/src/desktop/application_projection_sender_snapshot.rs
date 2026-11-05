//! UI 线程只共享权威像素及已提交编辑；retained source 必须先匹配活动 editor。
use super::*;
use crate::desktop::canvas::minified_source::Origin;
use crate::desktop::{canvas::Canvas, display_frame::Frame};
#[cfg(test)]
#[path = "application_projection_sender_provenance_tests.rs"]
mod provenance_tests;

pub(in crate::desktop::application) struct Snapshot {
    pub image: Image,
    pub editor: Editor,
    pub opacity: f32,
    pub kind: crate::projection_delivery::sender::Kind,
    pub geometry: Option<crate::desktop::export::projection::Geometry>,
    pub source_logical: Option<crate::crop_geometry::Size>,
    pub natural_origin: Option<Origin>,
    // 相对于实际选中的像素owner；不是canonical历史crop，也不重复翻转自然坐标。
    pub legacy_pixels: Option<crate::crop_geometry::Rect>,
    pending_content: bool,
}
pub(super) struct Stamp {
    size: (u32, u32),
    pixels: std::sync::Weak<crate::pixels::Storage>,
    source_logical: Option<crate::crop_geometry::Size>,
    natural_origin: Option<(super::content::SourceStamp, crate::workspace_id::UnitId)>,
    legacy_pixels: Option<crate::crop_geometry::Rect>,
    pending_content: bool,
}
struct Selected {
    image: Image,
    logical: Option<crate::crop_geometry::Size>,
    origin: Option<Origin>,
    legacy_pixels: Option<crate::crop_geometry::Rect>,
    content: bool,
}
impl Snapshot {
    pub(super) fn capture(
        canvas: &Canvas,
        frame: Option<&Frame>,
        kind: crate::projection_delivery::sender::Kind,
        documents: &[crate::desktop::document_window::DocumentWindow],
    ) -> Option<Self> {
        if canvas.editor.draft().is_some() {
            return None;
        }
        // export_snapshot 会创建新的文档身份，不能在那之后再验证 Source::matches。
        let Selected {
            image,
            logical: source_logical,
            origin: natural_origin,
            legacy_pixels,
            content: pending_content,
        } = select(canvas, frame, documents)?;
        Some(Self {
            image,
            source_logical,
            natural_origin,
            legacy_pixels,
            pending_content,
            // resolved 的 editor 可能属于上游，只可证明底图，不能接管目标标注/裁剪。
            editor: canvas.editor.export_snapshot(),
            opacity: canvas.normal_opacity,
            kind,
            geometry: None,
        })
    }
    pub(super) fn stamp(&self) -> Stamp {
        Stamp {
            size: (self.image.width, self.image.height),
            pixels: self.image.pixels.downgrade(),
            source_logical: self.source_logical,
            natural_origin: self.natural_origin.as_ref().map(|origin| {
                (
                    super::content::SourceStamp::capture(&origin.record),
                    origin.unit.clone(),
                )
            }),
            legacy_pixels: self.legacy_pixels,
            pending_content: self.pending_content,
        }
    }
    pub(in crate::desktop::application) fn render_input(
        &self,
    ) -> crate::desktop::export::projection::Input {
        use crate::desktop::export::projection::{Baseline, Input};
        Input {
            logical: self
                .editor
                .full_logical_frame()
                .map(|frame| crate::crop_geometry::Size {
                    width: frame.width,
                    height: frame.height,
                })
                .or_else(|| {
                    // 普通非恢复Editor没有logical_size，但活动底图仍给出完整逻辑框。
                    // 不回退到可能尚未保存的canonical窗口尺寸，也不使用裁剪后的visible框。
                    self.editor
                        .layers()
                        .map(|layers| crate::crop_geometry::Size {
                            width: layers.size.0 as f64,
                            height: layers.size.1 as f64,
                        })
                }),
            source_logical: self.source_logical,
            baseline: self.natural_origin.as_ref().map(|origin| Baseline {
                record: origin.record.clone(),
                unit: origin.unit.clone(),
            }),
            // 恢复路径已物化flip；实时flip替换底图并使retained失效，不能重放canonical标志。
            flip: (false, false),
            historical_erase: self.natural_origin.is_some(),
            content: self.pending_content,
            // 旧式crop没有活动Editor凭证时不能从canonical复活；需独立恢复/重置合同。
            legacy_crop: false,
            legacy_pixels: self.legacy_pixels,
        }
    }
}
impl Stamp {
    pub(super) fn matches(
        &self,
        canvas: &Canvas,
        frame: Option<&Frame>,
        documents: &[crate::desktop::document_window::DocumentWindow],
    ) -> bool {
        select(canvas, frame, documents).is_some_and(
            |Selected {
                 image,
                 logical,
                 origin,
                 legacy_pixels,
                 content,
             }| {
                self.size == (image.width, image.height)
                    && self.pixels.ptr_eq(&image.pixels.downgrade())
                    && self.source_logical == logical
                    && self.pending_content == content
                    && self.legacy_pixels == legacy_pixels
                    && match (&self.natural_origin, &origin) {
                        (None, None) => true,
                        (Some((record, unit)), Some(b)) => {
                            *unit == b.unit && record.matches(&b.record)
                        }
                        _ => false,
                    }
            },
        )
    }
}
fn select(
    canvas: &Canvas,
    frame: Option<&Frame>,
    documents: &[crate::desktop::document_window::DocumentWindow],
) -> Option<Selected> {
    if let Some(frame) = frame.filter(|frame| frame.has_local_source()) {
        let source = documents
            .iter()
            // DocumentWindow 由 UI 线程拥有；这里只查询现有 HWND，不分发消息。
            .find(|doc| frame.is_from(&doc.state.unit_id) && unsafe { doc.is_alive() })?;
        let upstream = source.state.canvas.borrow();
        return select_local(frame, &source.state.unit_id, &upstream);
    }
    let (image, editor) = if let Some(frame) = frame {
        // resolved 像素/底图可能属于上游；目标 retained source 不能替换它。
        (&frame.image, frame.editor.as_deref())
    } else {
        if let Some(source) = canvas
            .minified_source
            .as_ref()
            .filter(|source| source.matches(&canvas.editor))
        {
            return Some(Selected {
                image: source.image().clone(),
                logical: source.source_logical(&canvas.editor),
                origin: source.origin(&canvas.editor).cloned(),
                legacy_pixels: canvas
                    .editor
                    .crop_view()
                    .and_then(|view| view.legacy_pixel_region()),
                content: true,
            });
        }
        (canvas.image.as_ref()?, Some(&canvas.editor))
    };
    // 底图擦除会替换 layers.base，但宿主 image 仍可能是旧图；不从宿主复活内容。
    let image = editor.and_then(Editor::layers).map_or_else(
        || image.clone(),
        |layers| Image {
            width: layers.size.0,
            height: layers.size.1,
            pixels: layers.base.clone(),
        },
    );
    Some(Selected {
        image,
        logical: None,
        origin: None,
        legacy_pixels: editor
            .and_then(Editor::raster_crop_view)
            .and_then(|view| view.legacy_pixel_region()),
        content: frame.is_some() || canvas.editor.layers().is_none(),
    })
}

fn select_local(
    frame: &Frame,
    unit: &crate::workspace_id::UnitId,
    upstream: &Canvas,
) -> Option<Selected> {
    let image = frame.natural_source(unit, upstream)?;
    // source_logical属于自然源，不属于目标框。渲染入口使用目标当前logical逐轴
    // 归一化crop，再映回原sourceSize；目标非等比缩放不应丢弃原图密度凭证。
    Some(Selected {
        image,
        logical: frame.natural_source_logical(unit, upstream),
        origin: frame.natural_source_origin(unit, upstream),
        legacy_pixels: if upstream
            .minified_source
            .as_ref()
            .is_some_and(|source| source.matches(&upstream.editor))
        {
            upstream.editor.crop_view()
        } else {
            upstream.editor.raster_crop_view()
        }
        .and_then(|view| view.legacy_pixel_region()),
        content: upstream
            .minified_source
            .as_ref()
            .is_some_and(|source| source.matches(&upstream.editor)),
    })
}

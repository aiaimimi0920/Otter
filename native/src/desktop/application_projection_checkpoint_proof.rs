//! canonical checkpoint 的显示凭证；与原保存提交同步提升，不以 dirty Stamp 猜测图片/输出。
use super::*;
use crate::{projection_delivery::binding::checkpoint::Index, workspace_id::UnitId};

pub(super) struct Document {
    id: UnitId,
    guard: restore_batch::guard::Document,
    // 只共享原受预算像素；下一次提交整体替换，最多 MAX_ENTRIES 项，不复制或解码。
    image: crate::image_decode::Image,
    source: crate::image_decode::Image,
}
pub(super) type Documents = Option<Vec<Document>>;
pub(in crate::desktop::application) fn source(
    canvas: &crate::desktop::Canvas,
) -> Option<&crate::image_decode::Image> {
    canvas
        .minified_source
        .as_ref()
        .filter(|source| source.matches(&canvas.editor))
        .map(|source| source.image())
        .or(canvas.image.as_ref())
}
fn same(a: &crate::image_decode::Image, b: &crate::image_decode::Image) -> bool {
    a.width == b.width && a.height == b.height && a.pixels.same_storage(&b.pixels)
}
pub(super) unsafe fn capture(app: &Application, index: Option<&Index>) -> Documents {
    let index = index?;
    app.documents
        .iter()
        .filter(|doc| index.for_unit(doc.state.unit_id.as_str()).is_some())
        .map(|doc| {
            let guard = restore_batch::guard::Document::capture(doc)?;
            let canvas = doc.state.canvas.borrow();
            Ok(Document {
                id: doc.state.unit_id.clone(),
                guard,
                image: canvas
                    .image
                    .as_ref()
                    .ok_or_else(|| Error::new(E_ABORT, "Missing checkpoint display"))?
                    .clone(),
                source: source(&canvas)
                    .ok_or_else(|| Error::new(E_ABORT, "Missing checkpoint source"))?
                    .clone(),
            })
        })
        .collect::<Result<Vec<_>>>()
        .ok()
}
impl Owner {
    pub(in crate::desktop::application) unsafe fn projection_model_current(
        &self,
        app: &Application,
        unit: &str,
    ) -> bool {
        !self.stopped
            && !self.exiting
            && self.exit.is_none()
            && self
                .projection_workspace
                .as_ref()
                .is_some_and(|id| id.matches(&app.workspace))
            && self.stamp.as_ref().is_some_and(|(stamp, source)| {
                stamp.matches_automatic(app)
                    && app
                        .restored_session
                        .as_ref()
                        .is_some_and(|record| source.same_storage(&record.original))
            })
            && self.projection_display.as_ref().is_some_and(|documents| {
                documents.iter().any(|saved| {
                    saved.id.as_str() == unit
                        && app
                            .documents
                            .iter()
                            .find(|doc| doc.state.unit_id == saved.id)
                            .is_some_and(|doc| {
                                let canvas = doc.state.canvas.borrow();
                                saved.guard.matches(doc)
                                    && canvas
                                        .image
                                        .as_ref()
                                        .is_some_and(|image| same(image, &saved.image))
                                    && source(&canvas)
                                        .is_some_and(|image| same(image, &saved.source))
                            })
                })
            })
    }
}

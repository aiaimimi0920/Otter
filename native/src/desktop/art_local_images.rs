//! 本地 Art 复用图输入准备 worker；资源只沿既有 mount 通道交付，不访问任意路径/URL。
//! 退休先取消，终态前仍占原 Entry；像素凭证在每次帧/正式结果领取时重验。
use super::*;
use crate::desktop::{
    application::Application,
    loom_input_job::{Freshness, Job, PreparedBatch},
};
use crate::live_relay_http::art_prepare::Prepared;
use crate::surface_javascript::signed::runtime::Content;
use serde_json::{json, Map, Value};

#[derive(Default)]
pub(super) struct Images {
    job: Job,
    prepared: Option<Prepared>,
    freshness: Option<Freshness>,
}
impl Images {
    pub fn start(
        &mut self,
        prepared: Prepared,
        documents: &[DocumentWindow],
        sessions: &[LiveSession],
        workspace: &Workspace,
        unit: &UnitId,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<(), &'static str> {
        let inputs =
            Application::image_inputs(documents, sessions, workspace, unit, true, None, None)?
                .ok_or("Art 图片输入尚未就绪")?;
        let document = documents
            .iter()
            .find(|doc| &doc.state.unit_id == unit)
            .ok_or("Art 目标已关闭")?;
        self.job
            .start_with_notify(
                unit.clone(),
                &document.state.canvas.borrow().editor,
                inputs.sources,
                notify,
            )
            .map_err(|_| "Art 图片准备超过预算或正在回收")?;
        self.prepared = Some(prepared);
        Ok(())
    }
    pub fn poll(&mut self) -> Option<Result<(Prepared, Content), &'static str>> {
        let result = self.job.poll()?;
        Some((|| {
            let batch = result.map_err(|_| "Art 图片输入准备失败")?;
            let prepared = self.prepared.take().ok_or("Art 图片准备已取消")?;
            let freshness = batch.freshness().map_err(|_| "无法保留 Art 图片凭证")?;
            let content = content(&batch)?;
            self.freshness = Some(freshness);
            Ok((prepared, content))
        })())
    }
    pub fn current(
        &self,
        documents: &[DocumentWindow],
        sessions: &[LiveSession],
        workspace: &Workspace,
    ) -> bool {
        self.freshness.as_ref().is_none_or(|freshness| {
            freshness.is_current(|unit, stamp| {
                workspace.contains(unit)
                    && stamp.matches_owners(unit, documents, sessions, workspace)
                    && documents
                        .iter()
                        .filter(|doc| &doc.state.unit_id == unit)
                        .all(|doc| {
                            (unsafe { doc.is_alive() })
                                && doc.state.pending_adoption.borrow().is_none()
                        })
            })
        })
    }
    pub fn busy(&self) -> bool {
        self.job.is_busy()
    }
    pub fn cancel(&mut self) {
        self.job.cancel();
        self.prepared.take();
        self.freshness.take();
    }
}

fn content(batch: &PreparedBatch) -> Result<Content, &'static str> {
    // 与 Session::resources 的同一门限；先估算再分配Data URL，不缩图、不截断或提高wire上限。
    const LIMIT: usize = crate::surface_javascript::MAX_RESOURCES_BYTES;
    let mut resources = Map::new();
    let mut inputs = Map::new();
    let mut images = Vec::new();
    let mut bytes = 2usize;
    for (index, ((port, descriptor), png)) in batch.descriptors().zip(batch.pngs()).enumerate() {
        let crate::loom_art_wire::Input::InlinePng {
            base64,
            width,
            height,
        } = descriptor
        else {
            return Err("Art 图片输入不是已准备的本地 PNG");
        };
        let id = format!("art-input:{index}");
        let next = bytes
            .checked_add(id.len() + base64.len() + 22 + 6)
            .ok_or("Art 图片资源超过本地通道预算")?;
        // 小图保留已有resource()合同；较大图片只能经image()句柄消费，不把Base64塞入JSON。
        if next <= LIMIT {
            resources.insert(
                id.clone(),
                Value::String(format!("data:image/png;base64,{base64}")),
            );
            bytes = next;
        }
        images.push(crate::surface_javascript::ImageResource {
            id: id.clone(),
            width,
            height,
            png,
        });
        inputs.insert(port.into(), json!({"kind":"image","resourceId":id,"mimeType":"image/png","width":width,"height":height}));
    }
    let resources = Value::Object(resources);
    crate::surface_javascript::resources(&resources).map_err(|_| "Art 图片资源超过本地通道预算")?;
    Ok(Content {
        snapshot: json!({"revision":1,"scene":{"id":"root","type":"column","children":[]},"state":{"inputs":inputs}}),
        resources,
        images,
    })
}
#[cfg(test)]
#[path = "art_local_images_tests.rs"]
mod tests;

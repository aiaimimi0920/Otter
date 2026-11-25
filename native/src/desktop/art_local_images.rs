//! Local Art reuses the graph-input preparation worker; resources travel only through the existing mount channel, never arbitrary paths/URLs.
//! Cancel before retirement, retaining the original Entry until terminal state; revalidate pixel evidence whenever collecting frames/official results.
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
                .ok_or(crate::ui_text::text("integration-art-image-not-ready"))?;
        let document = documents
            .iter()
            .find(|doc| &doc.state.unit_id == unit)
            .ok_or(crate::ui_text::text("integration-art-target-closed"))?;
        self.job
            .start_with_notify(
                unit.clone(),
                &document.state.canvas.borrow().editor,
                inputs.sources,
                notify,
            )
            .map_err(|_| crate::ui_text::text("integration-art-image-budget"))?;
        self.prepared = Some(prepared);
        Ok(())
    }
    pub fn poll(&mut self) -> Option<Result<(Prepared, Content), &'static str>> {
        let result = self.job.poll()?;
        Some((|| {
            let batch =
                result.map_err(|_| crate::ui_text::text("integration-art-image-prepare-failed"))?;
            let prepared = self
                .prepared
                .take()
                .ok_or(crate::ui_text::text("integration-art-image-cancelled"))?;
            let freshness = batch
                .freshness()
                .map_err(|_| crate::ui_text::text("integration-art-image-proof-failed"))?;
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
    // Use the Session::resources limit; estimate before allocating Data URLs, without resizing, truncating, or raising wire limits.
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
            return Err(crate::ui_text::text("integration-art-image-not-local-png"));
        };
        let id = format!("art-input:{index}");
        let next = bytes
            .checked_add(id.len() + base64.len() + 22 + 6)
            .ok_or(crate::ui_text::text("integration-art-image-channel-budget"))?;
        // Small images retain the existing resource() contract; larger images require image() handles, not Base64 embedded in JSON.
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
    crate::surface_javascript::resources(&resources)
        .map_err(|_| crate::ui_text::text("integration-art-image-channel-budget"))?;
    Ok(Content {
        snapshot: json!({"revision":1,"scene":{"id":"root","type":"column","children":[]},"state":{"inputs":inputs}}),
        resources,
        images,
    })
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "art_local_images_tests.rs"]
mod tests;

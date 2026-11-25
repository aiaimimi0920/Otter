//! Batch preparation of hidden windows: any failure releases this batch's owners without changing the existing workspace or showing candidates.
use super::*;
#[path = "application_append.rs"]
pub(super) mod append;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_frozen_append_smoke.rs"]
mod frozen_append_smoke;
#[path = "application_restore_guard.rs"]
pub(super) mod guard;
#[path = "application_restore_job.rs"]
mod job;
#[path = "application_restore_manifest.rs"]
mod manifest;
#[path = "application_workflow.rs"]
pub(super) mod workflow;
pub(super) use job::Job;
#[path = "application_restore_publish.rs"]
mod publish;
pub(super) use publish::transfer_hotkey;
pub(super) use publish::Record;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_restore_batch_smoke.rs"]
pub(super) mod smoke;
use crate::desktop::{
    capability_job::Loaded,
    session_binding::Prepared,
    session_candidate::{Candidate, Placement},
};
use std::{
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Windows {
    origin: guard::Guard,
    documents: Vec<DocumentWindow>,
    ready: Vec<guard::Document>,
    candidate: Candidate,
    catalog: Option<Rc<Loaded>>,
    annotation_ids: Vec<(crate::workspace_id::UnitId, Vec<String>)>,
}
impl Windows {
    pub unsafe fn matches_application(&self, app: &Application) -> bool {
        self.origin.matches(app)
            && self.ready.len() == self.documents.len()
            && self
                .ready
                .iter()
                .zip(&self.documents)
                .all(|(stamp, doc)| !IsWindowVisible(doc.window.0).as_bool() && stamp.matches(doc))
            && crate::desktop::session_binding::catalog_matches(
                &self.catalog,
                app.catalog.borrow().loaded.as_ref(),
            )
    }
    pub fn window_count(&self) -> usize {
        self.documents.len()
    }
}
impl Application {
    pub unsafe fn prepare_restored_batch(
        &self,
        prepared: Prepared,
        cancel: &AtomicBool,
        mut physical_frame: impl FnMut(&Placement) -> Result<crate::pin_geometry::PinRect>,
    ) -> Result<Windows> {
        manifest::validate(&prepared)?;
        let origin = guard::Guard::capture(self)?;
        let check = || -> Result<()> {
            if cancel.load(Ordering::Acquire) || self.modals.blocked() || self.exit_requested.get()
            {
                return Err(Error::new(
                    E_ABORT,
                    "Session window preparation cancelled or blocked",
                ));
            }
            if !origin.matches(self) {
                return Err(Error::new(
                    E_ABORT,
                    "Workspace changed during session preparation",
                ));
            }
            if !crate::desktop::session_binding::catalog_matches(
                &prepared.catalog,
                self.catalog.borrow().loaded.as_ref(),
            ) {
                return Err(Error::new(E_ABORT, "Session capability catalog changed"));
            }
            Ok(())
        };
        check()?;
        let mut documents = Vec::with_capacity(prepared.states.len());
        let mut ready = Vec::with_capacity(prepared.states.len());
        // manifest::validate already checked length and matching identity order; do not rescan the position table for every hidden window.
        for (state, placement) in prepared
            .states
            .into_iter()
            .zip(&prepared.candidate.placements)
        {
            check()?;
            let frame = physical_frame(placement)?;
            check()?;
            if placement.minified {
                // If the main record lacks savedRect, only leave minimized mode and retain the current frame; an expansion transaction is still required.
                let mut expanded = placement.clone();
                expanded.rect = placement
                    .expanded_rect()
                    .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid expanded placement"))?;
                expanded.minified = false;
                expanded.minified_views = None;
                let expanded_frame = physical_frame(&expanded)?;
                check()?;
                state.canvas.borrow_mut().minified_restore =
                    Some(crate::minified_transition::Transition {
                        initial: frame,
                        expanded: expanded_frame,
                        opacity: placement.opacity_normal as f32,
                        persist_expanded: placement.saved_rect.is_some(),
                    });
            }
            let document = self.prepare_restored_window(state, frame)?;
            ready.push(guard::Document::capture(&document)?);
            documents.push(document);
        }
        check()?;
        Ok(Windows {
            origin,
            documents,
            ready,
            candidate: prepared.candidate,
            catalog: prepared.catalog,
            annotation_ids: prepared.annotation_ids,
        })
    }
}

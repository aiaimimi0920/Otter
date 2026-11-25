//! Workflow archive asset batches; publish images individually and hold file handles, delivering complete references/indices only on success.
use super::{
    publish::{Published, Slot},
    Root,
};
use crate::session_document::workflow_snapshot::metadata::{archive, Plan};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG},
};

pub struct Asset {
    pub sticker_id: String,
    pub src: String,
    pub preview_src: Option<String>,
}
pub struct Batch<'a> {
    assets: Vec<Asset>,
    index: serde_json::Value,
    adoptions: &'a [crate::session_document::workflow_snapshot::metadata::Adoption],
    // Candidates can be generated only from the same original text verified during asset publication; different bytes cannot be supplied.
    saved: &'a [u8],
    // Keep Batch alive while session candidates use references; release asset file handles last.
    _pins: Vec<Published>,
}
pub struct Prepared {
    candidate: crate::session_write::Candidate,
    _pins: Vec<Published>,
}
impl Prepared {
    /// Lend only the session candidate; Prepared retains asset handles until the caller finishes disk commit.
    pub fn candidate(&self) -> &crate::session_write::Candidate {
        &self.candidate
    }
}
impl Asset {
    fn reference(&self) -> archive::Asset<'_> {
        archive::Asset {
            sticker_id: &self.sticker_id,
            src: &self.src,
            preview_src: self.preview_src.as_deref(),
        }
    }
}
impl Batch<'_> {
    pub fn into_candidate(self, cancel: &AtomicBool) -> Result<Prepared> {
        check(cancel)?;
        let references: Vec<_> = self.assets.iter().map(Asset::reference).collect();
        let candidate = crate::session_write::archive::prepare(
            self.saved,
            self.index,
            &references,
            self.adoptions,
            cancel,
        )
        .map_err(|_| invalid("Cannot assemble archived session candidate"))?;
        Ok(Prepared {
            candidate,
            _pins: self._pins,
        })
    }
    pub fn assets(&self) -> &[Asset] {
        &self.assets
    }
    pub fn index(&self) -> &serde_json::Value {
        &self.index
    }
}
pub fn prepare<'a>(
    root: &Root,
    plan: &'a Plan,
    saved: &'a [u8],
    timestamp_ms: u64,
    cancel: &AtomicBool,
) -> Result<Batch<'a>> {
    let mut source = archive::Source::read(plan, saved, cancel).map_err(|error| match error {
        crate::session_document::workflow_snapshot::Error::Cancelled => {
            Error::new(E_ABORT, "Archive batch cancelled")
        }
        crate::session_document::workflow_snapshot::Error::Revision => {
            invalid("Archive source no longer matches workflow plan")
        }
        _ => invalid("Invalid archive source"),
    })?;
    let mut assets = Vec::new();
    let mut pins = Vec::new();
    for sticker in source.take_stickers() {
        check(cancel)?;
        let src = publish(
            root,
            &sticker.id,
            Slot::Source,
            &sticker.src,
            &mut pins,
            cancel,
        )?;
        let preview_src = sticker
            .preview_src
            .as_ref()
            .map(|reference| {
                publish(
                    root,
                    &sticker.id,
                    Slot::Preview,
                    reference,
                    &mut pins,
                    cancel,
                )
            })
            .transpose()?;
        assets.push(Asset {
            sticker_id: sticker.id.into_owned(),
            src,
            preview_src,
        });
    }
    let references: Vec<_> = assets.iter().map(Asset::reference).collect();
    let index = source
        .index(&references, timestamp_ms, cancel)
        .map_err(|_| invalid("Cannot prepare workflow archive index"))?;
    check(cancel)?;
    Ok(Batch {
        assets,
        index,
        adoptions: plan.adoptions(),
        saved,
        _pins: pins,
    })
}
fn publish(
    root: &Root,
    id: &str,
    slot: Slot,
    reference: &str,
    pins: &mut Vec<Published>,
    cancel: &AtomicBool,
) -> Result<String> {
    if !reference.starts_with("data:image") {
        // Retain ordinary references as mainline does; never implicitly access arbitrary files/URLs or claim their content was archived.
        if reference.len() > 32 * 1024 || reference.contains('\0') {
            return Err(invalid("Archive reference exceeds bounds"));
        }
        return Ok(reference.into());
    }
    let resource = root.publish_inline(id, slot, reference, cancel)?;
    let reference = resource.reference().to_owned();
    pins.push(resource);
    Ok(reference)
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Archive batch cancelled"))
    } else {
        Ok(())
    }
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_asset_archive_tests.rs"]
mod tests;

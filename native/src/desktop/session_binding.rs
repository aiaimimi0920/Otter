//! UI-thread session-catalog binding: share rather than copy the catalog; old-catalog results cannot commit to a new catalog.
use super::{
    capability_job::Loaded,
    session_candidate::{Candidate, PreparedRestore, VectorCandidate},
};
use crate::session_art_batch::Entry;
use std::{
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::core::Result;
#[path = "session_states.rs"]
mod states;
pub use states::Prepared;

pub struct Bound {
    projection_edits: Vec<(
        crate::workspace_id::UnitId,
        super::projection_edit_workspace::Owner,
    )>,
    regions: Vec<(crate::workspace_id::UnitId, super::hit_region::HitRegion)>,
    candidate: Candidate,
    catalog: Option<Rc<Loaded>>,
    arts: Vec<Entry>,
    results: Vec<super::loom_result_state::Results>,
    vectors: Vec<(
        crate::workspace_id::UnitId,
        crate::session_vectors::Restored,
    )>,
}
/// Resources for subsequent window transactions after successful verification. The catalog accompanies results; never interpret capability_index across versions.
pub struct Parts {
    pub(in crate::desktop) projection_edits: Vec<(
        crate::workspace_id::UnitId,
        super::projection_edit_workspace::Owner,
    )>,
    pub(in crate::desktop) regions:
        Vec<(crate::workspace_id::UnitId, super::hit_region::HitRegion)>,
    pub candidate: Candidate,
    pub catalog: Option<Rc<Loaded>>,
    pub arts: Vec<Entry>,
    pub results: Vec<super::loom_result_state::Results>,
    pub vectors: Vec<(
        crate::workspace_id::UnitId,
        crate::session_vectors::Restored,
    )>,
}
impl Bound {
    /// Parsing only borrows the candidate; failure returns the original owner to the caller, while success transfers it to the binding owner.
    pub fn prepare(
        prepared: VectorCandidate,
        catalog: Rc<Loaded>,
        cancel: &AtomicBool,
    ) -> std::result::Result<Self, Box<(VectorCandidate, windows::core::Error)>> {
        Self::prepare_for(prepared, Some(catalog), cancel)
    }
    pub fn prepare_for(
        prepared: VectorCandidate,
        catalog: Option<Rc<Loaded>>,
        cancel: &AtomicBool,
    ) -> std::result::Result<Self, Box<(VectorCandidate, windows::core::Error)>> {
        let handshake = catalog.as_ref().map(|catalog| &catalog.handshake);
        match prepared.candidate.prepare_arts(handshake, cancel) {
            Ok(arts) => Ok(Self::from_prepared(
                PreparedRestore {
                    vector: prepared,
                    arts,
                },
                catalog,
            )),
            Err(error) => Err(Box::new((prepared, error))),
        }
    }

    /// The caller still holds the preparation-time catalog owner; check current catalog identity before publication.
    pub(in crate::desktop) fn from_prepared(
        prepared: PreparedRestore,
        catalog: Option<Rc<Loaded>>,
    ) -> Self {
        Self {
            projection_edits: prepared.vector.projection_edits,
            candidate: prepared.vector.candidate,
            vectors: prepared.vector.vectors,
            regions: prepared.vector.regions,
            catalog,
            arts: prepared.arts.arts,
            results: prepared.arts.results,
        }
    }
    pub fn candidate(&self) -> &Candidate {
        &self.candidate
    }
    pub fn arts(&self) -> &[Entry] {
        &self.arts
    }
    pub fn matches(&self, current: &Rc<Loaded>) -> bool {
        catalog_matches(&self.catalog, Some(current))
    }
    /// Failure returns the original owner for rebinding or abandonment; neither lose the original session nor partially deliver nodes.
    pub fn into_parts(
        self,
        current: &Rc<Loaded>,
        cancel: &AtomicBool,
    ) -> std::result::Result<Parts, Box<Self>> {
        self.into_parts_for(Some(current), cancel)
    }
    pub fn into_parts_for(
        self,
        current: Option<&Rc<Loaded>>,
        cancel: &AtomicBool,
    ) -> std::result::Result<Parts, Box<Self>> {
        if cancel.load(Ordering::Acquire) || !catalog_matches(&self.catalog, current) {
            return Err(Box::new(self));
        }
        Ok(Parts {
            projection_edits: self.projection_edits,
            candidate: self.candidate,
            catalog: self.catalog,
            arts: self.arts,
            results: self.results,
            vectors: self.vectors,
            regions: self.regions,
        })
    }
    /// Catalog changes require reparsing the whole batch, not retaining old secret policies or merely replacing the catalog pointer.
    pub fn rebind(&mut self, catalog: Rc<Loaded>, cancel: &AtomicBool) -> Result<()> {
        let arts = self
            .candidate
            .resolve_arts(Some(&catalog.handshake), cancel)?;
        self.catalog = Some(catalog);
        self.arts = arts;
        Ok(())
    }
}

/// An offline snapshot is also an exact state; None is not interchangeable with an actual catalog that appears later.
pub(in crate::desktop) fn catalog_matches(
    snapshot: &Option<Rc<Loaded>>,
    current: Option<&Rc<Loaded>>,
) -> bool {
    match (snapshot, current) {
        (None, None) => true,
        (Some(snapshot), Some(current)) => Rc::ptr_eq(snapshot, current),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_hdr_original_reaches_restored_document_and_can_be_saved_again() {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let cancel = AtomicBool::new(false);
        let bytes: Vec<_> = [0x4000u16; 4]
            .iter()
            .flat_map(|n| n.to_le_bytes())
            .collect();
        let frame = crate::capture_color::ScRgb::new(&bytes, 1, 1, 8)
            .unwrap()
            .to_hdr(0.01, 1000.0, 1.0, &cancel)
            .unwrap();
        let mut png = Vec::new();
        crate::capture_png::write(&mut png, &frame, &cancel).unwrap();
        let encoded = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
        let json = serde_json::json!({"stickers":[{"id":"capture","x":0,"y":0,"w":20,"h":20,
            "captureMeta":{"dynamicRange":"hdr","bitDepth":16,"colorSpace":"bt2020-pq"},
            "otterNativeCaptureOriginalSrc":encoded.as_str()}],"links":[]});
        let candidate = super::super::session_candidate::prepare(
            serde_json::to_vec(&json).unwrap(),
            None,
            &cancel,
        )
        .unwrap();
        let vectors = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let bound = Bound::prepare_for(vectors, None, &cancel).ok().unwrap();
        let prepared = bound
            .into_parts_for(None, &cancel)
            .ok()
            .unwrap()
            .prepare_states(&cancel)
            .unwrap();
        let canvas = prepared.states[0].canvas.borrow();
        let source = canvas.capture_source().unwrap();
        assert_eq!(source.original_label(), "HDR PNG");
        assert!(source.original.is_none()); // Do not fabricate an accessible external source-image path from inline resources.
        let file = source.original_lease.as_ref().unwrap();
        assert_eq!(
            crate::capture_file::encode_inline(file, &cancel)
                .unwrap()
                .as_str(),
            encoded.as_str()
        );
    }
    #[test]
    fn restored_capture_metadata_reaches_document_without_opening_original_or_granting_export() {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let cancel = AtomicBool::new(false);
        let candidate = super::super::session_candidate::prepare(br#"{"stickers":[{"id":"capture","x":0,"y":0,"w":20,"h":20,"captureMeta":{"dynamicRange":"hdr","bitDepth":16,"colorSpace":"bt2020-pq","extension":{"keep":true}}}],"links":[]}"#.to_vec(), None, &cancel).unwrap();
        let vectors = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let bound = Bound::prepare_for(vectors, None, &cancel).ok().unwrap();
        let parts = bound.into_parts_for(None, &cancel).ok().unwrap();
        let prepared = parts.prepare_states(&cancel).unwrap();
        let canvas = prepared.states[0].canvas.borrow();
        let source = canvas.capture_source().unwrap();
        assert_eq!(source.original_label(), "HDR（原图文件未恢复）");
        assert!(source.original_lease.is_none());
        assert!(source.original.is_none());
        assert_eq!(source.metadata()["extension"]["keep"], true);
    }
    use crate::session_art_batch::Binding;
    fn catalog(secret: bool) -> Rc<Loaded> {
        let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
        let mut value = crate::loom_handshake::tests::fixture();
        value["capabilities"]["artDefinitions"] = serde_json::json!([
            {"id":"a","parameters":[{"id":"value","secret":secret}],"inputs":[],"outputs":[]}
        ]);
        Rc::new(Loaded {
            identity,
            handshake: crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .unwrap(),
        })
    }
    fn candidate() -> Candidate {
        super::super::session_candidate::prepare(br#"{"stickers":[{"id":"u","type":"art","artId":"a","x":0,"y":0,"w":1,"h":1,"params":{"value":42}}],"links":[]}"#.to_vec(), None, &AtomicBool::new(false)).unwrap()
    }
    #[test]
    fn offline_binding_retains_art_identity_and_refuses_catalog_substitution() {
        let cancel = AtomicBool::new(false);
        let prepared = VectorCandidate::prepare(candidate(), &cancel).unwrap();
        let bound = Bound::prepare_for(prepared, None, &cancel).ok().unwrap();
        assert!(
            matches!(&bound.arts()[0].binding, Binding::Missing { art_id: Some(id) } if id == "a")
        );
        let original = bound.candidate().original_json().to_vec();
        let current = catalog(false);
        let bound = *bound.into_parts_for(Some(&current), &cancel).err().unwrap();
        assert_eq!(bound.candidate().original_json(), original);
        let parts = bound.into_parts_for(None, &cancel).ok().unwrap();
        assert!(parts.catalog.is_none());
        let prepared = parts.prepare_states(&cancel).unwrap();
        assert!(prepared.catalog.is_none());
        assert!(prepared.states[0].selected_art.borrow().is_none());
        assert!(prepared.states[0].is_art());
        crate::desktop::naming_context::with_context(&prepared.states[0], (1, 1), |context| {
            assert_eq!(context.process, "a");
            assert_eq!(context.label, "art");
            assert!(context.title.is_empty());
        })
        .unwrap();
        assert!(prepared.states[0]
            .execution_results
            .borrow()
            .active()
            .is_none());
        assert_eq!(prepared.candidate.original_json(), original);
        assert!(!catalog_matches(&Some(current), None));
    }
    #[test]
    fn catalog_refresh_requires_reparse_and_preserves_candidate_until_commit() {
        let old = catalog(false);
        let current = catalog(true);
        let cancel = AtomicBool::new(false);
        let prepared = VectorCandidate::prepare(candidate(), &cancel).unwrap();
        let mut bound = Bound::prepare(prepared, old.clone(), &cancel).ok().unwrap();
        let original = bound.candidate().original_json().to_vec();
        let Binding::Ready { restored, .. } = &bound.arts()[0].binding else {
            panic!("ready")
        };
        assert!(restored.parameters.get("value").is_some());
        bound = *bound.into_parts(&current, &cancel).err().unwrap();
        assert_eq!(bound.candidate().original_json(), original);
        assert!(bound.matches(&old));
        cancel.store(true, Ordering::Release);
        assert!(bound.rebind(current.clone(), &cancel).is_err());
        assert!(bound.matches(&old));
        bound = *bound.into_parts(&old, &cancel).err().unwrap();
        cancel.store(false, Ordering::Release);
        bound.rebind(current.clone(), &cancel).unwrap();
        let Binding::Ready { restored, .. } = &bound.arts()[0].binding else {
            panic!("ready")
        };
        assert!(restored.parameters.get("value").is_none());
        let parts = bound.into_parts(&current, &cancel).ok().unwrap();
        assert!(Rc::ptr_eq(parts.catalog.as_ref().unwrap(), &current));
        assert_eq!(parts.candidate.original_json(), original);
        assert_eq!(parts.arts.len(), 1);
        assert_eq!(parts.vectors.len(), 1);
        let prepared = parts.prepare_states(&cancel).unwrap();
        assert_eq!(prepared.state_count(), 1);
        assert_eq!(prepared.candidate.original_json(), original);
        let state = &prepared.states[0];
        assert_eq!(state.unit_id.as_str(), "u");
        assert_eq!(state.execution_results.borrow().unit(), &state.unit_id);
        assert!(state.execution_results.borrow().active().is_none());
        assert!(state.tool_panel.get().is_invalid());
        let selected = state.selected_art.borrow();
        let selected = selected.as_ref().unwrap();
        assert!(Rc::ptr_eq(&selected.catalog, &current));
        assert!(selected.parameters.get("value").is_none());
    }
    #[test]
    fn mixed_host_states_keep_document_order_and_reject_misaligned_resources() {
        let cancel = AtomicBool::new(false);
        // Nonlexicographic order, with an ordinary pin between two Art nodes.
        let bytes = br#"{"stickers":[
            {"id":"z","type":"art","x":0,"y":0,"w":2,"h":2,"outputs":{"value":42}},
            {"id":"pin","x":0,"y":0,"w":2,"h":2},
            {"id":"a","type":"art","x":0,"y":0,"w":2,"h":2,"outputs":{"value":17}}
        ],"links":[]}"#;
        let parts = || {
            let restore = PreparedRestore::prepare(bytes.to_vec(), None, None, &cancel).unwrap();
            Bound::from_prepared(restore, None)
                .into_parts_for(None, &cancel)
                .ok()
                .unwrap()
        };
        let prepared = parts().prepare_states(&cancel).unwrap();
        assert_eq!(
            prepared
                .states
                .iter()
                .map(|state| state.unit_id.as_str())
                .collect::<Vec<_>>(),
            ["z", "pin", "a"]
        );
        for (state, value) in prepared.states.iter().zip([Some(42), None, Some(17)]) {
            let results = state.execution_results.borrow();
            assert_eq!(results.unit(), &state.unit_id);
            assert_eq!(
                results
                    .save_value_outputs()
                    .map(|outputs| outputs["value"].as_i64().unwrap()),
                value
            );
        }
        let mut wrong = parts();
        wrong.regions.swap(0, 1);
        assert!(wrong.prepare_states(&cancel).is_err());
        let mut wrong = parts();
        wrong.results.swap(0, 1);
        assert!(wrong.prepare_states(&cancel).is_err());
    }

    #[test]
    fn cancelled_or_incomplete_host_state_never_returns_partial_batch() {
        let current = catalog(false);
        let cancel = AtomicBool::new(false);
        let parts = || {
            let prepared = VectorCandidate::prepare(candidate(), &AtomicBool::new(false)).unwrap();
            Bound::prepare(prepared, current.clone(), &AtomicBool::new(false))
                .ok()
                .unwrap()
                .into_parts(&current, &AtomicBool::new(false))
                .ok()
                .unwrap()
        };
        let cancelled = parts();
        cancel.store(true, Ordering::Release);
        assert_eq!(
            cancelled.prepare_states(&cancel).err().unwrap().code(),
            windows::Win32::Foundation::E_ABORT
        );
        let mut incomplete = parts();
        incomplete.regions.clear();
        assert!(incomplete.prepare_states(&AtomicBool::new(false)).is_err());
        let mut incomplete = parts();
        incomplete.results.clear();
        assert!(incomplete.prepare_states(&AtomicBool::new(false)).is_err());
        let mut extra = parts();
        extra
            .results
            .push(super::super::loom_result_state::Results::new(
                crate::workspace_id::UnitId::parse("unexpected").unwrap(),
            ));
        assert!(extra.prepare_states(&AtomicBool::new(false)).is_err());
    }
}

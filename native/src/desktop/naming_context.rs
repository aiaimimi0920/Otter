//! Match baseline buildUnitFileNamingContext: title/label are capability names, process is artId, without OS queries.
use super::State;
use crate::{
    file_naming::{Context, MAX_CONTEXT_BYTES},
    loom_capability::Capability,
};

#[derive(Default)]
pub(super) enum RestoredArtId {
    #[default]
    Absent,
    Value(String),
    Oversized,
}
impl RestoredArtId {
    pub fn from_id(id: Option<&str>) -> Self {
        match id {
            None => Self::Absent,
            Some(id) if id.len() <= MAX_CONTEXT_BYTES => Self::Value(id.to_owned()),
            Some(_) => Self::Oversized,
        }
    }
}
/// Borrow the selected immutable catalog without cloning parameters, catalogs, or images; callbacks must not retain context across modals.
pub(super) fn with_context<T>(
    state: &State,
    dimensions: (u32, u32),
    render: impl FnOnce(&Context<'_>) -> T,
) -> Option<T> {
    let selected = state.selected_art.borrow();
    let process = if let Some(selected) = selected.as_ref() {
        selected.parameters.art_id()
    } else if let Some(id) = state.local_art_id.as_deref() {
        id
    } else {
        match &state.restored_naming_art {
            RestoredArtId::Absent => "",
            RestoredArtId::Value(id) => id,
            RestoredArtId::Oversized => return None,
        }
    };
    let label = selected
        .as_ref()
        .and_then(|selected| {
            selected
                .catalog
                .handshake
                .arts()
                .iter()
                .filter_map(Capability::new)
                .find(|cap| cap.id == selected.parameters.art_id())
                .map(|cap| cap.label)
        })
        .unwrap_or("");
    let kind = if state.is_art() { "art" } else { "image" };
    let canvas = state.canvas.borrow();
    let resolved = state.resolved_display.borrow();
    let source = match resolved.as_ref() {
        Some(frame) => frame.naming.as_ref(),
        None => canvas.source_naming(),
    };
    Some(render(&Context {
        app: "Otter",
        kind,
        label: if label.trim().is_empty() {
            kind
        } else {
            label.trim()
        },
        title: label.trim(),
        process,
        source_app: source.map_or("", |s| s.app.as_str()),
        window_title: source.map_or("", |s| s.title.as_str()),
        unit_id: state.unit_id.as_str(),
        width: Some(dimensions.0),
        height: Some(dimensions.1),
        ..Default::default()
    }))
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "naming_context_tests.rs"]
mod tests;

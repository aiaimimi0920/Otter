//! Build effective parameter views in UI dispatch segments when versions change; reuse execution-graph resolution without overwriting manual values.
use super::Application;
use crate::{desktop::parameter_view::View, loom_capability::Capability};
use serde_json::Map;
impl Application {
    pub(super) fn sync_parameter_views(&self) {
        for doc in &self.documents {
            let selected = doc.state.selected_art.borrow();
            let Some(selected) = selected.as_ref() else {
                doc.state.parameter_view.borrow_mut().take();
                continue;
            };
            let Some(capability) = selected
                .catalog
                .handshake
                .arts()
                .iter()
                .filter_map(Capability::new)
                .find(|capability| capability.id == selected.parameters.art_id())
            else {
                doc.state.parameter_view.borrow_mut().take();
                continue;
            };
            let linked: Vec<String> =
                capability
                    .parameters()
                    .filter(|p| {
                        !p.secret
                            && self.workspace.links().iter().any(|link| {
                                link.to_unit == doc.state.unit_id && link.to_port == p.id
                            })
                    })
                    .map(|p| p.id.into())
                    .collect();
            // Menus directly borrow ordinary manual parameters; in particular, do not retain another full copy of file Data URLs.
            if linked.is_empty() {
                View::update(&doc.state, Map::new(), linked);
                continue;
            }
            let resolved = Self::with_parameters(
                &self.documents,
                &self.workspace,
                &doc.state.unit_id,
                &capability,
                &selected.parameters,
                |resolved| {
                    crate::desktop::parameter_view_values::capture(
                        resolved
                            .entries()
                            .filter(|(name, _)| linked.iter().any(|id| id == name)),
                    )
                },
            );
            // Keep link locking when a source is temporarily unavailable; resolution failure must not grant manual-edit admission.
            View::update(
                &doc.state,
                resolved.map(|(values, _)| values).unwrap_or_default(),
                linked,
            );
        }
    }
}

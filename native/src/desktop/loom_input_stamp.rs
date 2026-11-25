//! Independent input-provenance evidence; editing-document revisions alone cannot establish official-output freshness.
use super::{loom_result_state::OutputStamp, State};
use crate::editor::{DocumentStamp, Editor};
#[derive(Clone)]
pub(crate) enum SourceStamp {
    // Allow streaming after capturing a fixed frame; stopping/replacing owners invalidates it, without per-frame sequence changes cancelling long executions.
    Live(std::sync::Arc<()>),
    Document(DocumentStamp),
    Output(OutputStamp),
    Node {
        document: DocumentStamp,
        configuration: super::configuration_stamp::Stamp,
    },
}
impl SourceStamp {
    /// Compatibility for callers supplying only Editor; always reject official outputs rather than bypass result-owner validation.
    pub(crate) fn matches(&self, editor: &Editor) -> bool {
        matches!(self, Self::Document(stamp) if stamp.matches(editor))
    }
    pub(crate) fn matches_state(&self, state: &State) -> bool {
        match self {
            Self::Live(_) => false,
            Self::Document(stamp) => {
                let canvas = state.canvas.borrow();
                stamp.matches(&canvas.editor) && canvas.editor.draft().is_none()
            }
            Self::Output(stamp) => stamp.matches(&state.execution_results.borrow()),
            Self::Node {
                document,
                configuration,
            } => {
                let canvas = state.canvas.borrow();
                document.matches(&canvas.editor)
                    && canvas.editor.draft().is_none()
                    && configuration.matches(state)
            }
        }
    }
    pub(super) fn matches_owners(
        &self,
        unit: &crate::workspace_id::UnitId,
        documents: &[super::document_window::DocumentWindow],
        sessions: &[super::composition::live::session::Session],
        workspace: &crate::workspace::Workspace,
    ) -> bool {
        match self {
            Self::Live(identity) => sessions.iter().any(|session| {
                session.unit_id() == unit
                    && session.is_alive()
                    && session.belongs_to(workspace)
                    && std::sync::Arc::ptr_eq(identity, &session.input_identity())
            }),
            _ => documents
                .iter()
                .find(|doc| doc.state.unit_id == *unit)
                .is_some_and(|doc| self.matches_state(&doc.state)),
        }
    }
}

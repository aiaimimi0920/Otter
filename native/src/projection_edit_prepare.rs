//! Mode/automatic maintenance only prepares original v1 requests; no network owners, and metadata operations do not acknowledge local vector changes.
use super::*;
use protocol::Operation;

fn reclaim(snapshot: &Snapshot) -> bool {
    snapshot.role() == Role::Source
        && (snapshot.document().receipt_count() >= 224
            || (snapshot.document().objects().len() >= 240
                && snapshot
                    .document()
                    .objects()
                    .values()
                    .any(|object| object.value().is_null())))
}
fn automatic_allowed(snapshot: &Snapshot) -> bool {
    snapshot.suspended().is_none()
        && snapshot.conflict().is_none()
        && (snapshot.role() == Role::Source || snapshot.document().mode() == Mode::TwoWay)
}
pub(crate) fn automatic(snapshot: &Snapshot) -> bool {
    automatic_allowed(snapshot)
        && (reclaim(snapshot)
            || snapshot.editor_view().is_ok_and(|current| {
                View::from_document(snapshot.document(), current.size())
                    .and_then(|observed| observed.changes_to(&current))
                    .is_ok_and(|changes| !changes.is_empty())
            }))
}
pub(super) fn request(
    snapshot: &Snapshot,
    op_id: String,
    action: Action,
) -> Result<(Request, Objects), Failure> {
    let fail = || Failure::Protocol(protocol::Error::Response);
    let operation = match action {
        Action::Mode(mode) if snapshot.role() == Role::Source && snapshot.suspended().is_none() => {
            Operation::Mode {
                projection_id: snapshot.projection_id().into(),
                session_id: snapshot.document().session_id().into(),
                op_id,
                base_mode_revision: snapshot.document().mode_revision(),
                mode,
            }
        }
        Action::Automatic if automatic_allowed(snapshot) => {
            if !reclaim(snapshot) {
                return super::prepare(snapshot, op_id);
            }
            Operation::Checkpoint {
                projection_id: snapshot.projection_id().into(),
                session_id: snapshot.document().session_id().into(),
                expected_revision: snapshot.document().revision(),
            }
        }
        Action::Submit => return super::prepare(snapshot, op_id),
        _ => return Err(fail()),
    };
    // mode/checkpoint observe only remote baselines; using current would wrongly acknowledge unsent local changes.
    let observed = snapshot.document().live_objects();
    Request::prepare(operation)
        .map(|request| (request, observed))
        .map_err(Failure::Protocol)
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "projection_edit_prepare_tests.rs"]
mod tests;

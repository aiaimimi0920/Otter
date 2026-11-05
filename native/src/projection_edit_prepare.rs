//! 模式/自动维护只准备原v1请求；不创建网络owner，metadata操作不确认本地矢量修改。
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
pub(crate) fn automatic(snapshot: &Snapshot) -> bool {
    if snapshot.suspended().is_some() || snapshot.conflict().is_some() {
        return false;
    }
    if reclaim(snapshot) {
        return true;
    }
    (snapshot.role() == Role::Source || snapshot.document().mode() == Mode::TwoWay)
        && snapshot.editor_view().is_ok_and(|current| {
            View::from_document(snapshot.document(), current.size())
                .and_then(|observed| observed.changes_to(&current))
                .is_ok_and(|changes| !changes.is_empty())
        })
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
        Action::Automatic if automatic(snapshot) && reclaim(snapshot) => Operation::Checkpoint {
            projection_id: snapshot.projection_id().into(),
            session_id: snapshot.document().session_id().into(),
            expected_revision: snapshot.document().revision(),
        },
        Action::Automatic if automatic(snapshot) => return super::prepare(snapshot, op_id),
        Action::Submit => return super::prepare(snapshot, op_id),
        _ => return Err(fail()),
    };
    // mode/checkpoint只观察远端基线；使用current会把尚未发送的本地修改误当已确认。
    let observed = snapshot.document().live_objects();
    Request::prepare(operation)
        .map(|request| (request, observed))
        .map_err(Failure::Protocol)
}
#[cfg(test)]
#[path = "projection_edit_prepare_tests.rs"]
mod tests;

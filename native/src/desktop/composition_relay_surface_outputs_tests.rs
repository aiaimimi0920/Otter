//! 输出访问不消费收据，输入资格取决于节点确已应用，而不是原始 Commit 被删除。
use super::*;
fn presented() -> crate::desktop::surface_authorized_window::EventReceipt {
    let mut receipt = receipt(completed());
    let event = &receipt.event;
    let ack = receipt.result.as_ref().unwrap();
    let raw = serde_json::value::RawValue::from_string(
        serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":event.binding().instance(),
            "generation":event.binding().generation(),"requestId":ack.request_id(),
            "resultRevision":7,"outputs":{"output":{"kind":"value","value":false}}
        })
        .to_string(),
    )
    .unwrap();
    receipt.commit =
        crate::surface_result_commit::Commit::parse_matching(&raw, event, ack).unwrap();
    receipt.presentation = Ok(Some(3));
    receipt
}
#[test]
fn output_visit_preserves_commit_and_only_applied_version_unblocks_input() {
    let mut sessions = Sessions::default();
    let receipt = presented();
    let binding = receipt.event.binding().clone();
    sessions.event_receipts.push(receipt);
    assert!(!sessions.surface_input_ready(&binding));
    let mut visited = 0;
    sessions.visit_surface_outputs(|actual, commit, revision| {
        assert_eq!(actual, &binding);
        assert_eq!(revision, 3);
        assert_eq!(commit.revision(), 7);
        visited += 1;
    });
    assert_eq!(visited, 1);
    assert!(sessions.surface_input_ready_with(&binding, |commit| commit.revision() == 7));
    assert!(sessions.event_receipts[0].commit.is_some());
    assert!(!sessions.surface_input_ready_with(&binding, |_| false));
}
#[test]
fn revoked_or_unpresented_results_are_not_offered_for_application() {
    let mut sessions = Sessions::default();
    let mut revoked = presented();
    revoked.delivery = Err(crate::device_authorization_lease::Error::Revoked);
    let mut hidden = presented();
    hidden.presentation = Ok(None);
    sessions.event_receipts.extend([revoked, hidden]);
    sessions.visit_surface_outputs(|_, _, _| panic!("not authorized"));
    assert!(sessions
        .event_receipts
        .iter()
        .all(|receipt| receipt.commit.is_some()));
}

//! 真实本地 TLS：批次顺序、确认停止、失败不重放；不连接真实服务。
use super::*;
use crate::live_relay_http::{BatchError, BatchStop};
#[path = "surface_event_batch_runtime_tests.rs"]
mod runtime;
#[test]
fn exhausted_confirmation_budget_returns_ack_without_network_read() {
    let (_listener, client, _config) = fixture();
    let owner = active(&client.origin);
    let mut event = event(&owner);
    event.begin_submission();
    let ack = crate::surface_action_ack::Ack::parse(&serde_json::to_vec(&serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":owner.binding().instance(),"eventId":event.id(),
        "requestId":"request:budget","accepted":true,"status":"awaiting_confirmation"
    })).unwrap(), &event).unwrap();
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let outcome = control.surface_confirmation_until(&event, &ack, std::time::Instant::now());
    assert!(matches!(
        outcome.result,
        Err(crate::live_relay_http::ConfirmationError::Deadline)
    ));
    assert_eq!(outcome.ack.request_id(), "request:budget");
}
#[test]
fn batch_keeps_order_and_stops_at_confirmation_without_attempting_tail() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let mut events = vec![event(&owner), event(&owner), event(&owner)];
    let ids: Vec<_> = events.iter().map(|event| event.id().to_owned()).collect();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        for (index, status) in ["accepted", "awaiting_confirmation"]
            .into_iter()
            .enumerate()
        {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (_, body) = request(&mut stream);
            assert_eq!(body["eventId"], ids[index]);
            reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
                "protocolVersion":"loom.surface.v1","instanceId":body["instanceId"],
                "eventId":body["eventId"],"requestId":format!("request:{index}"),"accepted":true,"status":status
            })).unwrap());
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let result = control
        .submit_surface_click_batch(&owner, 3, &mut events)
        .unwrap();
    assert_eq!(result.stop, BatchStop::Confirmation);
    assert_eq!(result.acknowledgements.len(), 2);
    assert!(events[0].attempted() && events[1].attempted());
    assert!(!events[2].attempted());
    assert!(matches!(
        control.submit_surface_click_batch(&owner, 3, &mut events),
        Err(BatchError::Attempted)
    ));
    peer.join().unwrap();
}
#[test]
fn mixed_batch_is_rejected_before_any_network_attempt() {
    let (_listener, client, _config) = fixture();
    let owner = active(&client.origin);
    let foreign = active("https://foreign.example");
    let mut events = vec![event(&owner), event(&foreign)];
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    assert!(matches!(
        control.submit_surface_click_batch(&owner, 3, &mut events),
        Err(BatchError::Identity)
    ));
    assert!(events.iter().all(|event| !event.attempted()));
}

//! 交付失败、取消和释放重试均保留租约；控制回调注入，不访问真实服务。
use super::*;
use serde_json::json;
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn request() -> Request {
    Request {
        id: id("execution"),
        generation: 4,
    }
}
fn missing_mapping() -> Value {
    json!({"output_image":{"kind":"shared_memory","handle":format!("Loom_Buffer_missing_test_{}",std::process::id()),"format":"rgba8","width":1,"height":1,"size":4}})
}
#[test]
fn decode_failure_preserves_lease_and_transport_failure_allows_retry() {
    let mut delivery = Delivery::prepare(
        id("unit"),
        request(),
        &missing_mapping(),
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        delivery.result().err(),
        Some(&loom_output_batch::Error::Decode)
    );
    assert_eq!(delivery.pending_release_count(), 1);
    assert!(matches!(
        delivery.take_released(),
        Err(TakeError::PendingRelease)
    ));
    assert_eq!(delivery.pending_release_count(), 1);
    assert_eq!(
        delivery.release_with(id("release:first"), |_, _| Err(
            loom_art_control::Error::Closed
        )),
        Err(ReleaseError::Transport(loom_art_control::Error::Closed))
    );
    assert_eq!(delivery.pending_release_count(), 1);
    delivery
        .release_with(id("release:second"), |wire, release_id| {
            let value: Value = serde_json::from_str(&wire).unwrap();
            assert_eq!(value["params"]["requestId"], release_id.as_str());
            assert_eq!(value["params"]["executionRequestId"], "execution");
            assert_eq!(value["params"]["generation"], 4);
            Ok(Outcome::Succeeded)
        })
        .unwrap();
    assert_eq!(delivery.pending_release_count(), 0);
    assert_eq!(
        delivery.result().err(),
        Some(&loom_output_batch::Error::Decode)
    );
    assert!(matches!(
        delivery.take_released(),
        Err(TakeError::Decode(loom_output_batch::Error::Decode))
    ));
    assert!(matches!(
        delivery.take_released(),
        Err(TakeError::Decode(loom_output_batch::Error::Consumed))
    ));
}
#[test]
fn cancelled_decode_still_records_resources_and_rejected_release_does_not_clear_them() {
    let mut delivery = Delivery::prepare(
        id("unit"),
        request(),
        &missing_mapping(),
        None,
        &AtomicBool::new(true),
    )
    .unwrap();
    assert_eq!(
        delivery.result().err(),
        Some(&loom_output_batch::Error::Cancelled)
    );
    assert_eq!(delivery.pending_release_count(), 1);
    assert_eq!(
        delivery.release_with(id("release:one"), |_, _| Ok(Outcome::Failed)),
        Err(ReleaseError::Rejected(Outcome::Failed))
    );
    assert_eq!(delivery.pending_release_count(), 1);
}
#[test]
fn plain_result_requires_no_control_connection_and_preserves_request_identity() {
    let mut delivery = Delivery::prepare(
        id("unit"),
        request(),
        &json!({"output":{"kind":"value","value":42}}),
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(delivery.unit(), &id("unit"));
    assert_eq!(delivery.request(), &request());
    assert!(delivery.result().is_ok());
    delivery
        .release_with(id("release:unused"), |_, _| {
            panic!("no leases must not connect")
        })
        .unwrap();
}

#[test]
fn taking_released_ports_moves_values_once_without_copying_allocations() {
    let mut delivery = Delivery::prepare(
        id("unit"),
        request(),
        &json!({"output":{"kind":"value","value":"preserved allocation"},
            "other":{"kind":"value","value":null}}),
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    let batch = delivery.result().unwrap();
    let super::super::loom_output_batch::Output::Value(value) = &batch.primary().value else {
        panic!("value expected");
    };
    let pointer = value.as_str().unwrap().as_ptr();
    let batch = delivery.take_released().unwrap();
    let (ports, primary) = batch.into_ports();
    assert_eq!(ports[primary].name, "output");
    let super::super::loom_output_batch::Output::Value(value) = &ports[primary].value else {
        panic!("value expected");
    };
    assert_eq!(value.as_str().unwrap().as_ptr(), pointer);
    assert!(ports.iter().any(|port| port.name == "other"));
    assert!(matches!(
        delivery.take_released(),
        Err(TakeError::Decode(loom_output_batch::Error::Consumed))
    ));
    assert_eq!(
        delivery.result().err(),
        Some(&loom_output_batch::Error::Consumed)
    );
}
#[test]
fn derived_batch_requires_consumed_original_and_no_pending_remote_lease() {
    let make_batch = || {
        loom_output_batch::decode(
            &json!({"output":{"kind":"value","value":42}}),
            None,
            &AtomicBool::new(false),
        )
        .unwrap()
    };
    let mut pending = Delivery::prepare(
        id("unit"),
        request(),
        &missing_mapping(),
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        pending.install_derived(make_batch()),
        Err(TakeError::PendingRelease)
    );
    assert_eq!(pending.pending_release_count(), 1);
    let mut delivery = Delivery::prepare(
        id("unit"),
        request(),
        &json!({"output":{"kind":"value","value":7}}),
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        delivery.install_derived(make_batch()),
        Err(TakeError::Decode(loom_output_batch::Error::Consumed))
    );
    delivery.take_released().unwrap();
    delivery.install_derived(make_batch()).unwrap();
    assert_eq!(delivery.unit(), &id("unit"));
    assert_eq!(delivery.request(), &request());
    assert!(delivery.take_released().is_ok());
    assert!(delivery.take_released().is_err());
}

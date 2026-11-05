use super::*;
use serde_json::json;
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn request() -> Request {
    Request {
        id: id("execution"),
        generation: 7,
    }
}
#[test]
fn lease_snapshot_deduplicates_only_formal_resources_and_preserves_execution_identity() {
    let outputs = json!({"image":{"kind":"shared_memory","handle":"Loom_Buffer_a"},
        "mask":{"kind":"shared_memory","handle":"Loom_Buffer_a"},
        "text":{"kind":"value","value":{"kind":"shared_memory","handle":"Loom_Buffer_not_a_resource"}}});
    let mut leases = Leases::outputs(id("unit"), request(), &outputs).unwrap();
    assert_eq!(leases.pending_count(), 1);
    let wire = leases.begin_release(id("release:one")).unwrap();
    let value: Value = serde_json::from_str(&wire).unwrap();
    assert_eq!(value["params"]["nodeId"], "unit");
    assert_eq!(value["params"]["executionRequestId"], "execution");
    assert_eq!(value["params"]["generation"], 7);
    assert_eq!(value["params"]["handles"], json!(["Loom_Buffer_a"]));
    assert_eq!(leases.begin_release(id("release:two")), Err(Error::Busy));
}
#[test]
fn failed_or_stale_acknowledgement_never_drops_pending_resources() {
    let mut leases = Leases::preview(
        id("unit"),
        request(),
        &json!({"kind":"shared_memory","handle":"Loom_Buffer_a"}),
    )
    .unwrap();
    leases.begin_release(id("release:one")).unwrap();
    assert!(!leases.complete_release(&id("wrong"), Some(Outcome::Succeeded)));
    assert_eq!(leases.pending_count(), 1);
    assert!(leases.complete_release(&id("release:one"), None));
    for (index, outcome) in [
        Outcome::Failed,
        Outcome::Cancelled,
        Outcome::RequestNotFound,
    ]
    .into_iter()
    .enumerate()
    {
        let next = id(&format!("release:retry{index}"));
        leases.begin_release(next.clone()).unwrap();
        assert!(!leases.complete_release(&id("release:one"), Some(Outcome::Succeeded)));
        assert!(leases.complete_release(&next, Some(outcome)));
        assert_eq!(leases.pending_count(), 1);
    }
    leases.begin_release(id("release:final")).unwrap();
    assert!(leases.complete_release(&id("release:final"), Some(Outcome::Succeeded)));
    assert!(leases.is_empty());
    assert_eq!(leases.begin_release(id("release:empty")), Err(Error::Empty));
}
#[test]
fn invalid_resources_fail_and_plain_values_have_no_lease() {
    assert!(Leases::outputs(id("unit"), request(), &json!([])).is_err());
    assert!(matches!(
        Leases::preview(
            id("unit"),
            request(),
            &json!({"kind":"shared_memory","handle":"foreign"})
        ),
        Err(Error::InvalidHandle)
    ));
    assert!(
        Leases::preview(id("unit"), request(), &json!({"kind":"value","value":1}))
            .unwrap()
            .is_empty()
    );
    let map = (0..129)
        .map(|index| {
            (
                index.to_string(),
                json!({"kind":"shared_memory","handle":format!("Loom_Buffer_{index}")}),
            )
        })
        .collect();
    assert!(matches!(
        Leases::outputs(id("unit"), request(), &Value::Object(map)),
        Err(Error::Capacity)
    ));
}

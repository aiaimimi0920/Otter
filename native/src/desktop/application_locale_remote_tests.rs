//! Receiver scope and receipt acknowledgement are independent of transport retries.
use super::*;

fn target(name: &str) -> Target {
    Target {
        origin: format!("https://{name}.example"),
        loom: name.into(),
    }
}

fn receipt(id: &str) -> Receipt {
    Receipt {
        request_id: id.into(),
        status: "applied",
        code: "applied",
        applied_revision: Some(1),
    }
}

#[test]
fn locale_target_switch_and_profile_import_cannot_resume_an_old_receiver() {
    let mut owner = Owner::default();
    owner.enable();
    owner.select(Some(target("a")));
    let first = owner.receiver().unwrap().to_owned();
    owner.generation = Some("old-generation".into());
    owner.receipt = Some(receipt("old"));
    owner.select(Some(target("a")));
    assert_eq!(owner.receiver(), Some(first.as_str()));
    owner.select(Some(target("b")));
    assert!(owner.generation.is_none() && owner.receipt.is_none());
    owner.select(Some(target("a")));
    assert_ne!(owner.receiver(), Some(first.as_str()));
    let second = owner.receiver().unwrap().to_owned();
    owner.reset_profile();
    owner.select(Some(target("a")));
    assert_ne!(owner.receiver(), Some(second.as_str()));
    owner.shutdown();
    assert!(!owner.enabled);
    assert!(owner.deadline().is_none());
}

#[test]
fn locale_acknowledgement_retires_only_the_exact_sent_receipt() {
    let mut owner = Owner::default();
    let old = receipt("old");
    let new = receipt("new");
    owner.receipt = Some(new.clone());
    owner.acknowledge(Some(&old));
    assert_eq!(owner.receipt.as_ref(), Some(&new));
    owner.acknowledge(None);
    assert_eq!(owner.receipt.as_ref(), Some(&new));
    owner.acknowledge(Some(&new));
    assert!(owner.receipt.is_none());
}

#[test]
fn locale_result_completed_during_an_older_report_blocks_the_next_request_until_acknowledged() {
    let mut owner = Owner::default();
    // The in-flight report did not contain the result that has since completed locally.
    let completed = receipt("completed-after-report-start");
    owner.receipt = Some(completed.clone());
    owner.next = Some(Instant::now() + POLL_INTERVAL);
    owner.acknowledge(None);
    assert!(!owner.can_accept_request());
    assert!(owner.next.unwrap() <= Instant::now());
    assert_eq!(owner.receipt.as_ref(), Some(&completed));
    owner.acknowledge(Some(&receipt("older")));
    assert!(!owner.can_accept_request());
    owner.acknowledge(Some(&completed));
    assert!(owner.can_accept_request());
}

//! 主干异步草稿语义的有界版本；请求乱序、重复、旧作用域必须不覆盖新编辑。
use super::*;
#[test]
fn surface_input_draft_dispatch_failure_releases_reservation_without_losing_text() {
    let mut draft = Draft::new("source").unwrap();
    draft.edit("unsent input").unwrap();
    let ticket = draft.begin().unwrap();
    assert!(draft.cancel(ticket.clone()));
    assert!(!draft.settle(ticket));
    assert_eq!(draft.pending(), 0);
    assert_eq!(draft.value(), "unsent input");
}
#[test]
fn surface_input_draft_keeps_local_edits_until_all_current_results_settle() {
    let mut draft = Draft::new("initial").unwrap();
    draft.edit("typing").unwrap();
    let first = draft.begin().unwrap();
    draft.edit("typing more").unwrap();
    let second = draft.begin().unwrap();
    draft.synchronize("server result").unwrap();
    assert_eq!(draft.value(), "typing more");
    assert!(draft.settle(second.clone()));
    assert!(!draft.settle(second));
    assert_eq!(draft.pending(), 1);
    assert_eq!(draft.value(), "typing more");
    draft.synchronize("latest result").unwrap();
    assert!(draft.settle(first));
    assert_eq!(draft.value(), "latest result");
}
#[test]
fn surface_input_draft_unchanged_source_does_not_reset_synchronous_local_edits() {
    let mut draft = Draft::new("source").unwrap();
    draft.edit("local").unwrap();
    draft.synchronize("source").unwrap();
    assert_eq!(draft.value(), "local");
    draft.synchronize("changed").unwrap();
    assert_eq!(draft.value(), "changed");
}
#[test]
fn surface_input_draft_old_scope_foreign_and_duplicate_tickets_are_rejected() {
    let mut draft = Draft::new("first").unwrap();
    let old = draft.begin().unwrap();
    let foreign = Draft::new("other").unwrap().begin().unwrap();
    draft.reset("new field").unwrap();
    draft.edit("new input").unwrap();
    let current = draft.begin().unwrap();
    assert!(!draft.settle(old));
    assert!(!draft.settle(foreign));
    assert_eq!(draft.pending(), 1);
    assert_eq!(draft.value(), "new input");
    assert!(draft.settle(current));
    assert_eq!(draft.value(), "new field");
}
#[test]
fn surface_input_draft_rejects_limits_without_partial_changes() {
    let mut draft = Draft::new("valid").unwrap();
    let oversized = "中".repeat(MAX_VALUE_BYTES / 3 + 1);
    assert_eq!(draft.edit(&oversized), Err(Error::Value));
    assert_eq!(draft.synchronize(&oversized), Err(Error::Value));
    assert_eq!(draft.reset(&oversized), Err(Error::Value));
    assert_eq!(draft.value(), "valid");
    let mut tickets = Vec::new();
    for _ in 0..MAX_PENDING {
        tickets.push(draft.begin().unwrap());
    }
    assert!(matches!(draft.begin(), Err(Error::Pending)));
    assert_eq!(draft.pending(), MAX_PENDING);
    assert!(draft.settle(tickets.pop().unwrap()));
    assert!(draft.begin().is_ok());
}

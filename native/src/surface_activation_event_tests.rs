//! 点击和生命周期事务共享唯一 owner；事件身份不能被迟到结果替换。
use super::*;
use crate::{
    surface_activation::Activation,
    surface_click_event::{ClickEvent, Modifiers},
    surface_click_route::ClickRoute,
    surface_scene::Scene,
};
fn event(owner: &Owner) -> ClickEvent {
    let scene = Scene::parse(br#"{"id":"root","type":"view","events":{"click":"run"}}"#).unwrap();
    ClickRoute::new(&scene, Some("root"))
        .next()
        .unwrap()
        .encode(owner.binding(), 3, Modifiers::default())
        .unwrap()
}
#[test]
fn batch_custody_rejects_partial_reordered_and_single_event_returns() {
    let original = owner("active");
    let mut events = vec![event(&original), event(&original)];
    let mut state = Activation::new(original);
    let worker = state.take_batch(&events, 3).unwrap();
    assert!(state.take_event(&events[0]).is_none());
    assert!(state.take_batch(&events, 3).is_none());
    assert!(state.take_inactive().is_none());
    let worker = *state.restore_event(&events[0], worker).err().unwrap();
    let worker = *state.restore_batch(&events[..1], worker).err().unwrap();
    events.swap(0, 1);
    let worker = *state.restore_batch(&events, worker).err().unwrap();
    assert!(state.owner().is_none());
    events.swap(0, 1);
    events[0].begin_submission();
    assert!(state.restore_batch(&events, worker).is_ok());
    assert!(state.owner().unwrap().confirmed_active());
    assert!(state.take_batch(&events, 3).is_none());
    assert!(state.take_inactive().is_some());
}
#[test]
fn invalid_or_empty_batch_never_takes_window_owner() {
    let original = owner("active");
    let events = vec![event(&original)];
    let mut state = Activation::new(original);
    assert!(state.take_batch(&[], 3).is_none());
    assert!(state.take_batch(&events, 4).is_none());
    assert!(state.owner().is_some());
    let worker = state.take_event(&events[0]).unwrap();
    assert!(state.take_batch(&events, 3).is_none());
    let worker = *state.restore_batch(&events, worker).err().unwrap();
    assert!(state.restore_event(&events[0], worker).is_ok());
}
#[test]
fn event_exclusively_holds_owner_and_return_allows_later_deactivation() {
    let original = owner("active");
    let mut event = event(&original);
    let mut state = Activation::new(original);
    let worker = state.take_event(&event).unwrap();
    assert!(state.owner().is_none());
    assert!(state.take().is_none());
    assert!(state.take_inactive().is_none());
    assert!(state.take_event(&event).is_none());
    event.begin_submission();
    assert!(state.restore_event(&event, worker).is_ok());
    assert!(state.owner().unwrap().confirmed_active());
    assert!(state.take_event(&event).is_none());
    assert!(state.take_inactive().is_some());
}
#[test]
fn foreign_event_or_lifecycle_return_cannot_consume_pending_event() {
    let original = owner("active");
    let expected = event(&original);
    let other = event(&original);
    let mut state = Activation::new(original);
    let worker = state.take_event(&expected).unwrap();
    let worker = *state.restore_event(&other, worker).err().unwrap();
    assert!(state.owner().is_none());
    let worker = *state.restore(worker).err().unwrap();
    assert!(state.restore_event(&expected, owner("mounted")).is_err());
    assert!(state.owner().is_none());
    assert!(state.restore_event(&expected, worker).is_ok());
    assert!(state.restore_event(&expected, owner("active")).is_err());
}
#[test]
fn not_active_attempted_and_unmatched_binding_cannot_start_event_transfer() {
    for lifecycle in ["mounted", "inactive", "suspended"] {
        let original = owner(lifecycle);
        let event = event(&original);
        let mut state = Activation::new(original);
        assert!(state.take_event(&event).is_none());
        assert!(state.owner().is_some());
    }
    // mount 合同直接拒绝 disposed；通过真实生命周期 ACK 构造终态而非非法夹具。
    let mut disposed = owner("active");
    let revision = disposed.begin(Lifecycle::Disposed).unwrap();
    disposed
        .acknowledge(
            &ack(Lifecycle::Disposed, revision),
            Lifecycle::Disposed,
            revision,
        )
        .unwrap();
    let disposed_event = event(&disposed);
    assert!(Activation::new(disposed)
        .take_event(&disposed_event)
        .is_none());
    let original = owner("active");
    let mut attempted = event(&original);
    attempted.begin_submission();
    let mismatched = event(&owner("mounted"));
    let mut state = Activation::new(original);
    assert!(state.take_event(&attempted).is_none());
    assert!(state.take_event(&mismatched).is_none());
    assert!(state.owner().unwrap().confirmed_active());
}

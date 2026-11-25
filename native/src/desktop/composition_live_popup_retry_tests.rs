//! Failed-admission caches bind to original display provenance; no same-instance retries or inherited failures across new display/control cycles.
use super::*;
use crate::live_window_target::recovery::popup_layer::fixture::Fixture;

fn leased(fixture: &Fixture) -> Scope {
    Scope::bind(
        &Target::bind(fixture.root.0).unwrap(),
        Some(fixture.owner.identity()),
        fixture.popup.0,
    )
    .unwrap()
}

#[test]
fn a_reopened_display_on_the_same_hwnd_does_not_inherit_the_failed_admission() {
    let fixture = Fixture::new();
    let first = fixture.show();
    let mut family = Family::default();
    let hwnd = fixture.popup.0 .0 as usize;
    // Model production records after start_popup returns Err; this regression starts no capture/input workers.
    family.failed.push(Failure {
        hwnd,
        scope: leased(&fixture),
    });
    assert!(family.failed(hwnd));
    let next = fixture.show();
    assert_ne!(first.instance, next.instance);
    assert!(!first.valid());
    assert!(next.valid());
    // Both scans contain the same HWND; only original provenance reveals the intervening generation change.
    family.retain_failed(&[hwnd]);
    assert!(family.failed.is_empty());
    assert!(!family.failed(hwnd));
}

#[test]
fn a_still_valid_failure_is_suppressed_until_it_leaves_the_discovery_set() {
    let fixture = Fixture::new();
    fixture.show();
    let hwnd = fixture.popup.0 .0 as usize;
    let mut family = Family::default();
    family.failed.push(Failure {
        hwnd,
        scope: leased(&fixture),
    });
    for _ in 0..4 {
        family.retain_failed(&[hwnd]);
        assert_eq!(family.failed.len(), 1);
        assert!(family.failed(hwnd));
    }
    assert!(!family.failed(fixture.root.0 .0 as usize));
    family.retain_failed(&[]);
    assert!(family.failed.is_empty());
    assert!(!family.failed(hwnd));
}

#[test]
fn a_revoked_parent_epoch_cannot_keep_a_new_control_cycle_in_failed_state() {
    let fixture = Fixture::new();
    fixture.show();
    let bounds = rect(-28000, -28000, 80, 60);
    let child = Source::new(Some(fixture.popup.0), bounds, true);
    child.show();
    let parent_b = Source::new(Some(fixture.b.0), bounds, true);
    let owner = crate::live_hover_owner::Owner::new(parent_b.0).unwrap();
    owner.control(true).unwrap();
    let scope = leased(&fixture)
        .bind_descendant(Some(owner.identity()), child.0)
        .unwrap();
    let hwnd = child.0 .0 as usize;
    let mut family = Family::default();
    family.failed.push(Failure { hwnd, scope });
    assert!(family.failed(hwnd));
    owner.control(false).unwrap();
    owner.control(true).unwrap();
    assert!(leased(&fixture)
        .bind_descendant(Some(owner.identity()), child.0)
        .is_ok());
    // New provenance remains valid, but reenabling the same B must not revive failure records for old provenance.
    family.retain_failed(&[hwnd]);
    assert!(family.failed.is_empty());
    assert!(!family.failed(hwnd));
}

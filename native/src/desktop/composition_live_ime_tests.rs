//! IME-positioning coordinate, invalidation, and sampling-budget regressions; no IME startup or messages to other processes.
use super::*;
use crate::live_viewport::Viewport;

fn view(size: [u32; 2]) -> Viewport {
    Viewport::fit([200, 100], size).unwrap()
}
fn observed(point: [f64; 2]) -> Result<Option<Observation>> {
    Ok(Some(Observation {
        point,
        size: [200, 100],
    }))
}

#[test]
fn click_fallback_tracks_committed_resize_crop_and_mirror() {
    let mut anchor = Anchor::default();
    assert_eq!(anchor.point(view([400, 400])), None);
    anchor.click([0.25, 0.5]);
    assert_eq!(anchor.point(view([400, 400])), Some([100, 200]));
    assert_eq!(anchor.point(view([800, 400])), Some([200, 200]));
    let cropped = view([400, 400]).with_region([0.0, 0.25, 0.5, 0.5], [false, false]);
    assert_eq!(anchor.point(cropped), Some([200, 200]));
    let mirrored = view([400, 400]).with_region([0.0, 0.0, 1.0, 1.0], [true, false]);
    assert_eq!(anchor.point(mirrored), Some([300, 200]));
}

#[test]
fn source_caret_works_without_click_and_follows_focus_changes() {
    let mut anchor = Anchor::default();
    let now = Instant::now();
    let revision = anchor.due(now).unwrap();
    assert!(!anchor.observe(revision, &observed([0.25, 0.5])));
    assert_eq!(anchor.point(view([400, 400])), Some([100, 200]));
    let revision = anchor.due(now + Duration::from_millis(200)).unwrap();
    anchor.observe(revision, &observed([0.75, 0.25]));
    assert_eq!(anchor.point(view([400, 400])), Some([300, 150]));
    anchor.observe(revision, &Ok(None));
    assert_eq!(anchor.point(view([400, 400])), None);
}

#[test]
fn known_caret_outside_crop_does_not_use_stale_click() {
    let mut anchor = Anchor::default();
    anchor.click([0.25, 0.5]);
    let cropped = view([400, 400]).with_region([0.0, 0.0, 0.5, 1.0], [false, false]);
    let revision = anchor.due(Instant::now()).unwrap();
    anchor.observe(revision, &observed([0.75, 0.5]));
    assert_eq!(anchor.point(cropped), None);
    anchor.observe(revision, &observed([-0.1, 0.5]));
    assert_eq!(anchor.point(cropped), None);
    anchor.observe(revision, &Ok(None));
    assert_eq!(anchor.point(cropped), Some([200, 200]));
}

#[test]
fn source_resize_requires_matching_committed_texture_before_positioning() {
    let mut anchor = Anchor::default();
    anchor.click([0.25, 0.5]);
    let revision = anchor.due(Instant::now()).unwrap();
    anchor.observe(
        revision,
        &Ok(Some(Observation {
            point: [0.5, 0.5],
            size: [300, 100],
        })),
    );
    assert_eq!(anchor.point(view([400, 400])), None);
    let resized = Viewport::fit([300, 100], [600, 200]).unwrap();
    assert_eq!(anchor.point(resized), Some([300, 100]));
}

#[test]
fn clearing_or_new_click_cannot_publish_an_older_observation() {
    let mut input = Control::default();
    input.ime.click([0.25, 0.5]);
    let revision = input.ime.due(Instant::now()).unwrap();
    input.clear();
    assert!(!input.ime.observe(revision, &observed([0.75, 0.5])));
    assert_eq!(input.ime.point(view([400, 400])), None);
    let revision = input.ime.due(Instant::now()).unwrap();
    input.ime.click([0.5, 0.5]);
    assert!(!input.ime.observe(revision, &observed([0.75, 0.5])));
    assert_eq!(input.ime.point(view([400, 400])), Some([200, 200]));
}

#[test]
fn source_queries_are_throttled_and_error_reporting_is_deduplicated() {
    let mut anchor = Anchor::default();
    anchor.click([0.25, 0.5]);
    anchor.start_composition();
    let now = Instant::now();
    let revision = anchor.due(now).unwrap();
    assert!(anchor.due(now + Duration::from_millis(15)).is_none());
    assert!(anchor.due(now + Duration::from_millis(16)).is_some());
    anchor.observe(revision, &observed([0.75, 0.5]));
    let failure = Err(Error::new(E_FAIL, "injected caret transform failure"));
    assert!(anchor.observe(revision, &failure));
    assert_eq!(anchor.point(view([400, 400])), Some([100, 200]));
    assert!(!anchor.observe(revision, &failure));
    assert!(!anchor.observe(revision, &Ok(None)));
    assert!(anchor.observe(revision, &failure));
}

#[test]
fn composition_start_bypasses_idle_throttle_and_clear_revokes_observation() {
    let mut anchor = Anchor::default();
    let now = Instant::now();
    anchor.due(now).unwrap();
    assert!(anchor.due(now + Duration::from_millis(20)).is_none());
    anchor.start_composition();
    let revision = anchor.due(now + Duration::from_millis(20)).unwrap();
    assert!(anchor.due(now + Duration::from_millis(35)).is_none());
    anchor.observe(revision, &observed([0.75, 0.5]));
    assert_eq!(anchor.point(view([400, 400])), Some([300, 200]));
    anchor.clear();
    anchor.observe(revision, &observed([0.25, 0.5]));
    assert!(!anchor.composing);
    assert!(anchor.caret.is_none());
}

#[test]
fn only_current_active_changed_caret_requests_reposition() {
    let mut anchor = Anchor::default();
    let stale = anchor.due(Instant::now()).unwrap();
    anchor.start_composition();
    assert!(anchor
        .observe_for_position(stale, &observed([0.25, 0.5]))
        .1
        .is_none());
    let revision = anchor.due(Instant::now()).unwrap();
    let (_, position) = anchor.observe_for_position(revision, &observed([0.75, 0.5]));
    assert_eq!(position.unwrap().point(view([400, 400])), Some([300, 200]));
    assert!(anchor
        .observe_for_position(revision, &observed([0.75, 0.5]))
        .1
        .is_none());
    anchor.composing = false;
    assert!(anchor
        .observe_for_position(revision, &observed([0.25, 0.5]))
        .1
        .is_none());
    anchor.clear();
    assert!(anchor
        .observe_for_position(revision, &observed([0.75, 0.5]))
        .1
        .is_none());
}

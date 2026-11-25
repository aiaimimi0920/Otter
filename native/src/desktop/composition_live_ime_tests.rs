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
    let revision = anchor.due(now + Duration::from_millis(16)).unwrap();
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

use super::*;
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}

#[test]
fn idle_observation_preserves_requests_and_retired_generations() {
    let mut registry = Requests::default();
    assert!(registry.is_idle());
    let unit = id("orphan-unit");
    let request = registry.begin(unit.clone(), id("request")).unwrap();
    registry
        .mark_preview(&unit, &request, "preview".into())
        .unwrap();
    assert!(!registry.is_idle());
    assert_eq!(registry.active(&unit), Some(&request));
    assert_eq!(registry.preview(&unit, &request), Some("preview"));
    assert!(registry.finish(&unit, &request));
    assert!(registry.is_idle());
    let next = registry.begin(unit.clone(), id("request")).unwrap();
    assert_eq!(next.generation, request.generation + 1);
    assert!(!registry.is_idle());
    registry.retain_units(|_| false);
    assert!(registry.is_idle());
}
#[test]
fn stale_preview_and_finish_cannot_overwrite_replacement() {
    let mut registry = Requests::default();
    let unit = id("unit");
    let first = registry.begin(unit.clone(), id("request-1")).unwrap();
    assert!(registry
        .mark_preview(&unit, &first, "first".into())
        .unwrap());
    let second = registry.begin(unit.clone(), id("request-2")).unwrap();
    assert_eq!(second.generation, 2);
    assert_eq!(registry.preview_bytes(), 0);
    assert!(registry
        .mark_preview(&unit, &second, "second".into())
        .unwrap());
    assert!(!registry
        .mark_preview(&unit, &first, "stale".into())
        .unwrap());
    assert!(!registry.finish(&unit, &first));
    assert_eq!(registry.preview(&unit, &second), Some("second"));
    assert_eq!(registry.preview(&unit, &first), None);
    assert!(registry.finish(&unit, &second));
    assert!(registry.active(&unit).is_none());
    assert_eq!(registry.preview_bytes(), 0);
}
#[test]
fn reused_request_id_still_requires_generation_and_unit_match() {
    let mut registry = Requests::default();
    let unit = id("unit");
    let first = registry.begin(unit.clone(), id("reused")).unwrap();
    registry.invalidate(&unit);
    let second = registry.begin(unit.clone(), id("reused")).unwrap();
    assert!(!registry.is_latest(&unit, &first));
    assert!(registry.is_latest(&unit, &second));
    assert!(!registry
        .mark_preview(&id("another"), &second, "wrong".into())
        .unwrap());
    assert_eq!(registry.preview_bytes(), 0);
}
#[test]
fn preview_budget_rejection_is_atomic_and_replacement_reclaims_space() {
    let mut registry = Requests::default();
    let unit = id("unit");
    let request = registry.begin(unit.clone(), id("request")).unwrap();
    registry
        .mark_preview(&unit, &request, "old".into())
        .unwrap();
    assert_eq!(
        registry.mark_preview(&unit, &request, "x".repeat(MAX_PREVIEW_BYTES + 1)),
        Err(Error::PreviewTooLarge)
    );
    assert_eq!(registry.preview(&unit, &request), Some("old"));
    assert_eq!(registry.preview_bytes(), 3);
    registry.mark_preview(&unit, &request, "x".into()).unwrap();
    assert_eq!(registry.preview_bytes(), 1);
    registry.invalidate(&unit);
    assert_eq!(registry.preview_bytes(), 0);
}
#[test]
fn total_preview_budget_and_object_cleanup_are_bounded() {
    let mut registry = Requests::default();
    for index in 0..MAX_TOTAL_PREVIEW_BYTES / MAX_PREVIEW_BYTES {
        let unit = id(&index.to_string());
        let request = registry.begin(unit.clone(), id("request")).unwrap();
        registry
            .mark_preview(&unit, &request, "x".repeat(MAX_PREVIEW_BYTES))
            .unwrap();
    }
    let extra = id("extra");
    let request = registry.begin(extra.clone(), id("request")).unwrap();
    assert_eq!(
        registry.mark_preview(&extra, &request, "x".into()),
        Err(Error::PreviewTooLarge)
    );
    registry.retain_units(|unit| *unit != id("0"));
    assert_eq!(
        registry.preview_bytes(),
        MAX_TOTAL_PREVIEW_BYTES - MAX_PREVIEW_BYTES
    );
    assert!(registry.mark_preview(&extra, &request, "x".into()).unwrap());
    registry.clear();
    assert!(registry.active(&extra).is_none());
    assert_eq!(registry.preview_bytes(), 0);
}
#[test]
fn finished_metadata_capacity_requires_object_removal_not_unbounded_growth() {
    let mut registry = Requests::default();
    for index in 0..MAX_UNITS {
        let unit = id(&index.to_string());
        let request = registry.begin(unit.clone(), id("request")).unwrap();
        registry.finish(&unit, &request);
    }
    assert_eq!(
        registry.begin(id("extra"), id("request")),
        Err(Error::Capacity)
    );
    registry.retain_units(|unit| *unit != id("0"));
    assert!(registry.begin(id("extra"), id("request")).is_ok());
}
#[test]
fn generation_exhaustion_preserves_current_request_and_preview() {
    let mut registry = Requests::default();
    let unit = id("unit");
    let request = registry.begin(unit.clone(), id("request")).unwrap();
    registry
        .mark_preview(&unit, &request, "preview".into())
        .unwrap();
    registry.entries[0].generation = u64::MAX;
    assert_eq!(
        registry.begin(unit.clone(), id("overflow")),
        Err(Error::GenerationExhausted)
    );
    assert!(registry.is_latest(&unit, &request));
    assert_eq!(registry.preview(&unit, &request), Some("preview"));
}

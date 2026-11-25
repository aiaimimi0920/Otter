//! Repainting failures retain bounded settlement work without reformatting scenes or replaying extension operations.
use super::*;
use std::cell::RefCell;

#[test]
fn failed_stationary_refresh_retries_at_settlement_without_a_second_language_change() {
    let cache = RefCell::new(Cache::default());
    let failed = redraw_language(&cache, true, || {
        assert!(cache.borrow().language_redraw);
        Err(windows::core::Error::from(E_FAIL))
    });
    assert_eq!(failed.unwrap_err().code(), E_FAIL);
    assert!(cache.borrow().language_redraw);
    assert!(redraw_language(&cache, false, || {
        // The callback may synchronously inspect cache state without borrowing conflicts.
        assert!(cache.borrow().language_redraw);
        Ok(())
    })
    .unwrap());
    assert!(!cache.borrow().language_redraw);
    assert!(
        !redraw_language(&cache, false, || panic!("unchanged views must not repaint")).unwrap()
    );
}

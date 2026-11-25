//! Same-revision local presentation updates deliver only viewId without replacing authoritative scenes, resources, or children.
use super::*;

impl Session {
    /// Upper layers first recheck current catalog/host declarations; this seam grants no Art execution or remote-update permissions.
    pub fn select_view(&mut self, view_id: &str, stopped: &impl Fn() -> bool) -> io::Result<Frame> {
        let view_id = crate::art_surface_views::Id::parse(view_id).ok_or_else(wire::invalid)?;
        self.exchange(
            Command::SelectView { view_id },
            Instant::now() + TIMEOUT,
            stopped,
        )
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_javascript_view_tests.rs"]
mod tests;

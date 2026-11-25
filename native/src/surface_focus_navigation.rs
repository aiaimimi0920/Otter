//! Sequential keyboard navigation reveals targets before delivering visible focus; it creates no actions and changes no submission permissions.
use super::*;
impl Presentation {
    pub(crate) fn can_focus(&self, id: &str) -> bool {
        self.navigation.iter().any(|candidate| candidate == id)
    }
    pub(crate) fn reveal_focus(&mut self, id: &str) -> Result<bool> {
        if !self.can_focus(id) {
            return Ok(false);
        }
        let mut scroll = self.scroll.clone();
        if scroll.reveal(&self.geometry, id) {
            let frame = self
                .cache
                .project_with_images(&self.scene, &mut scroll, &self.images)?;
            let focusable = crate::surface_focus::buttons(&self.scene, &frame.geometry);
            // Unrevealable candidates must not scroll the view first; commit together only after successful, visible preparation.
            if focusable.iter().any(|candidate| candidate == id) {
                self.install_frame_with_focus(frame, scroll, focusable);
                return Ok(true);
            }
            Ok(false)
        } else {
            Ok(self.focusable(id))
        }
    }
    pub(crate) fn reveal_next_focus(
        &mut self,
        current: Option<&str>,
        reverse: bool,
    ) -> Result<Option<String>> {
        // Navigation includes offscreen controls; action eligibility still includes only currently visible controls.
        let mut candidate = current.map(str::to_owned);
        for _ in 0..self.navigation.len() {
            let Some(next) =
                crate::surface_focus::next(&self.navigation, candidate.as_deref(), reverse)
                    .map(str::to_owned)
            else {
                break;
            };
            if self.reveal_focus(&next)? {
                return Ok(Some(next));
            }
            // Unrevealable targets, such as those with static zero-area ancestors, must not block the entire Tab cycle.
            candidate = Some(next);
        }
        Ok(None)
    }
}

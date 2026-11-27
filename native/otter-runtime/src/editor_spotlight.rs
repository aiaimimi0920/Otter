//! Document-wide mask density; individual holes only carry geometry.
use super::{EditError, Editor, Tool};

impl Tool {
    pub fn spotlight(self) -> bool {
        matches!(self, Self::SpotRect | Self::SpotRounded | Self::SpotEllipse)
    }
}
impl Editor {
    pub fn has_spotlight(&self) -> bool {
        self.elements
            .iter()
            .chain(self.draft.iter())
            .any(|s| s.tool.spotlight())
    }
    pub fn spotlight_percent(&self) -> u8 {
        self.spotlight_percent.unwrap_or(60)
    }
    pub fn set_spotlight_percent(&mut self, value: u8) -> Result<(), EditError> {
        if !(20..=90).contains(&value) {
            return Err(EditError::InvalidPoint);
        }
        if self.spotlight_percent() != value {
            if self.draft.is_some() {
                return Err(EditError::InvalidPoint);
            }
            self.retained_points -= self.history.record(crate::edit_history::Entry::spotlight(
                self.spotlight_percent(),
            ));
            self.spotlight_percent = Some(value);
            self.revision = self.revision.wrapping_add(1);
        }
        Ok(())
    }
    pub(crate) fn restore_spotlight_percent(&mut self, value: u8) -> Result<(), EditError> {
        if self.revision != 0 || !(20..=90).contains(&value) {
            return Err(EditError::InvalidPoint);
        }
        self.spotlight_percent = Some(value);
        Ok(())
    }
}

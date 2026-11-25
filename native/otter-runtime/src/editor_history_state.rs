//! Toolbars read actual bounded-history availability, never inferring undo/redo from clicks or annotation counts.
impl super::Editor {
    pub fn history_state(&self) -> (bool, bool) {
        if self.draft.is_some() {
            (false, false)
        } else {
            self.history.availability()
        }
    }
}

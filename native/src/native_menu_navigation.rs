//! Bounded single-slot delivery for local menu navigation; presentation updates such as paging only, without business execution in modal callbacks.
use super::*;
#[derive(Default)]
pub(super) struct State {
    pub commands: Vec<usize>,
    pub pending: Option<usize>,
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "native_menu_navigation_tests.rs"]
mod tests;
impl Cascade {
    /// Register on the first dynamic callback, then retain the same mapping; at most eight nonzero unique navigation commands.
    pub fn navigation_commands(&mut self, commands: &[usize]) -> Result<()> {
        if self.closed
            || commands.len() > 8
            || commands.contains(&0)
            || commands
                .iter()
                .enumerate()
                .any(|(index, id)| commands[..index].contains(id))
            || (!self.navigation.commands.is_empty() && self.navigation.commands != commands)
        {
            return Err(Error::new(E_INVALIDARG, "Invalid menu navigation mapping"));
        }
        if self.navigation.commands.is_empty() {
            self.navigation.commands.extend_from_slice(commands);
        }
        Ok(())
    }
    /// Return only local navigation, never final business commands; callers must replace corresponding submenus within this update.
    pub fn take_navigation(&mut self) -> Option<usize> {
        self.navigation.pending.take()
    }
}

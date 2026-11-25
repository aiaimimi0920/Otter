//! Explicit pin-operation priority lasts only until foreground ownership changes; background object events cannot repeatedly seize z-order.
#[derive(Default)]
pub struct PriorityPolicy {
    foreground: isize,
    claimed: bool,
}

impl PriorityPolicy {
    pub fn resolve(
        &mut self,
        foreground: isize,
        blocker: Option<isize>,
        claim: bool,
    ) -> Option<isize> {
        if foreground != self.foreground {
            self.foreground = foreground;
            self.claimed = false;
        }
        self.claimed |= claim;
        if self.claimed {
            None
        } else {
            blocker
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_interaction_survives_noise_but_not_foreground_change() {
        let mut policy = PriorityPolicy::default();
        assert_eq!(policy.resolve(10, Some(10), false), Some(10));
        assert_eq!(policy.resolve(10, Some(10), true), None);
        assert_eq!(policy.resolve(10, Some(10), false), None);
        assert_eq!(policy.resolve(20, None, false), None);
        assert_eq!(policy.resolve(10, Some(10), false), Some(10));
    }
    #[test]
    fn closing_blocker_does_not_preserve_a_stale_anchor() {
        let mut policy = PriorityPolicy::default();
        assert_eq!(policy.resolve(10, Some(10), false), Some(10));
        assert_eq!(policy.resolve(10, None, false), None);
    }
}

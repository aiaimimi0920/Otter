//! 端点条精确键位；不读取全局前台或注入键盘输入。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    EnterRail(bool),
    Move(bool),
    Edge(bool),
    Disclose,
    Edit,
    Link,
    Escape,
}
pub(super) fn action(
    key: usize,
    control: bool,
    shift: bool,
    alt: bool,
    win: bool,
) -> Option<Action> {
    if alt || win {
        return None;
    }
    if control {
        return (key == 0x0d && !shift).then_some(Action::Link);
    }
    match key {
        0x75 => Some(Action::EnterRail(shift)),
        0x09 => Some(Action::Move(shift)),
        0x25 | 0x26 if !shift => Some(Action::Move(true)),
        0x27 | 0x28 if !shift => Some(Action::Move(false)),
        0x24 | 0x23 if !shift => Some(Action::Edge(key == 0x23)),
        0x0d | 0x20 if !shift => Some(Action::Disclose),
        0x71 if !shift => Some(Action::Edit),
        0x1b if !shift => Some(Action::Escape),
        _ => None,
    }
}
pub(super) fn key_bit(key: usize) -> u32 {
    [
        0x75, 0x09, 0x25, 0x26, 0x27, 0x28, 0x24, 0x23, 0x0d, 0x20, 0x1b, 0x71,
    ]
    .iter()
    .position(|value| *value == key)
    .map_or(0, |index| 1 << index)
}
pub(super) fn position(current: usize, len: usize, backward: bool) -> Option<usize> {
    if current >= len || len == 0 {
        return None;
    }
    Some(if backward {
        (current + len - 1) % len
    } else {
        (current + 1) % len
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_chords_and_keyup_bits_do_not_alias_modifiers_or_unhandled_keys() {
        assert_eq!(
            action(0x75, false, true, false, false),
            Some(Action::EnterRail(true))
        );
        assert_eq!(action(0x0d, true, false, false, false), Some(Action::Link));
        for key in [0x75, 0x09, 0x25, 0x0d, 0x20, 0x1b] {
            assert_ne!(key_bit(key), 0);
            assert_eq!(action(key, false, false, true, false), None);
            assert_eq!(action(key, false, false, false, true), None);
        }
        assert_eq!(key_bit(0x41), 0);
        assert_eq!(action(0x0d, true, true, false, false), None);
        assert_eq!(action(0x41, false, false, false, false), None);
    }
    #[test]
    fn navigation_wraps_empty_and_stale_positions_are_rejected() {
        assert_eq!(position(0, 4, true), Some(3));
        assert_eq!(position(3, 4, false), Some(0));
        assert_eq!(position(0, 1, false), Some(0));
        assert_eq!(position(0, 0, true), None);
        assert_eq!(position(4, 4, false), None);
    }
}

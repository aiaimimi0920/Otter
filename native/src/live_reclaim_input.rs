//! 无 OS 调用的物理 shell 手势分类；只保存修饰键位，不保存文本或输入历史。
use crate::live_reclaim::{Intent, IntentTracker};

#[derive(Default)]
pub(super) struct Input {
    modifiers: u8,
    tracker: IntentTracker,
}
impl Input {
    pub fn keyboard(&mut self, key: u32, down: bool, injected: bool, now: u64, origin: usize) {
        if injected {
            return;
        }
        let modifier = match key {
            0x12 => 1,
            0xA4 => 2,
            0xA5 => 4,
            0x5B => 8,
            0x5C => 16,
            _ => 0,
        };
        if down {
            self.modifiers |= modifier;
        } else {
            self.modifiers &= !modifier;
        }
        if !down {
            return;
        }
        let reclaim = (self.modifiers & 7 != 0 && matches!(key, 0x09 | 0x1B))
            || (self.modifiers & 24 != 0 && matches!(key, 0x09 | 0x30..=0x39));
        if reclaim {
            self.tracker.record(now, origin);
        } else {
            self.tracker.cancel();
        }
    }
    pub fn mouse_down(&mut self, shell: bool, injected: bool, now: u64, origin: usize) {
        if injected {
            return;
        }
        if shell {
            self.tracker.record(now, origin);
        } else {
            self.tracker.cancel();
        }
    }
    pub fn snapshot(&self, now: u64) -> Option<Intent> {
        self.tracker.snapshot(now)
    }
}
pub(super) fn shell_class(class: &[u16]) -> bool {
    [
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
        "TaskListThumbnailWnd",
        "MultitaskingViewFrame",
        "XamlExplorerHostIslandWindow",
    ]
    .iter()
    .any(|name| class.iter().copied().eq(name.encode_utf16()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_physical_shell_chords_mint_intent_and_release_preserves_it() {
        let mut input = Input::default();
        input.keyboard(0xA4, true, true, 1, 10);
        input.keyboard(0x09, true, false, 2, 10);
        assert!(input.snapshot(2).is_none());
        input.keyboard(0xA4, true, false, 3, 10);
        input.keyboard(0x09, true, true, 4, 10);
        assert!(input.snapshot(4).is_none());
        input.keyboard(0x09, true, false, 5, 10);
        input.keyboard(0xA4, false, false, 6, 10);
        assert!(input.snapshot(6).is_some());
        input.keyboard(0x41, true, false, 7, 10);
        assert!(input.snapshot(7).is_none());
        input.keyboard(0x5B, true, false, 8, 10);
        input.keyboard(0x31, true, false, 9, 10);
        assert!(input.snapshot(9).is_some());
        assert!(input.snapshot(2010).is_none());
    }
    #[test]
    fn shell_clicks_are_exact_and_injected_clicks_cannot_mint_or_cancel() {
        let mut input = Input::default();
        assert!(shell_class(
            &"Shell_TrayWnd".encode_utf16().collect::<Vec<_>>()
        ));
        assert!(!shell_class(
            &"Shell_TrayWnd.fake".encode_utf16().collect::<Vec<_>>()
        ));
        input.mouse_down(true, true, 1, 10);
        assert!(input.snapshot(1).is_none());
        input.mouse_down(true, false, 2, 10);
        input.mouse_down(false, true, 3, 10);
        assert!(input.snapshot(3).is_some());
        input.mouse_down(false, false, 4, 10);
        assert!(input.snapshot(4).is_none());
    }
}

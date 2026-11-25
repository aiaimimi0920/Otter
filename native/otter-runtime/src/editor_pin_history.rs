//! 贴图呈现操作使用标注的同一历史游标；只传纯数据，不从编辑模型调用 HWND。
use super::*;
use crate::pin_geometry::PinRect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PinValue {
    Frame(PinRect),
    Opacity { mini: bool, value: f32 },
    Style(crate::pin_frame_style::Style),
}
#[derive(Clone, Copy)]
pub struct PinChange {
    before: PinValue,
    after: PinValue,
    applied: bool,
}
impl PinChange {
    pub fn toggle(&mut self) -> PinValue {
        self.applied = !self.applied;
        if self.applied {
            self.after
        } else {
            self.before
        }
    }
}
impl Editor {
    /// undo/redo 已清空旧交付；本次 pin 项只推进呈现计数，内容项仍推进内容凭证。
    pub(super) fn commit_history_step(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.presentation_revision = self
            .presentation_revision
            .wrapping_add(u64::from(self.pending_pin.is_some()));
    }
    /// desktop 在实际呈现成功之后记历史；相同值不消耗历史，也不清重做分支。
    pub fn record_pin_change(&mut self, before: PinValue, after: PinValue) -> bool {
        if self.draft.is_some() || before == after {
            return false;
        }
        self.retained_points -= self.history.record(Entry::pin(PinChange {
            before,
            after,
            applied: true,
        }));
        self.revision = self.revision.wrapping_add(1);
        self.presentation_revision = self.presentation_revision.wrapping_add(1);
        true
    }
    /// 每次 undo/redo 清空旧交付；消息层必须在下一次历史动作之前同步消费。
    pub fn take_pin_change(&mut self) -> Option<PinValue> {
        self.pending_pin.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn opacity(value: f32) -> PinValue {
        PinValue::Opacity { mini: false, value }
    }
    #[test]
    fn pin_and_annotation_changes_share_one_order_and_noop_keeps_redo() {
        let mut e = Editor::default();
        assert!(e.record_pin_change(opacity(1.0), opacity(0.5)));
        e.begin(Tool::Brush, Point { x: 1.0, y: 1.0 }).unwrap();
        assert!(e.commit());
        assert!(e.undo());
        assert!(e.elements().is_empty());
        assert_eq!(e.take_pin_change(), None);
        assert!(e.undo());
        assert_eq!(e.take_pin_change(), Some(opacity(1.0)));
        assert!(!e.record_pin_change(opacity(1.0), opacity(1.0)));
        assert!(e.redo());
        assert_eq!(e.take_pin_change(), Some(opacity(0.5)));
        assert!(e.redo());
        assert_eq!(e.elements().len(), 1);
        assert_eq!(e.take_pin_change(), None);
    }
    #[test]
    fn frame_effect_is_one_shot_and_new_edit_drops_redo_in_same_bounded_history() {
        let mut e = Editor::default();
        let frame = PinRect {
            x: -200,
            y: 50,
            width: 200,
            height: 100,
        };
        let after = PinRect {
            width: 400,
            height: 200,
            ..frame
        };
        assert!(e.record_pin_change(PinValue::Frame(frame), PinValue::Frame(after)));
        assert!(e.undo());
        assert_eq!(e.take_pin_change(), Some(PinValue::Frame(frame)));
        assert_eq!(e.take_pin_change(), None);
        assert!(e.record_pin_change(opacity(1.0), opacity(0.3)));
        assert!(!e.redo());
        assert_eq!(e.take_pin_change(), None);
    }
    #[test]
    fn content_stamp_ignores_pin_history_but_not_annotation_edits_or_document_replacement() {
        let mut e = Editor::default();
        let stamp = e.document_stamp();
        assert!(e.record_pin_change(opacity(1.0), opacity(0.5)));
        assert!(!stamp.matches(&e));
        assert!(stamp.matches_content(&e));
        assert!(e.undo());
        assert!(stamp.matches_content(&e));
        assert!(e.redo());
        assert!(stamp.matches_content(&e));
        assert!(!stamp.matches_content(&e.export_snapshot()));
        e.begin(Tool::Brush, Point { x: 1.0, y: 1.0 }).unwrap();
        assert!(e.commit());
        assert!(!stamp.matches_content(&e));
        let edit = e.document_stamp();
        assert!(e.undo());
        assert!(!edit.matches_content(&e));
        let undone = e.document_stamp();
        assert!(e.redo());
        assert!(!undone.matches_content(&e));
    }
}

//! 遮挡按钮补偿的纯手势判定；不查询 HWND、不发送消息，也不授予输入权限。
//! 调用方必须在普通 mouse-up 成功后重新验证目标、按钮类型和遮挡，才能消费候选。
use crate::live_input::Button;

#[derive(Debug)]
pub struct Gesture<T> {
    target: T,
    origin: [i32; 2],
    threshold: [u32; 2],
    cancelled: bool,
}
impl<T: PartialEq> Gesture<T> {
    /// 仅显式左键、无其他 held 按钮、合格且遮挡的 push button 可以开始。
    /// threshold 使用目标 DPI 下系统拖动距离的一半；调用方保存绑定的目标身份。
    pub fn begin(
        target: T,
        origin: [i32; 2],
        threshold: [u32; 2],
        button: Button,
        previously_held: u8,
        eligible_and_occluded: bool,
    ) -> Option<Self> {
        (button == Button::Left && previously_held == 0 && eligible_and_occluded).then_some(Self {
            target,
            origin,
            threshold,
            cancelled: false,
        })
    }

    /// 拖出再返回仍取消；目标变化和坐标极值不能重置或溢出阈值判定。
    pub fn observe(&mut self, target: &T, point: [i32; 2]) {
        self.cancelled |= self.target != *target
            || self.origin[0].abs_diff(point[0]) > self.threshold[0]
            || self.origin[1].abs_diff(point[1]) > self.threshold[1];
    }

    /// 其他按钮边沿、滚轮、取消或失败必须终止候选，不能被后续 up 复活。
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// 消费所有权确保最多一次；cleanup 释放不得调用本入口。
    pub fn finish(
        mut self,
        target: &T,
        point: [i32; 2],
        button: Button,
        still_eligible_and_occluded: bool,
    ) -> Option<T> {
        self.observe(target, point);
        (!self.cancelled && button == Button::Left && still_eligible_and_occluded)
            .then_some(self.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn gesture() -> Gesture<u32> {
        Gesture::begin(1, [10, 20], [3, 3], Button::Left, 0, true).unwrap()
    }
    #[test]
    fn only_explicit_unaccompanied_left_on_occluded_button_starts() {
        for button in [Button::Right, Button::Middle] {
            assert!(Gesture::begin(1, [0, 0], [3, 3], button, 0, true).is_none());
        }
        for held in 1..=7 {
            assert!(Gesture::begin(1, [0, 0], [3, 3], Button::Left, held, true).is_none());
        }
        assert!(Gesture::begin(1, [0, 0], [3, 3], Button::Left, 0, false).is_none());
    }
    #[test]
    fn jitter_boundary_completes_but_leaving_and_returning_does_not() {
        let mut small = gesture();
        small.observe(&1, [13, 17]);
        assert_eq!(small.finish(&1, [10, 20], Button::Left, true), Some(1));
        for point in [[14, 20], [10, 24], [i32::MIN, i32::MAX]] {
            let mut dragged = gesture();
            dragged.observe(&1, point);
            assert_eq!(dragged.finish(&1, [10, 20], Button::Left, true), None);
        }
    }
    #[test]
    fn target_change_cancel_wrong_button_and_unoccluded_up_never_complete() {
        let mut changed = gesture();
        changed.observe(&2, [10, 20]);
        assert_eq!(changed.finish(&1, [10, 20], Button::Left, true), None);
        let mut cancelled = gesture();
        cancelled.cancel();
        assert_eq!(cancelled.finish(&1, [10, 20], Button::Left, true), None);
        assert_eq!(gesture().finish(&2, [10, 20], Button::Left, true), None);
        assert_eq!(gesture().finish(&1, [10, 20], Button::Right, true), None);
        assert_eq!(gesture().finish(&1, [10, 20], Button::Left, false), None);
    }
    #[test]
    fn extreme_origin_uses_unsigned_difference_without_overflow() {
        let value = Gesture::begin(1, [i32::MIN, 0], [3, 3], Button::Left, 0, true).unwrap();
        assert_eq!(value.finish(&1, [i32::MAX, 0], Button::Left, true), None);
    }
}

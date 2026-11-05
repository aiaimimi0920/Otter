//! Live 等比适配几何；绘制与输入共用实际 f32 目标矩形，不授予输入转发权限。
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    source: [u32; 2],
    rect: [f32; 4],
}
impl Viewport {
    pub fn fit(source: [u32; 2], destination: [u32; 2]) -> Option<Self> {
        if source.contains(&0) || destination.contains(&0) {
            return None;
        }
        let [width, height] = source.map(|value| value as f32);
        let [output_width, output_height] = destination.map(|value| value as f32);
        let scale = (output_width / width).min(output_height / height);
        let left = (output_width - width * scale) * 0.5;
        let top = (output_height - height * scale) * 0.5;
        let rect = [left, top, left + width * scale, top + height * scale];
        if rect[2] <= rect[0] || rect[3] <= rect[1] || rect.iter().any(|value| !value.is_finite()) {
            return None;
        }
        Some(Self { source, rect })
    }
    pub fn destination(&self) -> [f32; 4] {
        self.rect
    }
    /// 输入 transport 使用的连续坐标；普通输入拒绝黑边，已有捕获手势才可 clamp。
    pub fn normalized(&self, point: [f64; 2], clamp: bool) -> Option<[f64; 2]> {
        if !point.iter().all(|value| value.is_finite()) || (!clamp && self.map(point).is_none()) {
            return None;
        }
        let [left, top, right, bottom] = self.rect.map(f64::from);
        Some([
            ((point[0] - left) / (right - left)).clamp(0.0, 1.0),
            ((point[1] - top) / (bottom - top)).clamp(0.0, 1.0),
        ])
    }
    /// 新输入只接受图像半开区域，留黑边和右/下边界不能变成源内点击。
    pub fn map(&self, point: [f64; 2]) -> Option<[u32; 2]> {
        let [left, top, right, bottom] = self.rect.map(f64::from);
        if !point.iter().all(|value| value.is_finite())
            || point[0] < left
            || point[1] < top
            || point[0] >= right
            || point[1] >= bottom
        {
            return None;
        }
        self.map_clamped(point)
    }
    /// 仅供已获授权且已有按下状态的拖动/释放使用；调用方不能据此允许黑边新按下。
    /// 返回帧像素，不是屏幕/客户区坐标；目标几何与身份仍须由输入 transport 校验。
    pub fn map_clamped(&self, point: [f64; 2]) -> Option<[u32; 2]> {
        if !point.iter().all(|value| value.is_finite()) {
            return None;
        }
        let [left, top, right, bottom] = self.rect.map(f64::from);
        let normalized = [
            ((point[0] - left) / (right - left)).clamp(0.0, 1.0),
            ((point[1] - top) / (bottom - top)).clamp(0.0, 1.0),
        ];
        Some(std::array::from_fn(|axis| {
            ((normalized[axis] * f64::from(self.source[axis])).floor() as u32)
                .min(self.source[axis] - 1)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wide_source_letterbox_matches_paint_and_rejects_black_bars() {
        let view = Viewport::fit([200, 100], [400, 400]).unwrap();
        assert_eq!(view.destination(), [0.0, 100.0, 400.0, 300.0]);
        assert_eq!(view.map([200.0, 200.0]), Some([100, 50]));
        assert_eq!(view.normalized([200.0, 200.0], false), Some([0.5, 0.5]));
        assert_eq!(view.normalized([-1.0, 500.0], false), None);
        assert_eq!(view.normalized([-1.0, 500.0], true), Some([0.0, 1.0]));
        assert_eq!(view.map([0.0, 100.0]), Some([0, 0]));
        assert_eq!(view.map([399.99, 299.99]), Some([199, 99]));
        for point in [
            [200.0, 99.99],
            [200.0, 300.0],
            [-0.01, 200.0],
            [400.0, 200.0],
        ] {
            assert!(view.map(point).is_none());
        }
    }
    #[test]
    fn portrait_source_and_resize_change_mapping_without_stale_offsets() {
        let old = Viewport::fit([100, 200], [400, 400]).unwrap();
        assert_eq!(old.destination(), [100.0, 0.0, 300.0, 400.0]);
        assert_eq!(old.map([200.0, 200.0]), Some([50, 100]));
        let new = Viewport::fit([100, 200], [200, 200]).unwrap();
        assert_eq!(new.destination(), [50.0, 0.0, 150.0, 200.0]);
        assert!(new.map([200.0, 200.0]).is_none());
        assert_eq!(new.map([100.0, 100.0]), Some([50, 100]));
    }
    #[test]
    fn captured_release_can_clamp_but_invalid_coordinates_never_map() {
        let view = Viewport::fit([200, 100], [400, 400]).unwrap();
        assert_eq!(view.map_clamped([-50.0, 500.0]), Some([0, 99]));
        assert_eq!(view.map_clamped([400.0, 300.0]), Some([199, 99]));
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(view.map([value, 200.0]).is_none());
            assert!(view.map_clamped([200.0, value]).is_none());
        }
        assert!(Viewport::fit([0, 1], [1, 1]).is_none());
        assert!(Viewport::fit([1, 1], [1, 0]).is_none());
        assert_eq!(
            Viewport::fit([1, 1], [1, 1]).unwrap().map([0.5, 0.5]),
            Some([0, 0])
        );
    }
}

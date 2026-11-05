//! 冻结帧上的有界选区手势；只存矩形和当前事务，不分配图像或历史。
use windows::Win32::Foundation::{POINT, RECT};

#[derive(Clone, Copy)]
enum Kind {
    New,
    Move,
    Resize(usize),
}
#[derive(Clone, Copy)]
struct Drag {
    point: POINT,
    before: Option<RECT>,
    kind: Kind,
}
#[derive(Clone, Copy)]
pub(super) struct Selection {
    width: u32,
    height: u32,
    pub rect: Option<RECT>,
    drag: Option<Drag>,
}
pub(super) fn handles(r: RECT) -> [POINT; 8] {
    let (x, y) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2);
    [
        POINT {
            x: r.left,
            y: r.top,
        },
        POINT { x, y: r.top },
        POINT {
            x: r.right,
            y: r.top,
        },
        POINT { x: r.right, y },
        POINT {
            x: r.right,
            y: r.bottom,
        },
        POINT { x, y: r.bottom },
        POINT {
            x: r.left,
            y: r.bottom,
        },
        POINT { x: r.left, y },
    ]
}
impl Selection {
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rect: None,
            drag: None,
        }
    }
    pub fn begin(&mut self, point: POINT, tolerance: i32) {
        let kind = self.rect.map_or(Kind::New, |rect| {
            let handle = handles(rect)
                .iter()
                .enumerate()
                .map(|(index, p)| {
                    (
                        index,
                        (p.x - point.x) as i64 * (p.x - point.x) as i64
                            + (p.y - point.y) as i64 * (p.y - point.y) as i64,
                    )
                })
                .filter(|(_, distance)| *distance <= (tolerance as i64).pow(2))
                .min_by_key(|(_, distance)| *distance);
            if let Some((index, _)) = handle {
                Kind::Resize(index)
            } else if point.x > rect.left
                && point.x < rect.right
                && point.y > rect.top
                && point.y < rect.bottom
            {
                Kind::Move
            } else {
                Kind::New
            }
        });
        self.drag = Some(Drag {
            point,
            before: self.rect,
            kind,
        });
        if matches!(kind, Kind::New) {
            self.rect = None;
        }
    }
    pub fn update(&mut self, point: POINT) {
        let Some(drag) = self.drag else {
            return;
        };
        self.rect = match drag.kind {
            Kind::New => super::capture_crop::rectangle(drag.point, point, self.width, self.height),
            Kind::Move => drag
                .before
                .map(|rect| self.moved(rect, point.x - drag.point.x, point.y - drag.point.y)),
            Kind::Resize(index) => drag.before.map(|mut rect| {
                // 使用相对按下位置的位移，命中容差范围内的点击不能让边缘跳到指针。
                let dx = point.x - drag.point.x;
                let dy = point.y - drag.point.y;
                if matches!(index, 0 | 6 | 7) {
                    rect.left = (rect.left + dx).clamp(0, rect.right - 1);
                }
                if matches!(index, 2..=4) {
                    rect.right = (rect.right + dx).clamp(rect.left + 1, self.width as i32);
                }
                if matches!(index, 0..=2) {
                    rect.top = (rect.top + dy).clamp(0, rect.bottom - 1);
                }
                if matches!(index, 4..=6) {
                    rect.bottom = (rect.bottom + dy).clamp(rect.top + 1, self.height as i32);
                }
                rect
            }),
        };
    }
    fn moved(&self, rect: RECT, dx: i32, dy: i32) -> RECT {
        let dx = dx.clamp(-rect.left, self.width as i32 - rect.right);
        let dy = dy.clamp(-rect.top, self.height as i32 - rect.bottom);
        RECT {
            left: rect.left + dx,
            right: rect.right + dx,
            top: rect.top + dy,
            bottom: rect.bottom + dy,
        }
    }
    pub fn finish(&mut self, point: POINT) {
        self.update(point);
        self.drag = None;
    }
    pub fn cancel_drag(&mut self) {
        if let Some(drag) = self.drag.take() {
            self.rect = drag.before;
        }
    }
    pub fn nudge(&mut self, dx: i32, dy: i32) {
        if self.drag.is_none() {
            self.rect = self.rect.map(|rect| self.moved(rect, dx, dy));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: i32, y: i32) -> POINT {
        POINT { x, y }
    }
    fn region() -> Selection {
        let mut model = Selection::new(100, 80);
        model.begin(p(20, 20), 4);
        model.finish(p(60, 60));
        model
    }
    fn bounds(model: Selection) -> (i32, i32, i32, i32) {
        let r = model.rect.unwrap();
        (r.left, r.top, r.right, r.bottom)
    }
    #[test]
    fn move_clamps_whole_rectangle_and_cancel_restores_transaction() {
        let mut model = region();
        model.begin(p(40, 40), 4);
        model.update(p(-100, 200));
        assert_eq!(bounds(model), (0, 40, 40, 80));
        model.cancel_drag();
        assert_eq!(bounds(model), (20, 20, 60, 60));
        model.nudge(100, -100);
        assert_eq!(bounds(model), (60, 0, 100, 40));
    }
    #[test]
    fn all_handles_resize_only_their_edges_and_never_invert() {
        for index in 0..8 {
            let mut model = region();
            let start = handles(model.rect.unwrap())[index];
            model.begin(start, 4);
            model.finish(p(start.x + 5, start.y + 7));
            let r = model.rect.unwrap();
            assert_eq!(r.left, if matches!(index, 0 | 6 | 7) { 25 } else { 20 });
            assert_eq!(r.right, if matches!(index, 2..=4) { 65 } else { 60 });
            assert_eq!(r.top, if matches!(index, 0..=2) { 27 } else { 20 });
            assert_eq!(r.bottom, if matches!(index, 4..=6) { 67 } else { 60 });
        }
        let mut model = region();
        model.begin(p(20, 20), 4);
        model.finish(p(100, 100));
        assert_eq!(bounds(model), (59, 59, 60, 60));
    }
    #[test]
    fn outside_drag_replaces_region_and_capture_loss_restores_old_one() {
        let mut model = region();
        model.begin(p(90, 70), 4);
        model.update(p(75, 65));
        assert_eq!(bounds(model), (75, 65, 90, 70));
        model.cancel_drag();
        assert_eq!(bounds(model), (20, 20, 60, 60));
    }
    #[test]
    fn clicking_near_handle_does_not_jump_edge() {
        let mut model = region();
        model.begin(p(22, 21), 6);
        model.finish(p(22, 21));
        assert_eq!(bounds(model), (20, 20, 60, 60));
    }
}

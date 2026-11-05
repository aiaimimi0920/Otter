//! 物理屏幕几何及三态规则；不读取 HWND、图像、参数值或执行权限。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Mode {
    #[default]
    All,
    Border,
    None,
}
impl Mode {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Border,
            Self::Border => Self::None,
            Self::None => Self::All,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl Rect {
    pub fn valid(self) -> bool {
        self.right > self.left && self.bottom > self.top
    }
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.left && point.x < self.right && point.y >= self.top && point.y < self.bottom
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Side {
    Input,
    Output,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Port {
    pub node: usize,
    pub side: Side,
    pub index: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Node {
    pub endpoint: usize,
    pub hwnd: isize,
    pub marker: isize,
    pub rect: Rect,
    pub dpi: u32,
    pub minified: bool,
    pub expanded: bool,
    pub interactive: bool,
    pub protected: bool,
    pub inputs: usize,
    pub outputs: usize,
    pub input_visibility: std::rc::Rc<[bool]>,
    pub output_visibility: std::rc::Rc<[bool]>,
}
impl Node {
    pub fn frame(&self, mode: Mode) -> bool {
        self.rect.valid() && !self.minified && mode != Mode::None
    }
    pub fn ports(&self, mode: Mode, hide_collapsed: bool) -> bool {
        self.rect.valid()
            && ((!self.minified && self.expanded) || (mode == Mode::All && !hide_collapsed))
    }
    pub fn point(&self, side: Side, index: usize) -> Option<Point> {
        let (count, mask) = match side {
            Side::Input => (self.inputs, &self.input_visibility),
            Side::Output => (self.outputs, &self.output_visibility),
        };
        if index >= count || !self.rect.valid() || mask.get(index) == Some(&false) {
            return None;
        }
        // 目录索引保持稳定，位置按可见顺序紧排；隐藏不重命名端口或改变图。
        let rank = (0..index).filter(|&i| mask.get(i) != Some(&false)).count();
        let count = (0..count).filter(|&i| mask.get(i) != Some(&false)).count();
        let height = i64::from(self.rect.bottom) - i64::from(self.rect.top);
        Some(Point {
            x: match side {
                Side::Input => self.rect.left,
                Side::Output => self.rect.right,
            },
            y: (i64::from(self.rect.top) + height * (2 * rank as i64 + 1) / (2 * count as i64))
                as i32,
        })
    }
    pub fn radius(&self) -> i32 {
        let logical = if self.minified { 4 } else { 12 };
        (logical * self.dpi.clamp(96, 768) as i32 / 96).max(1)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Edge {
    pub from: Port,
    pub to: Port,
}
pub(super) fn edge_style(mode: Mode, from: &Node, to: &Node) -> (bool, bool) {
    (
        mode != Mode::None,
        if mode == Mode::None {
            from.expanded && to.expanded
        } else {
            from.expanded || to.expanded
        },
    )
}
pub(super) fn moved(start: Point, current: Point, dpi: u32) -> bool {
    let x = i64::from(current.x) - i64::from(start.x);
    let y = i64::from(current.y) - i64::from(start.y);
    let threshold = i64::from(dpi.clamp(96, 768)) * 5 / 96;
    // 避免跨极端屏幕坐标时平方溢出；先比较绝对轴长。
    x.abs() >= threshold || y.abs() >= threshold || x * x + y * y >= threshold * threshold
}
#[cfg(test)]
#[path = "workspace_chrome_model_tests.rs"]
mod tests;

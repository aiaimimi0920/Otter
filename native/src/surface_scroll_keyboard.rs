//! 非文本编辑键的本地滚动语义；从锚点向祖先消费一次，hidden 轴保持不可手动滚动。
use super::*;
#[derive(Clone, Copy)]
pub(crate) enum Command {
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}
impl Scroll {
    pub(crate) fn keyboard(&mut self, geometry: &Geometry, anchor: &str, command: Command) -> bool {
        let (axis, delta, page) = match command {
            Command::Left => (0, -40., false),
            Command::Right => (0, 40., false),
            Command::Up => (1, -40., false),
            Command::Down => (1, 40., false),
            Command::PageUp => (1, -1., true),
            Command::PageDown => (1, 1., true),
            Command::Home => (1, -f32::MAX, false),
            Command::End => (1, f32::MAX, false),
        };
        self.wheel(geometry, anchor, axis, delta, page)
    }
}

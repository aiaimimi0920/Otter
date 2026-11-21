//! 连续展开行的非交互底板投影；每个钉图独立呈现，不扩大端口条的输入 region。
use super::super::*;
use neuro_mini_ui::Frame;
use windows::Win32::Graphics::Gdi::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Group {
    frame: Frame,
    dpi: u32,
    endpoint: usize,
}
#[derive(Clone, Default, PartialEq, Eq)]
pub(in super::super) struct Backdrops {
    groups: Vec<Group>,
    occluders: Vec<Rect>,
}
pub(in super::super) fn collect(ui: &Ui) -> Backdrops {
    let mut result = Backdrops::default();
    if ui.blocked() {
        return result;
    }
    let nodes = ui.nodes.borrow().clone();
    for node in &nodes {
        if !node.ports(ui.mode.get(), node.hide_collapsed_endpoints) {
            continue;
        }
        for (side, count) in [(Side::Input, node.inputs), (Side::Output, node.outputs)] {
            let mut run: Option<Group> = None;
            let mut length = 0;
            for index in 0..=count {
                let port = Port {
                    node: node.endpoint,
                    side,
                    index,
                };
                let row = (index < count)
                    .then(|| port_visual::layout(ui, port))
                    .flatten()
                    .filter(|layout| layout.expanded)
                    .map(|layout| {
                        let rect = draw::port_rect(ui, port, Point::default(), 0);
                        Frame {
                            y: rect.top,
                            height: rect.bottom - rect.top,
                            ..layout.frame
                        }
                    });
                if let Some(row) = row.filter(|row| row.height > 0) {
                    if let Some(group) = &mut run {
                        group.frame.height =
                            (row.y + row.height - group.frame.y).max(group.frame.height);
                    } else {
                        run = Some(Group {
                            frame: row,
                            dpi: node.dpi,
                            endpoint: node.endpoint,
                        });
                    }
                    length += 1;
                } else {
                    if length > 1 {
                        result.groups.push(run.expect("consecutive expanded rows"));
                    }
                    run = None;
                    length = 0;
                }
            }
        }
    }
    if !result.groups.is_empty() {
        result.occluders = nodes
            .iter()
            .map(|node| {
                let padding = if node.frame(ui.mode.get()) {
                    desktop::pin_border_style::width(node.dpi) as i32
                } else {
                    0
                };
                Rect {
                    left: node.rect.left.saturating_sub(padding),
                    top: node.rect.top.saturating_sub(padding),
                    right: node.rect.right.saturating_add(padding),
                    bottom: node.rect.bottom.saturating_add(padding),
                }
            })
            .collect();
    }
    result
}
impl Backdrops {
    pub(in super::super) fn for_node(&self, endpoint: usize) -> Self {
        let groups: Vec<_> = self
            .groups
            .iter()
            .filter(|g| g.endpoint == endpoint)
            .copied()
            .collect();
        // 只保留与该底板实际相交的遮挡物；远处钉图移动不应让所有节点重新提交像素。
        let occluders = self
            .occluders
            .iter()
            .filter(|rect| {
                groups.iter().any(|group| {
                    let f = group.frame;
                    rect.left < f.x + f.width
                        && rect.right > f.x
                        && rect.top < f.y + f.height
                        && rect.bottom > f.y
                })
            })
            .copied()
            .collect();
        Self { groups, occluders }
    }
    pub(in super::super) fn bounds(&self) -> Option<Rect> {
        let f = self.groups.first()?.frame;
        Some(self.include_bounds(Rect {
            left: f.x,
            top: f.y,
            right: f.x + f.width,
            bottom: f.y + f.height,
        }))
    }
    pub(in super::super) fn include_bounds(&self, mut rect: Rect) -> Rect {
        for group in &self.groups {
            let f = group.frame;
            rect.left = rect.left.min(f.x);
            rect.top = rect.top.min(f.y);
            rect.right = rect.right.max(f.x + f.width);
            rect.bottom = rect.bottom.max(f.y + f.height);
        }
        rect
    }
    pub(in super::super) unsafe fn paint(&self, ui: &Ui, dc: HDC, origin: POINT) -> Result<()> {
        if self.groups.is_empty() {
            return Ok(());
        }
        if ui.port_painter.borrow().is_none() {
            *ui.port_painter.borrow_mut() = Some(neuro_mini_ui::CapsulePainter::new()?);
        }
        let saved = SaveDC(dc);
        if saved == 0 {
            return Err(Error::from_win32());
        }
        for rect in &self.occluders {
            ExcludeClipRect(
                dc,
                rect.left - origin.x,
                rect.top - origin.y,
                rect.right - origin.x,
                rect.bottom - origin.y,
            );
        }
        let result = (|| {
            let mut painter = ui.port_painter.borrow_mut();
            let painter = painter.as_mut().expect("prepared capsule painter");
            for group in &self.groups {
                painter.backdrop(
                    dc,
                    Frame {
                        x: group.frame.x - origin.x,
                        y: group.frame.y - origin.y,
                        ..group.frame
                    },
                    group.dpi,
                )?;
            }
            Ok(())
        })();
        RestoreDC(dc, saved).ok()?;
        result
    }
}

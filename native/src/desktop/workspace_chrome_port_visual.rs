//! Port painting, hit rectangles, and link anchors consume identical theme geometry without changing connection-layer HWND Z-order.
use super::*;
use neuro_mini_ui::{
    CapsuleAction, CapsuleAppearance, CapsuleLayout, CapsuleSide, CapsuleSize, RailLayout,
};
use windows::Win32::Graphics::Gdi::HDC;
pub(super) fn layout(ui: &Ui, port: Port) -> Option<CapsuleLayout> {
    SideGeometry::new(ui, &ui.node(port.node)?, port.side).layout(port.index, true)
}

#[cfg(test)]
pub(super) fn unclipped(ui: &Ui, port: Port) -> Option<CapsuleLayout> {
    SideGeometry::new(ui, &ui.node(port.node)?, port.side).layout(port.index, false)
}

/// One side's geometry for the current operation. Never retained across window messages.
pub(super) struct SideGeometry<'a> {
    ui: &'a Ui,
    node: &'a Node,
    side: Side,
    pub(super) rail: Option<RailLayout>,
}
impl<'a> SideGeometry<'a> {
    pub(super) fn new(ui: &'a Ui, node: &'a Node, side: Side) -> Self {
        Self {
            ui,
            node,
            side,
            rail: (side == Side::Input)
                .then(|| parameter_rail::layout(ui, node))
                .flatten(),
        }
    }

    pub(super) fn port(&self, index: usize) -> Port {
        Port {
            node: self.node.endpoint,
            side: self.side,
            index,
        }
    }

    pub(super) fn indices(&self) -> std::ops::Range<usize> {
        0..match self.side {
            Side::Input => self.node.inputs,
            Side::Output => self.node.outputs,
        }
    }

    pub(super) fn clip(&self, index: usize) -> Option<RECT> {
        let rail = self.rail.as_ref()?;
        let f = rail.visible(index).unwrap_or(neuro_mini_ui::Frame {
            height: 0,
            ..rail.row(index)?
        });
        Some(RECT {
            left: f.x,
            top: f.y,
            right: f.x + f.width,
            bottom: f.y + f.height,
        })
    }

    pub(super) fn rect(&self, index: usize, layout: &CapsuleLayout) -> RECT {
        let f = layout.frame;
        let mut rect = RECT {
            left: f.x,
            top: f.y,
            right: f.x.saturating_add(f.width),
            bottom: f.y.saturating_add(f.height),
        };
        if let Some(clip) = self.clip(index) {
            rect.top = rect.top.max(clip.top);
            rect.bottom = rect.bottom.min(clip.bottom);
        }
        rect
    }

    pub(super) fn layout(&self, index: usize, clipped: bool) -> Option<CapsuleLayout> {
        geometry(
            self.ui,
            self.port(index),
            self.node,
            self.rail.as_ref(),
            clipped,
        )
    }

    fn action_contains(&self, index: usize, layout: &CapsuleLayout, point: Point) -> bool {
        !self
            .clip(index)
            .is_some_and(|clip| !Rect::from(clip).contains(point))
            && layout.inline_action(self.node.dpi).is_some_and(|f| {
                (f.x..f.x + f.width).contains(&point.x) && (f.y..f.y + f.height).contains(&point.y)
            })
    }
}
fn geometry(
    ui: &Ui,
    port: Port,
    node: &Node,
    rail: Option<&RailLayout>,
    clipped: bool,
) -> Option<CapsuleLayout> {
    let mut edge = if ui.disclosed.borrow().is_empty() {
        node.point(port.side, port.index)
    } else {
        node.point_revealed(port.side, port.index, |index| {
            disclosure::expanded(ui, Port { index, ..port }, node)
        })
    }?;
    if let Some(rail) = rail {
        if clipped {
            rail.visible(port.index)?;
        }
        let row = rail.row(port.index)?;
        edge.y = row.y + row.height / 2;
    }
    let expanded = disclosure::expanded(ui, port, node);
    Some(CapsuleLayout::new(
        (edge.x, edge.y),
        node.dpi,
        match port.side {
            Side::Input => CapsuleSide::Input,
            Side::Output => CapsuleSide::Output,
        },
        expanded,
        if node.minified {
            CapsuleSize::Compact
        } else {
            CapsuleSize::Normal
        },
    ))
}
/// Inline bodies are not connection points; only anchors start expansion/link gestures, while eye actions have separate hit slots.
pub(super) fn anchor_hit(ui: &Ui, port: Port, point: Point) -> bool {
    let Some(node) = ui.node(port.node) else {
        return false;
    };
    let side = SideGeometry::new(ui, &node, port.side);
    if side
        .clip(port.index)
        .is_some_and(|clip| !Rect::from(clip).contains(point))
    {
        return false;
    }
    side.layout(port.index, true).is_some_and(|layout| {
        let anchor = layout.anchor;
        (anchor.x..anchor.x + anchor.width).contains(&point.x)
            && (anchor.y..anchor.y + anchor.height).contains(&point.y)
    })
}
pub(super) fn action_hit(ui: &Ui, port: Port, point: Point) -> bool {
    let Some(node) = ui.node(port.node) else {
        return false;
    };
    let side = SideGeometry::new(ui, &node, port.side);
    side.layout(port.index, true)
        .is_some_and(|layout| side.action_contains(port.index, &layout, point))
}
pub(super) unsafe fn paint(hwnd: HWND, ui: &Ui, dc: HDC, origin: POINT) -> Result<()> {
    let binding = ui
        .input_windows
        .borrow()
        .iter()
        .find(|(h, _, _)| *h == hwnd)
        .copied();
    let Some((_, endpoint, side)) = binding else {
        return Ok(());
    };
    let Some(scene) = ui.scene.borrow().clone() else {
        return Ok(());
    };
    let Some(node) = ui.node(endpoint) else {
        return Ok(());
    };
    let Some(entry) = scene.catalog.endpoints.get(endpoint) else {
        return Ok(());
    };
    let appearances = match side {
        Side::Input => &entry.input_presentation,
        Side::Output => &entry.output_presentation,
    };
    if ui.port_painter.borrow().is_none() {
        *ui.port_painter.borrow_mut() = Some(neuro_mini_ui::CapsulePainter::new()?);
    }
    let mut painter = ui.port_painter.borrow_mut();
    let painter = painter.as_mut().expect("port painter was prepared");
    let geometry = SideGeometry::new(ui, &node, side);
    for (index, appearance) in appearances.iter().enumerate() {
        let port = geometry.port(index);
        let Some(mut layout) = geometry.layout(index, true) else {
            continue;
        };
        let saved = windows::Win32::Graphics::Gdi::SaveDC(dc);
        if let Some(clip) = geometry.clip(index) {
            windows::Win32::Graphics::Gdi::IntersectClipRect(
                dc,
                clip.left - origin.x,
                clip.top - origin.y,
                clip.right - origin.x,
                clip.bottom - origin.y,
            );
        }
        let active = match side {
            Side::Input => &node.input_visibility,
            Side::Output => &node.output_visibility,
        }
        .get(index)
        .copied()
        .unwrap_or(true);
        let pressed = ui.press.borrow().as_ref().is_some_and(|p| {
            p.action == Some(port) && geometry.action_contains(index, &layout, p.current)
        });
        for frame in [&mut layout.frame, &mut layout.anchor, &mut layout.body] {
            frame.x = frame.x.saturating_sub(origin.x);
            frame.y = frame.y.saturating_sub(origin.y);
        }
        let result = painter.paint(
            dc,
            layout,
            node.dpi,
            CapsuleAppearance {
                icon: appearance.icon,
                label: &appearance.label,
                enabled: node.interactive,
                focused: ui.keyboard.focused() == Some(port),
                action: Some(CapsuleAction {
                    icon: neuro_mini_ui::Icon::svg(if active {
                        include_str!("../../assets/edit-strip/PortVisible.svg")
                    } else {
                        include_str!("../../assets/edit-strip/PortHidden.svg")
                    }),
                    active,
                    pressed,
                }),
                background: 0,
            },
        );
        windows::Win32::Graphics::Gdi::RestoreDC(dc, saved).ok()?;
        result?;
    }
    if side == Side::Input && parameter_rail::active(ui, &node) {
        if let Some(rail) = geometry.rail {
            rail.paint_scrollbar(dc, (origin.x, origin.y));
        }
    }
    Ok(())
}

//! 端口绘制、命中矩形和连线锚点消费同一主题几何；不改变连接层的 HWND 层序。
use super::*;
use neuro_mini_ui::{CapsuleAction, CapsuleAppearance, CapsuleLayout, CapsuleSide, CapsuleSize};
use windows::Win32::Graphics::Gdi::HDC;
pub(super) fn layout(ui: &Ui, port: Port) -> Option<CapsuleLayout> {
    if port.side == Side::Input {
        if let Some(rail) = parameter_rail::layout(ui, &ui.node(port.node)?) {
            rail.visible(port.index)?;
        }
    }
    unclipped(ui, port)
}
pub(super) fn unclipped(ui: &Ui, port: Port) -> Option<CapsuleLayout> {
    let node = ui.node(port.node)?;
    let mut edge = if ui.disclosed.borrow().is_empty() {
        node.point(port.side, port.index)
    } else {
        node.point_revealed(port.side, port.index, |index| {
            disclosure::expanded(ui, Port { index, ..port }, &node)
        })
    }?;
    if port.side == Side::Input {
        if let Some(rail) = parameter_rail::layout(ui, &node) {
            let row = rail.row(port.index)?;
            edge.y = row.y + row.height / 2;
        }
    }
    let expanded = disclosure::expanded(ui, port, &node);
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
/// 行内主体不是连接点；仅 anchor 启动展开/连线手势，眼睛操作有自己的命中槽。
pub(super) fn anchor_hit(ui: &Ui, port: Port, point: Point) -> bool {
    if parameter_rail::clip(ui, port).is_some_and(|clip| !Rect::from(clip).contains(point)) {
        return false;
    }
    layout(ui, port).is_some_and(|layout| {
        let anchor = layout.anchor;
        (anchor.x..anchor.x + anchor.width).contains(&point.x)
            && (anchor.y..anchor.y + anchor.height).contains(&point.y)
    })
}
pub(super) fn action_hit(ui: &Ui, port: Port, point: Point) -> bool {
    if parameter_rail::clip(ui, port).is_some_and(|clip| !Rect::from(clip).contains(point)) {
        return false;
    }
    layout(ui, port)
        .and_then(|layout| layout.inline_action(ui.node(port.node)?.dpi))
        .is_some_and(|f| {
            (f.x..f.x + f.width).contains(&point.x) && (f.y..f.y + f.height).contains(&point.y)
        })
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
    for (index, appearance) in appearances.iter().enumerate() {
        let port = Port {
            node: endpoint,
            side,
            index,
        };
        let Some(mut layout) = layout(ui, port) else {
            continue;
        };
        let saved = windows::Win32::Graphics::Gdi::SaveDC(dc);
        if let Some(clip) = parameter_rail::clip(ui, port) {
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
        let pressed = ui
            .press
            .borrow()
            .as_ref()
            .is_some_and(|p| p.action == Some(port) && action_hit(ui, port, p.current));
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
        if let Some(rail) = parameter_rail::layout(ui, &node) {
            rail.paint_scrollbar(dc, (origin.x, origin.y));
        }
    }
    Ok(())
}

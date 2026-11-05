//! 提交布局时生成有序绘制指令；绘制期间不解析样式、不重新分配 native 画刷。
#[path = "surface_paint_controls.rs"]
mod controls;
use crate::{
    surface_background::Background,
    surface_button_visual::{PaintColor, State},
    surface_clip::{self, Rect},
    surface_color::Color,
    surface_layout_tree::Tree,
    surface_length::Context,
    surface_native_measure::TextFrame,
    surface_scene::Node,
};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{
        Common::{D2D1_COLOR_F, D2D_RECT_F},
        ID2D1RenderTarget, ID2D1SolidColorBrush, D2D1_LAYER_PARAMETERS,
    },
};
enum Command {
    Control(crate::surface_embedded_control::Marker),
    Image(crate::surface_image_paint::Paint),
    ImagePlaceholder(crate::surface_image_paint::placeholder::Paint, PaintColor),
    Switch(crate::surface_switch::Switch),
    Progress(crate::surface_progress::Progress),
    Push(Rect, f32),
    Mask(Rect, usize),
    Pop,
    Background(Background, PaintColor),
    Border(crate::surface_border::Border, PaintColor),
    Text(usize, PaintColor),
    Focus(crate::surface_focus_draw::Focus),
}
pub(crate) struct PaintFrame {
    commands: Vec<Command>,
    clips: crate::surface_round_clip::Clips,
    select_ink: std::collections::BTreeMap<String, Color>,
    edit_colors: std::collections::BTreeMap<String, [Color; 2]>,
    controls: std::collections::BTreeMap<String, crate::surface_embedded_control::Paint>,
}
impl Drop for PaintFrame {
    fn drop(&mut self) {
        // 即使外部像素 owner 仍存活，旧帧退场也不能把旧 target 留在不可见图片中。
        self.release_device_resources();
    }
}
impl PaintFrame {
    pub(crate) fn edit_colors(&self, id: &str) -> Option<[Color; 2]> {
        self.edit_colors.get(id).copied()
    }
    pub(crate) fn select_ink(&self, id: &str) -> Option<Color> {
        self.select_ink.get(id).copied()
    }
    pub(crate) fn build_with_images(
        tree: &Tree<'_>,
        text: &TextFrame,
        context: Context,
        font: impl Fn(&Node) -> Option<f32>,
        image: impl Fn(&Node) -> Option<std::rc::Rc<crate::surface_image_paint::Bitmap>>,
    ) -> Self {
        let texts: std::collections::BTreeMap<_, _> = text
            .items()
            .iter()
            .enumerate()
            .map(|(i, item)| (item.node_id(), i))
            .collect();
        let mut colors = Vec::with_capacity(tree.nodes().len());
        let mut select_ink = std::collections::BTreeMap::new();
        let mut edit_colors = std::collections::BTreeMap::new();
        let clips = surface_clip::ancestors_with_border(tree, |node| {
            crate::surface_box::borders(
                node,
                Context {
                    font: font(node).unwrap_or(context.font),
                    ..context
                },
            )
        });
        // 组层覆盖子树可见范围，而非只用父盒子裁掉 overflow:visible 的子节点。
        let mut bounds: Vec<_> = tree
            .nodes()
            .iter()
            .zip(&clips)
            .map(|(placed, clip)| Rect::from_box(placed.origin, placed.size).intersect(*clip))
            .collect();
        for index in (0..bounds.len()).rev() {
            if let Some(parent) = tree.nodes()[index].parent {
                bounds[parent] = union(bounds[parent], bounds[index]);
            }
        }
        let mut hidden = Vec::with_capacity(tree.nodes().len());
        let mut stack: Vec<(usize, usize)> = Vec::new();
        let mut shapes = Vec::new();
        let mut commands = Vec::new();
        for (index, (placed, clip)) in tree.nodes().iter().zip(clips).enumerate() {
            while stack
                .last()
                .is_some_and(|(index, _)| Some(*index) != placed.parent)
            {
                commands.extend((0..stack.pop().unwrap().1).map(|_| Command::Pop));
            }
            let opacity = placed
                .node
                .style()
                .get("opacity")
                .and_then(|value| value.as_f64())
                .filter(|value| value.is_finite())
                .map(|value| value.clamp(0., 1.) as f32)
                .unwrap_or(1.);
            let invisible = placed.parent.is_some_and(|parent| hidden[parent]) || opacity == 0.;
            hidden.push(invisible);
            let layer = !invisible && opacity < 1. && !bounds[index].empty();
            stack.push((index, usize::from(layer)));
            if layer {
                commands.push(Command::Push(bounds[index], opacity));
            }
            let inherited = placed
                .parent
                .map(|index| colors[index])
                .unwrap_or(Color::BLACK);
            let declared = placed
                .node
                .style()
                .get("color")
                .and_then(|v| v.as_str())
                .and_then(|value| Color::parse(value, inherited));
            let foreground = declared.unwrap_or(inherited);
            if placed.node.kind() == crate::surface_scene::Kind::Textarea {
                let background = placed
                    .node
                    .style()
                    .get("background")
                    .and_then(|v| v.as_str())
                    .and_then(|v| Color::parse(v, foreground))
                    .unwrap_or(Color([17. / 255., 23. / 255., 32. / 255., 1.]));
                edit_colors.insert(placed.node.id().to_owned(), [foreground, background]);
            }
            // 控件 class 有自己的默认文字色；只有节点声明覆盖默认，currentcolor 使用父计算值。
            if placed.node.kind() == crate::surface_scene::Kind::Select {
                if let Some(color) = declared {
                    select_ink.insert(placed.node.id().to_owned(), color);
                }
            }
            colors.push(foreground);
            if invisible {
                continue;
            }
            if placed.node.kind() == crate::surface_scene::Kind::Switch {
                commands.push(Command::Switch(crate::surface_switch::Switch::new(
                    placed, clip,
                )));
                commands.push(Command::Focus(crate::surface_focus_draw::Focus::new(
                    placed, clip,
                )));
                continue;
            }
            if placed.node.kind() == crate::surface_scene::Kind::Progress {
                commands.push(Command::Progress(crate::surface_progress::Progress::new(
                    placed, clip,
                )));
                continue;
            }
            let local = Context {
                font: font(placed.node).unwrap_or(context.font),
                ..context
            };
            let shape = Background::new(placed, clip, local);
            let rounded = shape.as_ref().and_then(|shape| {
                shape.padding_round(crate::surface_box::borders(placed.node, local))
            });
            // 替换元素裁剪到 content edge；与容器子树的 padding edge 圆角不同。
            let image_round = (placed.node.kind() == crate::surface_scene::Kind::Image)
                .then(|| {
                    shape.as_ref().and_then(|shape| {
                        shape.content_round(crate::surface_box::inset(
                            placed.node,
                            placed.containing_width,
                            local,
                        ))
                    })
                })
                .flatten();
            let borders = shape
                .as_ref()
                .map(|shape| {
                    crate::surface_border::style::build(shape, placed.node, local, foreground)
                })
                .unwrap_or_default();
            if let Some(background) = placed
                .node
                .style()
                .get("background")
                .and_then(|v| v.as_str())
                .and_then(|value| Color::parse(value, foreground))
            {
                if background.0[3] != 0. {
                    if let Some(shape) = shape {
                        commands.push(Command::Background(
                            shape,
                            PaintColor::new(placed.node, "background", background),
                        ));
                    }
                }
            }
            commands.extend(
                borders
                    .into_iter()
                    .map(|(border, color)| Command::Border(border, color)),
            );
            if placed.node.kind() == crate::surface_scene::Kind::Image {
                let paint = image(placed.node)
                    .and_then(|image| {
                        crate::surface_image_paint::Paint::new(placed, clip, local, image)
                    })
                    .map(Command::Image)
                    .or_else(|| {
                        placed
                            .node
                            .image_placeholder()
                            .then(|| {
                                crate::surface_image_paint::placeholder::Paint::new(
                                    placed, clip, local,
                                )
                            })
                            .flatten()
                            .map(|paint| {
                                Command::ImagePlaceholder(
                                    paint,
                                    PaintColor::new(placed.node, "color", foreground),
                                )
                            })
                    });
                if let Some(paint) = paint {
                    let mask = image_round.map(|rounded| {
                        let index = shapes.len();
                        shapes.push(rounded);
                        Command::Mask(
                            Rect::from_box(placed.origin, placed.size).intersect(clip),
                            index,
                        )
                    });
                    let masked = mask.is_some();
                    if let Some(mask) = mask {
                        commands.push(mask);
                    }
                    commands.push(paint);
                    if let Some(index) = texts.get(placed.node.id()) {
                        commands.push(Command::Text(
                            *index,
                            PaintColor::new(placed.node, "color", foreground),
                        ));
                    }
                    if masked {
                        commands.push(Command::Pop);
                    }
                }
            }
            if let Some(index) = texts
                .get(placed.node.id())
                .filter(|_| placed.node.kind() != crate::surface_scene::Kind::Image)
            {
                commands.push(Command::Text(
                    *index,
                    PaintColor::new(placed.node, "color", foreground),
                ));
            }
            if placed.node.kind() == crate::surface_scene::Kind::Button {
                commands.push(Command::Focus(crate::surface_focus_draw::Focus::new(
                    placed, clip,
                )));
            }
            if crate::surface_embedded_control::is_control(placed.node.kind()) {
                if let Some(round) = rounded {
                    let index = shapes.len();
                    shapes.push(round);
                    commands.push(Command::Mask(
                        Rect::from_box(placed.origin, placed.size).intersect(clip),
                        index,
                    ));
                }
                commands.push(Command::Control(
                    crate::surface_embedded_control::Marker::new(placed, clip, local, declared),
                ));
                if rounded.is_some() {
                    commands.push(Command::Pop);
                }
            }
            // 使用计算值：单轴 hidden/auto 会把另一轴 visible 归一为 auto。
            if !placed.node.children().is_empty()
                && crate::surface_overflow::computed(placed.node)
                    .iter()
                    .all(|axis| axis.clips())
            {
                if let Some(rounded) = rounded {
                    let index = shapes.len();
                    shapes.push(rounded);
                    commands.push(Command::Mask(
                        Rect::from_box(placed.origin, placed.size).intersect(clip),
                        index,
                    ));
                    stack.last_mut().unwrap().1 += 1;
                }
            }
        }
        while let Some((_, layers)) = stack.pop() {
            commands.extend((0..layers).map(|_| Command::Pop));
        }
        Self {
            commands,
            select_ink,
            edit_colors,
            controls: Default::default(),
            clips: crate::surface_round_clip::Clips::new(shapes),
        }
    }
    pub(crate) fn release_device_resources(&self) {
        for command in &self.commands {
            if let Command::Image(image) = command {
                image.clear();
            }
        }
    }
    pub(crate) fn draw(
        &self,
        text: &TextFrame,
        target: &ID2D1RenderTarget,
        brush: &ID2D1SolidColorBrush,
        visual: State<'_>,
        switch_value: impl Fn(&str) -> Option<bool>,
        selection: Option<&crate::surface_presentation::Selection>,
    ) -> Result<()> {
        let original = unsafe { brush.GetColor() };
        let clips = self.clips.prepare(target)?;
        let selection = selection.and_then(|selection| selection.project(text));
        let result = (|| {
            let mut layers = Layers { target, depth: 0 };
            for command in &self.commands {
                let color = match command {
                    Command::Image(image) => {
                        image.draw(target)?;
                        continue;
                    }
                    Command::Switch(switch) => {
                        switch.draw(target, brush, &switch_value, visual);
                        continue;
                    }
                    Command::Focus(indicator) => {
                        indicator.draw(visual.focus, target, brush);
                        continue;
                    }
                    Command::Progress(progress) => {
                        progress.draw(target, brush);
                        continue;
                    }
                    Command::Mask(rect, index) => {
                        let geometry = clips.as_ref().unwrap().geometries[*index].clone();
                        let mut parameters = D2D1_LAYER_PARAMETERS {
                            contentBounds: native_rect(*rect),
                            geometricMask: std::mem::ManuallyDrop::new(Some(geometry)),
                            opacity: 1.,
                            ..Default::default()
                        };
                        parameters.maskTransform.M11 = 1.;
                        parameters.maskTransform.M22 = 1.;
                        unsafe {
                            target.PushLayer(&parameters, None);
                            std::mem::ManuallyDrop::drop(&mut parameters.geometricMask);
                        }
                        layers.depth += 1;
                        continue;
                    }
                    Command::Push(rect, opacity) => {
                        let mut parameters = D2D1_LAYER_PARAMETERS {
                            contentBounds: native_rect(*rect),
                            opacity: *opacity,
                            ..Default::default()
                        };
                        parameters.maskTransform.M11 = 1.;
                        parameters.maskTransform.M22 = 1.;
                        unsafe {
                            target.PushLayer(&parameters, None);
                        }
                        layers.depth += 1;
                        continue;
                    }
                    Command::Pop => {
                        debug_assert!(layers.depth > 0);
                        unsafe {
                            target.PopLayer();
                        }
                        layers.depth -= 1;
                        continue;
                    }
                    Command::Background(_, color)
                    | Command::ImagePlaceholder(_, color)
                    | Command::Border(_, color)
                    | Command::Text(_, color) => color.resolve(visual).0,
                    Command::Control(marker) => marker.ink.resolve(visual).0,
                };
                unsafe {
                    brush.SetColor(&D2D1_COLOR_F {
                        r: color[0],
                        g: color[1],
                        b: color[2],
                        a: color[3],
                    });
                }
                match command {
                    Command::Push(_, _)
                    | Command::Image(_)
                    | Command::Mask(_, _)
                    | Command::Pop
                    | Command::Switch(_)
                    | Command::Focus(_)
                    | Command::Progress(_) => {
                        unreachable!("layer commands handled before brush update")
                    }
                    Command::Background(shape, _) => shape.draw(target, brush),
                    Command::Control(marker) => {
                        if let Some(paint) = self.controls.get(&marker.id) {
                            paint.draw(target, brush)?;
                        }
                    }
                    Command::ImagePlaceholder(paint, _) => paint.draw(target, brush),
                    Command::Border(border, _) => border.draw(target, brush)?,
                    Command::Text(index, _) => {
                        if let Some(selection) = selection {
                            selection.draw(&text.items()[*index], *index, target, brush)?;
                        }
                        text.draw_item(*index, target, brush)?;
                    }
                }
            }
            Ok(())
        })();
        unsafe {
            brush.SetColor(&original);
        }
        result
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    if a.empty() {
        return b;
    }
    if b.empty() {
        return a;
    }
    Rect {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}
fn native_rect(rect: Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}
// 文字绘制失败也必须弹出所有已入栈组层，不能污染调用方后续绘制。
struct Layers<'a> {
    target: &'a ID2D1RenderTarget,
    depth: usize,
}
impl Drop for Layers<'_> {
    fn drop(&mut self) {
        while self.depth > 0 {
            unsafe {
                self.target.PopLayer();
            }
            self.depth -= 1;
        }
    }
}

//! child 私有同步度量：复用真实 DirectWrite/八轮重排；每 operation 单缓存且原生工作有硬界。
use crate::{
    surface_javascript_wire as wire,
    surface_native_measure::NativeMeasure,
    surface_viewport::{style, Viewport},
};
use std::{collections::BTreeMap, io, time::Instant};

const MAX_READS: usize = 256;
const MAX_LAYOUTS: usize = 8;
// 仅测试记录固定阶段和累计耗时；不存脚本、节点或快照，不改变产品协议/预算。
macro_rules! mark {
    ($owner:expr, $stage:literal) => {
        #[cfg(test)]
        $owner.mark($stage);
    };
}
struct Cache {
    source: String,
    boxes: BTreeMap<String, [f32; 12]>,
}
#[derive(Default)]
pub(super) struct Geometry {
    viewport: Option<Viewport>,
    deadline: Option<Instant>,
    reads: usize,
    layouts: usize,
    failed: bool,
    cache: Option<Cache>,
    #[cfg(test)]
    started: Option<Instant>,
    #[cfg(test)]
    pub(super) trace: Vec<(&'static str, u128)>,
}
impl Geometry {
    pub(super) fn begin(
        &mut self,
        viewport: Option<Viewport>,
        deadline: Instant,
    ) -> io::Result<()> {
        if let Some(view) = viewport {
            view.validate().map_err(|_| wire::invalid())?;
        }
        *self = Self {
            viewport,
            deadline: Some(deadline),
            #[cfg(test)]
            started: Some(Instant::now()),
            ..Self::default()
        };
        Ok(())
    }
    pub(super) fn valid(&self) -> bool {
        !self.failed && self.deadline.is_some_and(|v| Instant::now() < v)
    }
    // 只有私有宿主交换可扣除等待；不重置布局次数、缓存或失败状态。
    pub(super) fn exclude_wait(&mut self, elapsed: std::time::Duration) {
        self.deadline = self.deadline.and_then(|v| v.checked_add(elapsed));
    }
    pub(super) fn read(&mut self, source: &str, target: &str) -> String {
        match self.read_inner(source, target) {
            Ok(value) if self.valid() => value,
            Ok(_) => {
                mark!(self, "post-read-deadline");
                self.failed = true;
                String::new()
            }
            _ => {
                self.failed = true;
                String::new()
            }
        }
    }
    fn read_inner(&mut self, source: &str, target: &str) -> io::Result<String> {
        mark!(self, "read-admission");
        self.reads += 1;
        if !self.valid()
            || self.reads > MAX_READS
            || source.len() > crate::surface_scene::MAX_BYTES
            || !crate::surface_javascript_event::safe(target)
        {
            return Err(wire::invalid());
        }
        let view = self.viewport.ok_or_else(wire::invalid)?;
        if self
            .cache
            .as_ref()
            .is_none_or(|cache| cache.source != source)
        {
            self.layouts += 1;
            if self.layouts > MAX_LAYOUTS {
                return Err(wire::invalid());
            }
            mark!(self, "scene-parse");
            let scene =
                wire::canvas::scene(&serde_json::from_str(source).map_err(|_| wire::invalid())?)?;
            let context = view.context();
            mark!(self, "native-measure-new");
            let mut measure = NativeMeasure::new(&scene, style(), context, view.logical())
                .map_err(|_| wire::invalid())?;
            mark!(self, "native-prepare");
            let prepared = measure
                .prepare(&scene, view.logical(), context)
                .map_err(|_| wire::invalid())?;
            mark!(self, "native-boxes");
            // 隐藏节点/后代没有 box；仍只允许本次投影内的身份，不把未知 key 当零矩形。
            let mut boxes = BTreeMap::new();
            let mut pending = vec![scene.root()];
            while let Some(node) = pending.pop() {
                boxes.insert(node.id().to_owned(), [0.; 12]);
                pending.extend(node.children());
            }
            for placed in prepared.tree().nodes() {
                let font = measure
                    .resolved_style(placed.node)
                    .ok_or_else(wire::invalid)?
                    .size;
                let local = crate::surface_length::Context { font, ..context };
                let border = crate::surface_box::borders(placed.node, local);
                // 和文字排版共用内容内缩，含按钮默认 padding；百分比以包含块宽度求值。
                let inset =
                    crate::surface_button::insets(placed.node, placed.containing_width, local);
                let content = crate::surface_box::inner(placed.size, inset);
                let padding: [f32; 2] =
                    std::array::from_fn(|a| (inset.start()[a] - border.start()[a]).max(0.));
                let padding_box = crate::surface_box::inner(placed.size, border);
                // 本宿主的 device content box 以两条内容边独立取整，不伪造浏览器的 snapping。
                let pixels: [f32; 2] = std::array::from_fn(|axis| {
                    let start = f64::from(placed.origin[axis] + inset.start()[axis]) * view.ratio();
                    ((start + f64::from(content[axis]) * view.ratio()).round() - start.round())
                        as f32
                });
                boxes.insert(
                    placed.node.id().to_owned(),
                    [
                        placed.origin[0],
                        placed.origin[1],
                        placed.size[0],
                        placed.size[1],
                        padding_box[0],
                        padding_box[1],
                        content[0],
                        content[1],
                        padding[0],
                        padding[1],
                        pixels[0],
                        pixels[1],
                    ],
                );
            }
            self.cache = Some(Cache {
                source: source.to_owned(),
                boxes,
            });
        }
        mark!(self, "target-lookup");
        let value = self
            .cache
            .as_ref()
            .and_then(|c| c.boxes.get(target))
            .ok_or_else(wire::invalid)?;
        serde_json::to_string(value).map_err(|_| wire::invalid())
    }
    #[cfg(test)]
    fn mark(&mut self, stage: &'static str) {
        let item = (stage, self.started.unwrap().elapsed().as_micros());
        if self.trace.len() < 32 {
            self.trace.push(item);
        } else {
            *self.trace.last_mut().unwrap() = item;
        }
    }
}
#[cfg(test)]
#[path = "surface_javascript_geometry_model_tests.rs"]
mod tests;

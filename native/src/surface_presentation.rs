//! 单场景呈现状态；准备成功后原子替换，绘制路径复用已持有的 DirectWrite 布局。
use crate::{
    surface_geometry::Geometry,
    surface_length::Context,
    surface_native_measure::{NativeMeasure, TextFrame},
    surface_paint_frame::PaintFrame,
    surface_scene::Scene,
    surface_text::Style,
};
use windows::core::Result;
/// 仅用于已经完成当前场景来源校验的 UI 像素；按节点 ID 索引，不是跨 revision 缓存。
pub type Images =
    std::collections::BTreeMap<String, std::rc::Rc<crate::surface_image_paint::Bitmap>>;
#[path = "surface_focus_navigation.rs"]
mod focus_navigation;
#[path = "surface_scroll_frame.rs"]
mod scroll_frame;
#[path = "surface_presentation_scroll_state.rs"]
mod scroll_state;
#[path = "surface_scrollbar_presentation.rs"]
mod scrollbar_presentation;
pub use scroll_frame::ControlFont;
use scroll_frame::{Cache, Frame};
#[path = "surface_presentation_selection.rs"]
mod selection;
pub(crate) use selection::Selection;

pub struct Presentation {
    scene: Scene,
    text: TextFrame,
    paint: PaintFrame,
    geometry: Geometry,
    hits: crate::surface_hit_test::Hits,
    size: [f32; 2],
    focusable: Vec<String>,
    navigation: Vec<String>,
    scroll: crate::surface_scroll::Scroll,
    cache: Cache,
    selection: Option<Selection>,
    images: Images,
}
impl Presentation {
    /// 仅改嵌入绘制缓存；不改scene、权威snapshot或正式结果，清除预览时重新读取声明值。
    pub(crate) fn preview_slider(&mut self, id: &str, value: Option<f64>) -> bool {
        let Some(node) = self.scene.root().visible_node(id) else {
            return false;
        };
        let Ok(range) = crate::surface_slider::Range::parse(node) else {
            return false;
        };
        let Ok(fraction) = range.fraction(value.unwrap_or(range.source())) else {
            return false;
        };
        self.paint.preview_slider(id, fraction as f32)
    }
    #[cfg(test)]
    pub(crate) fn embedded_control(
        &self,
        id: &str,
    ) -> Option<&crate::surface_embedded_control::Paint> {
        self.paint.embedded_control(id)
    }
    /// 非交互嵌入宿主不创建EDIT/COMBO子窗口，仍在原有z序/opacity/clip指令流绘制声明值。
    pub(crate) fn embed_controls(&mut self) -> Result<()> {
        self.paint
            .embed_controls(&self.scene, |id| self.cache.control_font(id))
    }
    pub fn edit_colors(&self, id: &str) -> Option<[crate::surface_color::Color; 2]> {
        self.paint.edit_colors(id)
    }
    pub(crate) fn release_device_resources(&self) {
        self.paint.release_device_resources();
    }
    pub fn select_ink(&self, id: &str) -> Option<crate::surface_color::Color> {
        self.paint.select_ink(id)
    }
    pub fn control_font(&self, id: &str) -> Option<&ControlFont> {
        self.cache.control_font(id)
    }
    pub fn textarea_box(&self, id: &str) -> Option<crate::surface_textarea::BoxStyle> {
        self.cache.textarea_box(id)
    }
    pub fn new(scene: Scene, style: Style<'_>, context: Context, size: [f32; 2]) -> Result<Self> {
        Self::with_images(scene, style, context, size, Images::new())
    }
    pub fn with_images(
        scene: Scene,
        style: Style<'_>,
        context: Context,
        size: [f32; 2],
        images: Images,
    ) -> Result<Self> {
        Self::new_scrolled(scene, style, context, size, Default::default(), images)
    }
    fn new_scrolled(
        scene: Scene,
        style: Style<'_>,
        context: Context,
        size: [f32; 2],
        mut scroll: crate::surface_scroll::Scroll,
        images: Images,
    ) -> Result<Self> {
        let (frame, cache) = Self::prepare(&scene, style, context, size, &mut scroll, &images)?;
        let focusable = crate::surface_focus::buttons(&scene, &frame.geometry);
        let navigation = crate::surface_focus::candidates(&scene, &frame.geometry);
        Ok(Self {
            scene,
            text: frame.text,
            paint: frame.paint,
            geometry: frame.geometry,
            hits: frame.hits,
            size,
            focusable,
            navigation,
            scroll,
            cache,
            selection: None,
            images,
        })
    }
    fn prepare(
        scene: &Scene,
        style: Style<'_>,
        context: Context,
        size: [f32; 2],
        scroll: &mut crate::surface_scroll::Scroll,
        images: &Images,
    ) -> Result<(Frame, Cache)> {
        if images.len() > crate::surface_scene::MAX_NODES {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Too many Surface image owners",
            ));
        }
        let mut measure = NativeMeasure::with_image_sizes(
            scene,
            style,
            context,
            size,
            images
                .iter()
                .map(|(id, image)| (id.clone(), image.natural_size()))
                .collect(),
        )?;
        let prepared = measure.prepare(scene, size, context)?;
        let cache = Cache::new(prepared.tree(), prepared.text(), context, |node| {
            measure.resolved_style(node)
        });
        let frame = cache.project_with_images(scene, scroll, images)?;
        Ok((frame, cache))
    }
    /// 场景及其快照一起更新；不保留无界历史，也不把错误候选暴露给绘制方。
    pub fn replace(
        &mut self,
        scene: Scene,
        style: Style<'_>,
        context: Context,
        size: [f32; 2],
    ) -> Result<()> {
        self.replace_with_images(scene, style, context, size, Images::new())
    }
    /// 新场景必须显式提交其资源；绝不按相同节点 ID 自动继承旧图片。
    pub fn replace_with_images(
        &mut self,
        scene: Scene,
        style: Style<'_>,
        context: Context,
        size: [f32; 2],
        images: Images,
    ) -> Result<()> {
        let next = Self::new_scrolled(scene, style, context, size, self.scroll.clone(), images)?;
        *self = next;
        Ok(())
    }
    /// 同一权威场景只更新图片就绪投影；保留文字未变的选区，不把异步图片当作新场景。
    pub(crate) fn refresh_images(
        &mut self,
        scene: Scene,
        style: Style<'_>,
        context: Context,
        size: [f32; 2],
        images: Images,
    ) -> Result<()> {
        let mut next =
            Self::new_scrolled(scene, style, context, size, self.scroll.clone(), images)?;
        self.reconcile_text_selection(&next.text);
        next.selection = self.selection.take();
        next.reset_text_navigation();
        *self = next;
        Ok(())
    }
    /// 尺寸或样式变化才重新测量；调用方必须传入与新视口一致的 Context。
    pub fn reflow(&mut self, style: Style<'_>, context: Context, size: [f32; 2]) -> Result<()> {
        let mut scroll = self.scroll.clone();
        let (frame, cache) =
            Self::prepare(&self.scene, style, context, size, &mut scroll, &self.images)?;
        self.navigation = crate::surface_focus::candidates(&self.scene, &frame.geometry);
        self.install_frame(frame, scroll);
        self.size = size;
        self.cache = cache;
        self.reset_text_navigation();
        Ok(())
    }
    fn install_frame(&mut self, frame: Frame, scroll: crate::surface_scroll::Scroll) {
        self.reconcile_text_selection(&frame.text);
        self.focusable = crate::surface_focus::buttons(&self.scene, &frame.geometry);
        self.text = frame.text;
        self.geometry = frame.geometry;
        self.hits = frame.hits;
        self.paint = frame.paint;
        self.scroll = scroll;
    }
    /// 只重投影已测量快照，成功后提交偏移与整帧，不创建 DirectWrite 布局。
    pub(crate) fn wheel(
        &mut self,
        point: [f32; 2],
        axis: usize,
        delta: f32,
        page: bool,
    ) -> Result<bool> {
        let Some(hit) = self
            .scrollbar_at(point)
            .map(|bar| bar.id)
            .or_else(|| self.hit_test(point).map(str::to_owned))
        else {
            return Ok(false);
        };
        let mut scroll = self.scroll.clone();
        if !scroll.wheel(&self.geometry, &hit, axis, delta, page) {
            return Ok(false);
        }
        let frame = self
            .cache
            .project_with_images(&self.scene, &mut scroll, &self.images)?;
        self.install_frame(frame, scroll);
        Ok(true)
    }
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn focusable(&self, id: &str) -> bool {
        self.focusable.iter().any(|item| item == id)
    }
    pub fn next_focus(&self, current: Option<&str>, reverse: bool) -> Option<&str> {
        crate::surface_focus::next(&self.focusable, current, reverse)
    }
    /// 键盘激活验证当前可聚焦按钮，不借助伪造坐标绕过目标校验。
    pub fn keyboard_events(
        &self,
        id: &str,
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
        modifiers: crate::surface_click_event::Modifiers,
    ) -> std::result::Result<
        Vec<crate::surface_click_event::ClickEvent>,
        crate::surface_click_event::Error,
    > {
        self.keyboard_events_with_values(id, binding, revision, modifiers, &Default::default())
    }
    pub fn keyboard_events_with_values(
        &self,
        id: &str,
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
        modifiers: crate::surface_click_event::Modifiers,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<
        Vec<crate::surface_click_event::ClickEvent>,
        crate::surface_click_event::Error,
    > {
        if !self.focusable(id) {
            return Ok(Vec::new());
        }
        crate::surface_click_route::ClickRoute::new(&self.scene, Some(id))
            .map(|action| action.encode_with_values(binding, revision, modifiers, values))
            .collect()
    }
    pub fn text(&self) -> &TextFrame {
        &self.text
    }
    /// 绘制按场景顺序交织背景与文字；复用画刷并恢复调用方原颜色。
    pub fn draw(
        &self,
        target: &windows::Win32::Graphics::Direct2D::ID2D1RenderTarget,
        brush: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
    ) -> Result<()> {
        self.draw_with_focus(target, brush, None)
    }
    pub(crate) fn draw_with_focus(
        &self,
        target: &windows::Win32::Graphics::Direct2D::ID2D1RenderTarget,
        brush: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
        focus: Option<&str>,
    ) -> Result<()> {
        self.draw_with_visual(
            target,
            brush,
            crate::surface_button_visual::State {
                focus,
                ..Default::default()
            },
        )
    }
    pub(crate) fn draw_with_visual(
        &self,
        target: &windows::Win32::Graphics::Direct2D::ID2D1RenderTarget,
        brush: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
        visual: crate::surface_button_visual::State<'_>,
    ) -> Result<()> {
        self.draw_with_switches(target, brush, visual, |_| None)
    }
    pub(crate) fn draw_with_switches(
        &self,
        target: &windows::Win32::Graphics::Direct2D::ID2D1RenderTarget,
        brush: &windows::Win32::Graphics::Direct2D::ID2D1SolidColorBrush,
        visual: crate::surface_button_visual::State<'_>,
        switch_value: impl Fn(&str) -> Option<bool>,
    ) -> Result<()> {
        self.paint.draw(
            &self.text,
            target,
            brush,
            crate::surface_button_visual::State {
                focus: visual.focus.filter(|id| self.focusable(id)),
                hover: visual.hover.filter(|id| self.focusable(id)),
                pressed: visual.pressed.filter(|id| self.focusable(id)),
            },
            switch_value,
            self.selection.as_ref(),
        )?;
        self.draw_scrollbars(target, brush);
        Ok(())
    }
    pub fn size(&self) -> [f32; 2] {
        self.size
    }
    /// 只替换嵌入文本绘制缓存，场景、布局和正式值仍是权威快照。
    pub(crate) fn preview_text(&mut self, id: &str, value: Option<&str>) -> Result<bool> {
        let font = self.control_font(id).cloned();
        self.paint
            .preview_text(&self.scene, font.as_ref(), id, value)
    }
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
    /// 输入为宿主逻辑坐标；仅返回最上层节点，不代表该节点已授权执行动作。
    pub fn hit_test(&self, point: [f32; 2]) -> Option<&str> {
        if self.scrollbar_at(point).is_some() {
            return None;
        }
        self.hits
            .hit(&self.geometry, point)
            .map(|index| self.geometry.nodes()[index].id())
    }
    pub(crate) fn button_enabled_at(&self, point: [f32; 2]) -> Option<bool> {
        if self.scrollbar_at(point).is_some() {
            return None;
        }
        let index = self.hits.hit(&self.geometry, point)?;
        let node = &self.geometry.nodes()[index];
        matches!(
            node.kind(),
            crate::surface_scene::Kind::Button | crate::surface_scene::Kind::Switch
        )
        .then(|| self.focusable(node.id()))
    }
    /// 仅解析 click 声明；输入 owner 必须先处理选择手势、穿透、授权和生命周期。
    pub fn click_route(&self, point: [f32; 2]) -> crate::surface_click_route::ClickRoute<'_> {
        crate::surface_click_route::ClickRoute::new(&self.scene, self.hit_test(point))
    }
    /// 全批准备成功才返回，避免祖先 payload 超限时仅提交部分冒泡动作。
    pub fn click_events(
        &self,
        point: [f32; 2],
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
        modifiers: crate::surface_click_event::Modifiers,
    ) -> std::result::Result<
        Vec<crate::surface_click_event::ClickEvent>,
        crate::surface_click_event::Error,
    > {
        self.click_events_with_values(point, binding, revision, modifiers, &Default::default())
    }
    pub fn click_events_with_values(
        &self,
        point: [f32; 2],
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
        modifiers: crate::surface_click_event::Modifiers,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<
        Vec<crate::surface_click_event::ClickEvent>,
        crate::surface_click_event::Error,
    > {
        let mut total = 0;
        let mut events = Vec::new();
        for action in self.click_route(point) {
            let event = action.encode_with_values(binding, revision, modifiers, values)?;
            total += event.bytes().len();
            if total > crate::surface_click_event::MAX_EVENT_BYTES {
                return Err(crate::surface_click_event::Error::Encoding);
            }
            events.push(event);
        }
        Ok(events)
    }
}

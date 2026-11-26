//! Single-scene presentation state; replace atomically after successful preparation and reuse owned DirectWrite layouts during painting.
use crate::{
    surface_geometry::Geometry,
    surface_length::Context,
    surface_native_measure::{NativeMeasure, TextFrame},
    surface_paint_frame::PaintFrame,
    surface_scene::Scene,
    surface_text::Style,
};
use windows::core::Result;
/// Only for UI pixels already validated against current-scene sources; indexed by node ID, not a cross-revision cache.
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
    /// Change only embedded paint caches, not scene, authoritative snapshot, or committed results; reread declared values when clearing previews.
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
    /// Noninteractive embedded hosts create no EDIT/COMBO child windows, but paint declared values in the original z-order/opacity/clip command stream.
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
        let (focusable, navigation) = crate::surface_focus::lists(&scene, &frame.geometry);
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
    /// Update scenes and snapshots together; retain no unbounded history and expose no invalid candidates to painters.
    pub fn replace(
        &mut self,
        scene: Scene,
        style: Style<'_>,
        context: Context,
        size: [f32; 2],
    ) -> Result<()> {
        self.replace_with_images(scene, style, context, size, Images::new())
    }
    /// New scenes must explicitly submit their resources; never inherit old images automatically by matching node IDs.
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
    /// Update only image-readiness projection for the same authoritative scene; preserve selections when text is unchanged and do not treat async images as new scenes.
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
    /// Remeasure only when dimensions or styles change; callers must supply a Context matching the new viewport.
    pub fn reflow(&mut self, style: Style<'_>, context: Context, size: [f32; 2]) -> Result<()> {
        let mut scroll = self.scroll.clone();
        let (frame, cache) =
            Self::prepare(&self.scene, style, context, size, &mut scroll, &self.images)?;
        let (focusable, navigation) = crate::surface_focus::lists(&self.scene, &frame.geometry);
        self.navigation = navigation;
        self.install_frame_with_focus(frame, scroll, focusable);
        self.size = size;
        self.cache = cache;
        self.reset_text_navigation();
        Ok(())
    }
    fn install_frame(&mut self, frame: Frame, scroll: crate::surface_scroll::Scroll) {
        let focusable = crate::surface_focus::buttons(&self.scene, &frame.geometry);
        self.install_frame_with_focus(frame, scroll, focusable);
    }
    fn install_frame_with_focus(
        &mut self,
        frame: Frame,
        scroll: crate::surface_scroll::Scroll,
        focusable: Vec<String>,
    ) {
        self.reconcile_text_selection(&frame.text);
        self.focusable = focusable;
        self.text = frame.text;
        self.geometry = frame.geometry;
        self.hits = frame.hits;
        self.paint = frame.paint;
        self.scroll = scroll;
    }
    /// Reproject only measured snapshots, committing offsets and complete frames on success without creating DirectWrite layouts.
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
    pub(crate) fn image_bitmap(
        &self,
        id: &str,
    ) -> Option<&std::rc::Rc<crate::surface_image_paint::Bitmap>> {
        self.images.get(id)
    }
    pub fn focusable(&self, id: &str) -> bool {
        self.focusable.iter().any(|item| item == id)
    }
    pub fn next_focus(&self, current: Option<&str>, reverse: bool) -> Option<&str> {
        crate::surface_focus::next(&self.focusable, current, reverse)
    }
    /// Keyboard activation validates the current focusable button without fabricated coordinates that bypass target checks.
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
    /// Painting interleaves backgrounds and text in scene order, reusing brushes and restoring the caller's original colors.
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
    /// Replace only embedded text paint caches; scene, layout, and committed values remain the authoritative snapshot.
    pub(crate) fn preview_text(&mut self, id: &str, value: Option<&str>) -> Result<bool> {
        let font = self.control_font(id).cloned();
        self.paint
            .preview_text(&self.scene, font.as_ref(), id, value)
    }
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
    /// Input uses host logical coordinates; return only the topmost node without implying authorization to execute its actions.
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
    /// Parse click declarations only; input owners must first handle selection gestures, pass-through, authorization, and lifecycle.
    pub fn click_route(&self, point: [f32; 2]) -> crate::surface_click_route::ClickRoute<'_> {
        crate::surface_click_route::ClickRoute::new(&self.scene, self.hit_test(point))
    }
    /// Return only after preparing the entire batch, preventing partial bubbling-action submission when ancestor payloads exceed limits.
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

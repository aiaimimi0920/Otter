//! 原 Canvas 内的扩展视图；只准备声明式 Surface，不创建覆盖层 HWND 或独立呈现层。
use super::canvas::Canvas;
use super::{display_frame::Frame, extension_document::Source};
use crate::{
    extension_visual::{Bounds, Visual},
    surface_presentation::Presentation,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use windows::{
    core::{Interface, Result},
    Win32::{
        Foundation::*,
        Graphics::Direct2D::{Common::*, *},
        Graphics::Gdi::InvalidateRect,
    },
};
pub(super) struct Views {
    items: Vec<(Visual, RefCell<Presentation>)>,
    target: Cell<usize>,
    valid: Cell<bool>,
    press: RefCell<Option<input::Press>>,
    regions: RefCell<Vec<crate::surface_clip::Rect>>,
    size: Cell<[f32; 2]>,
}
#[path = "extension_visual_input.rs"]
mod input;
pub(super) use input::Outcome;
pub(super) struct Bound {
    source: Source,
    views: Rc<Views>,
}
impl Views {
    pub(super) fn prepare(visuals: Vec<Visual>, unit_size: [f32; 2]) -> Result<Self> {
        let mut items = Vec::new();
        for visual in visuals {
            let bytes = serde_json::to_vec(&visual.scene)
                .map_err(|_| windows::core::Error::new(E_INVALIDARG, "Invalid extension scene"))?;
            let scene = crate::surface_scene::Scene::parse(&bytes)
                .map_err(|_| windows::core::Error::new(E_INVALIDARG, "Invalid extension scene"))?;
            let bounds = visual.bounds.resolve(unit_size);
            let size = [bounds[2].max(1.), bounds[3].max(1.)];
            let mut presentation = Presentation::new(
                scene,
                crate::surface_viewport::style(),
                crate::surface_viewport::context(size),
                size,
            )?;
            presentation.embed_controls()?;
            items.push((visual, RefCell::new(presentation)));
        }
        let mut views = Self {
            items,
            target: Cell::new(0),
            valid: Cell::new(true),
            press: RefCell::new(None),
            regions: RefCell::new(Vec::new()),
            size: Cell::new(unit_size),
        };
        *views.regions.get_mut() = views.build_regions();
        Ok(views)
    }
    fn resize(&self, unit_size: [f32; 2]) -> Result<()> {
        if self.size.get() == unit_size
            || self
                .items
                .iter()
                .all(|(v, _)| matches!(v.bounds, Bounds::Fixed(_)))
        {
            return Ok(());
        }
        // 仅 Unit 尺寸的主干 fallback/占位重排，声明式固定场景不重新解析或测量。
        for (v, p) in &self.items {
            if matches!(v.bounds, Bounds::UnitInset(_)) {
                let bounds = v.bounds.resolve(unit_size);
                let size = [bounds[2].max(1.), bounds[3].max(1.)];
                let mut p = p.borrow_mut();
                p.reflow(
                    crate::surface_viewport::style(),
                    crate::surface_viewport::context(size),
                    size,
                )?;
                p.embed_controls()?;
            }
        }
        self.size.set(unit_size);
        *self.regions.borrow_mut() = self.build_regions();
        Ok(())
    }
    pub(super) fn bind(self, canvas: &Canvas, frame: Option<&Frame>) -> Bound {
        Bound {
            source: Source::capture(canvas, frame),
            views: Rc::new(self),
        }
    }
}
impl Bound {
    pub(super) fn resize(
        &self,
        canvas: &Canvas,
        frame: Option<&Frame>,
        size: [f32; 2],
    ) -> Result<()> {
        if self.views.valid.get() && self.source.matches(&canvas.editor, frame) {
            self.views.resize(size)?;
        }
        Ok(())
    }
    pub(super) fn shared(&self) -> Rc<Views> {
        self.views.clone()
    }
    pub(super) fn views(&self, canvas: &Canvas, frame: Option<&Frame>) -> Option<Rc<Views>> {
        (self.views.valid.get() && self.source.matches(&canvas.editor, frame))
            .then(|| self.views.clone())
    }
    pub(super) fn rebind(
        &self,
        source: &Canvas,
        copy: &Canvas,
        frame: Option<&Frame>,
    ) -> Option<Self> {
        self.source.matches(&source.editor, frame).then(|| Self {
            source: Source::capture(copy, frame),
            views: self.views.clone(),
        })
    }
    pub(super) fn active(&self, canvas: &Canvas, frame: Option<&Frame>, translation: bool) -> bool {
        let plugin = if translation {
            crate::extension_visual::TRANSLATE
        } else {
            crate::extension_visual::OCR
        };
        self.views.valid.get()
            && self.source.matches(&canvas.editor, frame)
            && self
                .views
                .items
                .iter()
                .any(|(v, _)| !v.placeholder && v.plugin == plugin)
    }
    pub(super) unsafe fn draw(
        &self,
        editor: &crate::editor::Editor,
        frame: Option<&Frame>,
        target: &ID2D1RenderTarget,
        brush: &ID2D1SolidColorBrush,
        viewport: (D2D_SIZE_U, u32),
    ) -> Result<()> {
        if !self.views.valid.get() || !self.source.matches(editor, frame) {
            return Ok(());
        }
        let (size, dpi) = viewport;
        if self.views.target.replace(target.as_raw() as usize) != target.as_raw() as usize {
            for (_, p) in &self.views.items {
                p.borrow().release_device_resources();
            }
        }
        let unit = D2D_RECT_F {
            left: 0.,
            top: 0.,
            right: size.width as f32 * 96. / dpi as f32,
            bottom: size.height as f32 * 96. / dpi as f32,
        };
        let _unit = super::render_frame::Clip::push(target, &unit);
        for (visual, presentation) in &self.views.items {
            let [x, y, w, h] = visual.bounds.resolve(self.views.size.get());
            let mut transform = Default::default();
            target.GetTransform(&mut transform);
            let saved = transform;
            transform.M31 += x * transform.M11 + y * transform.M21;
            transform.M32 += x * transform.M12 + y * transform.M22;
            target.SetTransform(&transform);
            let result = {
                let _clip = super::render_frame::Clip::push(
                    target,
                    &D2D_RECT_F {
                        left: 0.,
                        top: 0.,
                        right: w,
                        bottom: h,
                    },
                );
                presentation.borrow().draw(target, brush)
            };
            target.SetTransform(&saved);
            result?;
        }
        Ok(())
    }
}
pub(super) unsafe fn invalidate(hwnd: HWND) {
    let _ = InvalidateRect(Some(hwnd), None, false);
}
pub(super) fn clear(canvas: &mut Canvas) -> bool {
    if let Some(views) = canvas.extension_views.take() {
        views.views.valid.set(false);
        return views.views.cancel(true);
    }
    false
}

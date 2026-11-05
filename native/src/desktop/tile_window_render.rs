//! Tile 复用产品 D3D12→D3D11→CPU 选择/关联准入；只在完整 present 后签发回执。
use crate::desktop::render_surface::RenderSurface;
use crate::{tile_shared::Frame, tile_wire::Size};
use std::collections::BTreeMap;
#[path = "tile_surface_render.rs"]
mod art;
#[path = "tile_identification_render.rs"]
pub(super) mod identification;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{Common::*, *},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        },
    },
};
pub(super) struct Renderer {
    surface: RenderSurface,
    bitmaps: BTreeMap<String, ID2D1Bitmap>,
    _uploads: Option<crate::byte_budget::Lease>,
    frame: Option<std::sync::Arc<Frame>>,
    arts: BTreeMap<String, art::Prepared>,
    scrolls: art::scroll::Memory,
    identification: Option<identification::Painter>,
    texts: std::rc::Rc<Vec<crate::tile_surface_text::Preview>>,
    applied_texts: std::rc::Rc<Vec<crate::tile_surface_text::Preview>>,
    focus: Option<(crate::tile_surface_focus::Focus, bool)>,
    locals: std::rc::Rc<Vec<crate::tile_surface_local::Entry>>,
    applied_locals: std::rc::Rc<Vec<crate::tile_surface_local::Entry>>,
}
impl Renderer {
    pub(super) fn scrollbar(
        &self,
        frame: &Frame,
        pixel: [i32; 2],
    ) -> Option<crate::tile_surface_scrollbar::Field> {
        art::scroll::scrollbar(&self.arts, frame, pixel)
    }
    pub(super) fn current_bar(
        &self,
        frame: &Frame,
        field: &crate::tile_surface_scrollbar::Field,
    ) -> Option<crate::tile_surface_scrollbar::Field> {
        art::scroll::current_bar(&self.arts, frame, field)
    }
    pub(super) fn plan_bar(
        &self,
        frame: &Frame,
        field: &crate::tile_surface_scrollbar::Field,
        value: f32,
    ) -> Option<crate::surface_scroll::Scroll> {
        art::scroll::plan_bar(&self.arts, frame, field, value)
    }
    pub(super) fn synchronize_scroll(&mut self, state: &crate::tile_shared::State) {
        self.scrolls
            .synchronize(self.frame.as_deref(), &self.arts, state);
    }
    pub(super) fn retained_focus(
        &self,
        frame: &Frame,
        field: &crate::tile_surface_focus::Focus,
    ) -> Option<crate::tile_surface_focus::Focus> {
        art::scroll::retained_focus(&self.arts, frame, field)
    }
    pub(super) fn retained_text(
        &self,
        frame: &Frame,
        field: &crate::tile_surface_text::Field,
    ) -> Option<crate::tile_surface_text::Field> {
        art::scroll::retained_text(&self.arts, frame, field)
    }
    pub(super) fn planned_wheel(
        &self,
        frame: &Frame,
        pixel: [i32; 2],
        axis: usize,
        delta: f32,
        page: bool,
    ) -> Option<(String, crate::surface_scroll::Scroll)> {
        art::scroll::wheel(&self.arts, frame, pixel, axis, delta, page)
    }
    pub(super) fn install_scroll(
        &mut self,
        frame: &Frame,
        id: &str,
        scroll: &crate::surface_scroll::Scroll,
    ) -> Result<bool> {
        art::scroll::install(&mut self.arts, frame, id, scroll)
    }
    pub(super) fn matches_scroll(
        &self,
        frame: &Frame,
        id: &str,
        scroll: &crate::surface_scroll::Scroll,
    ) -> bool {
        art::scroll::matches(&self.arts, frame, id, scroll)
    }
    pub(super) fn local_values(
        &mut self,
        values: std::rc::Rc<Vec<crate::tile_surface_local::Entry>>,
    ) {
        self.locals = values;
    }
    pub(super) fn control_font(
        &self,
        frame: &Frame,
        focus: &crate::tile_surface_focus::Focus,
    ) -> Option<crate::surface_presentation::ControlFont> {
        art::focus::font(&self.arts, frame, focus)
    }
    pub(super) fn focus_at(
        &self,
        frame: &Frame,
        pixel: [i32; 2],
    ) -> Option<crate::tile_surface_focus::Focus> {
        art::focus::at(&self.arts, frame, pixel)
    }
    pub(super) fn next_focus(
        &mut self,
        frame: &Frame,
        current: Option<&crate::tile_surface_focus::Focus>,
        reverse: bool,
    ) -> Result<Option<crate::tile_surface_focus::Focus>> {
        art::focus::next(&mut self.arts, frame, current, reverse)
    }
    pub(super) fn focus(&mut self, focus: Option<(crate::tile_surface_focus::Focus, bool)>) {
        self.focus = focus;
    }
    pub(super) fn text(
        &self,
        frame: &Frame,
        pixel: [i32; 2],
    ) -> Option<crate::tile_surface_text::Field> {
        art::text(&self.arts, frame, pixel)
    }
    pub(super) fn text_previews(
        &mut self,
        texts: std::rc::Rc<Vec<crate::tile_surface_text::Preview>>,
    ) {
        self.texts = texts;
    }
    pub(super) fn slider(
        &self,
        frame: &Frame,
        pixel: [i32; 2],
    ) -> Option<crate::tile_surface_slider::Field> {
        art::slider(&self.arts, frame, pixel)
    }
    pub(super) fn value(
        &self,
        frame: &Frame,
        pixel: [i32; 2],
        modifiers: crate::surface_click_event::Modifiers,
    ) -> Option<crate::tile_surface_value::Field> {
        art::value(&self.arts, frame, pixel, modifiers)
    }
    pub(super) fn click(
        &self,
        frame: &Frame,
        pixel: [i32; 2],
        modifiers: crate::surface_click_event::Modifiers,
    ) -> Result<Vec<crate::tile_surface_actions::Command>> {
        art::click(&self.arts, frame, pixel, modifiers)
    }
    pub fn new(hwnd: HWND, size: Size) -> Result<Self> {
        let factory: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
        #[cfg(test)]
        if let Some(surface) = super::test_access::private_surface(&factory, size)? {
            return Ok(Self::from_surface(surface));
        }
        let surface = RenderSurface::new(
            &factory,
            hwnd,
            D2D_SIZE_U {
                width: size.width,
                height: size.height,
            },
            96,
            false,
        )
        .map_err(|e| e.into_error())?;
        Ok(Self::from_surface(surface))
    }
    fn from_surface(surface: RenderSurface) -> Self {
        Self {
            surface,
            bitmaps: BTreeMap::new(),
            _uploads: None,
            frame: None,
            arts: BTreeMap::new(),
            scrolls: Default::default(),
            identification: None,
            texts: Default::default(),
            applied_texts: Default::default(),
            focus: None,
            locals: Default::default(),
            applied_locals: Default::default(),
        }
    }
    pub fn paint(
        &mut self,
        frame: Option<&std::sync::Arc<Frame>>,
        marker: Option<&crate::tile_identification::Marker>,
        hwnd: HWND,
        slider: Option<&crate::tile_surface_slider::Drag>,
    ) -> Result<bool> {
        if marker.is_some() && self.identification.is_none() {
            self.identification = Some(identification::Painter::new()?);
        }
        if marker.is_none() {
            self.identification = None;
        }
        let changed = match (&self.frame, frame) {
            (Some(a), Some(b)) => !std::sync::Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        if changed {
            art::prepare(&mut self.arts, frame.map(|f| f.as_ref()))?;
            self.scrolls
                .restore(frame.map(|f| f.as_ref()), &mut self.arts)?;
            self.bitmaps.clear();
            self._uploads = None;
            self.frame = None;
            if let Some(f) = frame {
                let bytes = f.images.values().map(|i| i.pixels.len()).sum();
                self._uploads = Some(
                    crate::pixels::reserve(bytes)
                        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Tile bitmap upload budget"))?,
                );
                for (id, image) in &f.images {
                    let bitmap = unsafe {
                        self.surface.resource_target().CreateBitmap(
                            D2D_SIZE_U {
                                width: image.width,
                                height: image.height,
                            },
                            Some(image.pixels.as_ptr().cast()),
                            image.width * 4,
                            &D2D1_BITMAP_PROPERTIES {
                                pixelFormat: D2D1_PIXEL_FORMAT {
                                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                                },
                                dpiX: 96.,
                                dpiY: 96.,
                            },
                        )?
                    };
                    self.bitmaps.insert(id.clone(), bitmap);
                }
                self.frame = Some(f.clone());
            }
        }
        if changed || !std::rc::Rc::ptr_eq(&self.texts, &self.applied_texts) {
            art::texts(&mut self.arts, &self.texts)?;
            self.applied_texts = self.texts.clone();
        }
        if changed || !std::rc::Rc::ptr_eq(&self.locals, &self.applied_locals) {
            art::local::apply(&mut self.arts, frame.map(|f| f.as_ref()), &self.locals)?;
            self.applied_locals = self.locals.clone();
        }
        art::preview(&mut self.arts, frame.map(|f| f.as_ref()), slider);
        let bitmaps = &self.bitmaps;
        let arts = &self.arts;
        let identification = &self.identification;
        let completed = self.surface.draw(|target| unsafe {
            paint(
                target,
                frame.map(|f| f.as_ref()),
                bitmaps,
                arts,
                self.focus.as_ref(),
            )?;
            if let Some(marker) = marker {
                identification
                    .as_ref()
                    .unwrap()
                    .draw(target, marker, std::time::Instant::now())?;
            }
            Ok(())
        })?;
        if let Some(submission) = self.surface.take_submission() {
            submission.present(hwnd)?;
            return Ok(true);
        }
        Ok(completed)
    }
}
unsafe fn paint(
    target: &ID2D1RenderTarget,
    frame: Option<&Frame>,
    bitmaps: &BTreeMap<String, ID2D1Bitmap>,
    arts: &BTreeMap<String, art::Prepared>,
    focus: Option<&(crate::tile_surface_focus::Focus, bool)>,
) -> Result<()> {
    let mut identity = Default::default();
    target.GetTransform(&mut identity);
    identity.M11 = 1.;
    identity.M12 = 0.;
    identity.M21 = 0.;
    identity.M22 = 1.;
    identity.M31 = 0.;
    identity.M32 = 0.;
    target.SetTransform(&identity);
    target.Clear(Some(&D2D1_COLOR_F {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 1.,
    }));
    let Some(f) = frame else {
        return Ok(());
    };
    let tile = f
        .layout
        .tiles
        .iter()
        .find(|t| t.endpoint_id == f.key.endpoint)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Tile missing"))?;
    let (width, height) = (f.size.width as f32, f.size.height as f32);
    let mut placements: Vec<_> = f.layout.placements.iter().collect();
    placements.sort_by(|a, b| {
        a.z_index
            .cmp(&b.z_index)
            .then(a.placement_id.cmp(&b.placement_id))
    });
    if placements.is_empty() {
        // 空墙测试图仍使用墙坐标；相邻输出共享网格，不把每屏各画一份局部网格。
        let brush = target.CreateSolidColorBrush(
            &D2D1_COLOR_F {
                r: 0.25,
                g: 0.7,
                b: 0.45,
                a: 1.,
            },
            None,
        )?;
        let bounds = f.layout.bounds;
        let point = |p| {
            let [x, y] = crate::tile_geometry::native(tile, p);
            D2D_POINT_2F {
                x: x as f32 * width,
                y: y as f32 * height,
            }
        };
        for i in 0..=16 {
            let x = bounds.x + bounds.width * i as f64 / 16.;
            let y = bounds.y + bounds.height * i as f64 / 16.;
            target.DrawLine(
                point([x, bounds.y]),
                point([x, bounds.y + bounds.height]),
                &brush,
                1.,
                None,
            );
            target.DrawLine(
                point([bounds.x, y]),
                point([bounds.x + bounds.width, y]),
                &brush,
                1.,
                None,
            );
        }
        target.DrawLine(
            point([bounds.x, bounds.y]),
            point([bounds.x + bounds.width, bounds.y + bounds.height]),
            &brush,
            2.,
            None,
        );
        return Ok(());
    }
    for p in placements {
        let Some(projection) = crate::tile_geometry::project(tile, p) else {
            continue;
        };
        let [q0, q1, q3] = projection.quad;
        let mut transform = identity;
        transform.M11 = (q1[0] - q0[0]) as f32 * width;
        transform.M12 = (q1[1] - q0[1]) as f32 * height;
        transform.M21 = (q3[0] - q0[0]) as f32 * width;
        transform.M22 = (q3[1] - q0[1]) as f32 * height;
        transform.M31 = q0[0] as f32 * width;
        transform.M32 = q0[1] as f32 * height;
        target.SetTransform(&transform);
        let crop = projection.crop;
        if p.source.kind == "surface" {
            let prepared = arts
                .get(&p.source.id)
                .ok_or_else(|| Error::new(E_INVALIDARG, "Tile Art missing"))?;
            let focused = focus.filter(|(f, _)| {
                f.anchor.placement == p.placement_id
                    && f.anchor.view == prepared.owner.view
                    && std::sync::Arc::ptr_eq(&f.anchor.snapshot, &prepared.owner.snapshot)
            });
            let visual = crate::surface_button_visual::State {
                focus: focused.map(|(f, _)| f.node.as_str()),
                pressed: focused
                    .filter(|(_, pressed)| *pressed)
                    .map(|(f, _)| f.node.as_str()),
                ..Default::default()
            };
            art::draw(prepared, target, crop, visual)?;
            continue;
        }
        let image = f
            .images
            .get(&p.source.id)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Tile image missing"))?;
        let bitmap = bitmaps
            .get(&p.source.id)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Tile bitmap missing"))?;
        target.DrawBitmap(
            bitmap,
            Some(&D2D_RECT_F {
                left: 0.,
                top: 0.,
                right: 1.,
                bottom: 1.,
            }),
            1.,
            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
            Some(&D2D_RECT_F {
                left: crop.x as f32 * image.width as f32,
                top: crop.y as f32 * image.height as f32,
                right: (crop.x + crop.width) as f32 * image.width as f32,
                bottom: (crop.y + crop.height) as f32 * image.height as f32,
            }),
        );
    }
    target.SetTransform(&identity);
    Ok(())
}
#[cfg(test)]
#[path = "tile_surface_pixel_tests.rs"]
mod art_tests;
#[cfg(test)]
#[path = "tile_surface_local_pixel_tests.rs"]
mod local_tests;
#[cfg(test)]
#[path = "tile_presented_pixel_tests.rs"]
mod presented_tests;
#[cfg(test)]
#[path = "tile_pixel_tests.rs"]
mod tests;

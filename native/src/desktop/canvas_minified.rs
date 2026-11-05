//! 恢复缩略窗口的独立取景呈现；共享原图，不把 contain 留白再次当成原图缩放。
use super::*;
use crate::{
    crop_geometry::Size,
    session_minified::{View, Views},
};
use windows::{core::Error, Win32::Foundation::E_INVALIDARG};

pub(in crate::desktop) struct Presentation {
    image: Option<Image>,
    views: Views,
    size: Size,
    stamp: crate::editor::DocumentStamp,
}

impl Presentation {
    pub(in crate::desktop) fn for_display(&self, image: &Image, editor: &Editor) -> Result<Self> {
        let annotations = self.views.annotations;
        let image_view =
            if let Some(frame) = editor.annotation_frame().or_else(|| editor.visible_frame()) {
                let sx = annotations.size.width / frame.width;
                let sy = annotations.size.height / frame.height;
                View {
                    size: Size {
                        width: image.width as f64 * sx,
                        height: image.height as f64 * sy,
                    },
                    offset: (
                        frame.x * sx + annotations.offset.0,
                        frame.y * sy + annotations.offset.1,
                    ),
                }
            } else {
                let scale = (annotations.size.width / image.width as f64)
                    .min(annotations.size.height / image.height as f64);
                let size = Size {
                    width: image.width as f64 * scale,
                    height: image.height as f64 * scale,
                };
                View {
                    size,
                    offset: (
                        (annotations.offset.0 - (annotations.size.width - size.width) / 2.0)
                            .clamp(0.0, (size.width - self.size.width).max(0.0)),
                        (annotations.offset.1 - (annotations.size.height - size.height) / 2.0)
                            .clamp(0.0, (size.height - self.size.height).max(0.0)),
                    ),
                }
            };
        Self::restored(
            Some(image.clone()),
            Views {
                image: image_view,
                annotations,
            },
            self.size,
            editor,
        )
    }
    pub(in crate::desktop) fn geometry(&self) -> (Views, Size) {
        (self.views, self.size)
    }
    pub(in crate::desktop) fn image(&self) -> Option<&Image> {
        self.image.as_ref()
    }
    pub(in crate::desktop) fn matches(&self, editor: &Editor) -> bool {
        self.stamp.matches(editor) && editor.draft().is_none()
    }
    pub(in crate::desktop) fn restored(
        image: Option<Image>,
        views: Views,
        size: Size,
        editor: &Editor,
    ) -> Result<Self> {
        for view in [
            views.image,
            views.annotations,
            View {
                size,
                offset: (0.0, 0.0),
            },
        ] {
            if ![
                view.size.width,
                view.size.height,
                view.offset.0,
                view.offset.1,
            ]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
                || view.size.width <= 0.0
                || view.size.height <= 0.0
            {
                return Err(Error::new(E_INVALIDARG, "Invalid minified view"));
            }
        }
        Ok(Self {
            image,
            views,
            size,
            stamp: editor.document_stamp(),
        })
    }

    // 仅绘制已提交文档；后续编辑/切换入口必须重新准备呈现，不能沿用旧原图。
    unsafe fn paint(
        &self,
        target: &ID2D1RenderTarget,
        resources: &mut raster::Resources,
        brush: &ID2D1SolidColorBrush,
        effects: &mut super::super::effect_cache::Cache,
        editor: &Editor,
        output: (D2D_SIZE_U, u32),
    ) -> Result<()> {
        let (size, dpi) = output;
        if !self.matches(editor) || dpi == 0 {
            return Err(Error::new(E_INVALIDARG, "Stale minified presentation"));
        }
        resources.sync_sized(
            target,
            self.image.as_ref(),
            editor
                .layers()
                .and_then(|layers| layers.annotations.as_ref()),
            editor
                .layers()
                .map_or((0, 0), |layers| layers.annotation_dimensions()),
        )?;
        let sx = size.width as f64 * 96.0 / dpi as f64 / self.size.width;
        let sy = size.height as f64 * 96.0 / dpi as f64 / self.size.height;
        let mapping = self
            .views
            .annotations
            .document_mapping(self.editor_frame(editor))
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid minified document mapping"))?;
        // 物理框四舍五入可能造成两轴比例略有差别；不要累计修正文档坐标。
        let mut original = Default::default();
        target.GetTransform(&mut original);
        let _restore = super::super::render_frame::save_transform(target);
        let mut transform = original;
        transform.M11 = sx as f32;
        transform.M22 = sy as f32;
        transform.M12 = 0.0;
        transform.M21 = 0.0;
        transform.M31 = 0.0;
        transform.M32 = 0.0;
        target.SetDpi(dpi as f32, dpi as f32);
        target.Clear(Some(&D2D1_COLOR_F::default()));
        target.SetTransform(&transform);
        let clip = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: self.size.width as f32,
            bottom: self.size.height as f32,
        };
        let _clipped = super::super::render_frame::Clip::push(target, &clip);
        resources.draw(
            target,
            &destination(self.views.image),
            &destination(self.views.annotations),
        );
        transform.M11 = (sx * mapping.scale.0) as f32;
        transform.M22 = (sy * mapping.scale.1) as f32;
        transform.M31 = (sx * mapping.offset.0) as f32;
        transform.M32 = (sy * mapping.offset.1) as f32;
        target.SetTransform(&transform);
        super::super::annotations::draw_committed(
            target,
            brush,
            editor,
            Viewport {
                origin: Point::default(),
                scale: 1.0,
            },
            effects,
        )
    }
    // WIC 像素测试仍自有一个事务；真实窗口改由 RenderSurface 独占事务，不嵌套 BeginDraw。
    #[cfg(test)]
    unsafe fn draw(
        &self,
        target: &ID2D1RenderTarget,
        resources: &mut raster::Resources,
        brush: &ID2D1SolidColorBrush,
        effects: &mut super::super::effect_cache::Cache,
        editor: &Editor,
        output: (D2D_SIZE_U, u32),
    ) -> Result<()> {
        super::super::render_frame::draw(target, || {
            self.paint(target, resources, brush, effects, editor, output)
        })
    }
    fn editor_frame(&self, editor: &Editor) -> Option<crate::crop_geometry::Rect> {
        editor.annotation_frame().or_else(|| editor.visible_frame())
    }
}

fn destination(view: View) -> D2D_RECT_F {
    D2D_RECT_F {
        left: -view.offset.0 as f32,
        top: -view.offset.1 as f32,
        right: (view.size.width - view.offset.0) as f32,
        bottom: (view.size.height - view.offset.1) as f32,
    }
}

impl Canvas {
    pub(super) unsafe fn draw_minified_display(
        &mut self,
        frame: &super::super::display_frame::Frame,
        size: D2D_SIZE_U,
        dpi: u32,
    ) -> Result<()> {
        let empty = Editor::default();
        let editor = frame.editor.as_deref().unwrap_or(&empty);
        let presentation = self
            .minified
            .as_ref()
            .expect("checked by paint")
            .for_display(&frame.image, editor)?;
        let resources = self.target.as_mut().expect("paint owns target creation");
        resources.surface.resize(size)?;
        resources.surface.draw(|target| {
            presentation.paint(
                target,
                &mut resources.raster,
                &resources.brush,
                &mut resources.effects,
                editor,
                (size, dpi),
            )
        })?;
        self.paints = self.paints.saturating_add(1);
        Ok(())
    }
    pub(super) unsafe fn draw_minified(&mut self, size: D2D_SIZE_U, dpi: u32) -> Result<()> {
        let presentation = self.minified.as_ref().expect("checked by paint");
        let resources = self.target.as_mut().expect("paint owns target creation");
        resources.surface.resize(size)?;
        resources.surface.draw(|target| {
            presentation.paint(
                target,
                &mut resources.raster,
                &resources.brush,
                &mut resources.effects,
                &self.editor,
                (size, dpi),
            )
        })?;
        self.paints = self.paints.saturating_add(1);
        Ok(())
    }
}

#[cfg(test)]
#[path = "canvas_minified_crop_tests.rs"]
mod crop_tests;
#[cfg(test)]
#[path = "canvas_minified_tests.rs"]
mod tests;

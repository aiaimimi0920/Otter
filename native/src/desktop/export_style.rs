//! 输出快照显式携带 node-owned 内容样式；不塞进 Editor，不修改或重复烘焙可编辑像素。
use super::*;
use crate::{crop_geometry::Rect, pin_frame_style::Style};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::desktop) struct Snapshot {
    pub style: Style,
    logical: (f64, f64),
}
impl Snapshot {
    pub(in crate::desktop) fn capture(c: &crate::desktop::canvas::Canvas) -> Self {
        if c.frame_style == Style::default() {
            return Self::default();
        }
        let logical = if let Some(mini) = &c.minified {
            let size = mini.geometry().0.annotations.size;
            (size.width, size.height)
        } else if let Some(size) = c.client_size {
            (f64::from(size.x), f64::from(size.y))
        } else if let Some(image) = &c.image {
            (
                f64::from(image.width) * f64::from(c.viewport.scale),
                f64::from(image.height) * f64::from(c.viewport.scale),
            )
        } else {
            (0.0, 0.0)
        };
        Self {
            style: c.frame_style,
            logical,
        }
    }
    fn rect(self) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: self.logical.0,
            height: self.logical.1,
        }
    }
    pub(super) unsafe fn transform(&self, target: &ID2D1RenderTarget, output: (u32, u32)) {
        let mut m = Default::default();
        target.GetTransform(&mut m);
        m.M11 = output.0 as f32 / self.logical.0 as f32;
        m.M22 = output.1 as f32 / self.logical.1 as f32;
        m.M12 = 0.0;
        m.M21 = 0.0;
        m.M31 = 0.0;
        m.M32 = 0.0;
        target.SetTransform(&m);
    }
    pub(super) unsafe fn clip<'a>(
        &self,
        target: &'a ID2D1RenderTarget,
        factory: &ID2D1Factory,
        output: (u32, u32),
        reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<Option<source_resolution::frame_clip::Clip<'a>>> {
        source_resolution::frame_clip::push_frame(
            target,
            factory,
            self.rect(),
            self.style.radius(),
            output,
            reserve,
        )
    }
    pub(super) unsafe fn border(&self, target: &ID2D1RenderTarget) -> Result<()> {
        if let Some(border) = source_resolution::frame_border::Border::from_style(self.style) {
            border.draw(target, self.rect())?;
        }
        Ok(())
    }
    pub(in crate::desktop) fn compose(
        &self,
        image: &Image,
        editor: &Editor,
        wic: &IWICImagingFactory,
        opacity: f32,
        reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
    ) -> Result<ComposedImage> {
        if self.style == Style::default() {
            return compose_opacity(image, editor, wic, opacity, reserve);
        }
        if ![self.logical.0, self.logical.1]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0 && *v <= 1_000_000.0)
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid frame style output geometry",
            ));
        }
        compose_frame(image, editor, wic, opacity, reserve, None, Some(self))
    }
    pub(in crate::desktop) fn clipboard(
        self,
        image: &Image,
        editor: &Editor,
        opacity: f32,
    ) -> Result<super::super::clipboard_bitmap::Payload> {
        let _apartment = Apartment::new()?;
        let wic: IWICImagingFactory =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
        let bitmap = self.compose(image, editor, &wic, opacity, crate::pixels::reserve)?;
        let (stream, size) = encode_bitmap(&wic, &bitmap.bitmap)?;
        let dibv5 = super::super::clipboard_bitmap::encode(&wic, &bitmap.bitmap)?;
        drop(bitmap);
        Ok(super::super::clipboard_bitmap::Payload {
            png: stream_pixels(stream, size)?,
            dibv5,
        })
    }
    pub(in crate::desktop) fn png(
        self,
        image: &Image,
        editor: &Editor,
        opacity: f32,
    ) -> Result<(crate::pixels::Pixels, crate::desktop::drag_preview::Preview)> {
        let _apartment = Apartment::new()?;
        let wic: IWICImagingFactory =
            unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
        let bitmap = self.compose(image, editor, &wic, opacity, crate::pixels::reserve)?;
        let preview = crate::desktop::drag_preview::prepare(&wic, &bitmap.bitmap)?;
        let (stream, size) = encode_bitmap(&wic, &bitmap.bitmap)?;
        drop(bitmap);
        Ok((stream_pixels(stream, size)?, preview))
    }
    pub(in crate::desktop) fn save_new(
        self,
        path: &Path,
        image: &Image,
        editor: &Editor,
        opacity: f32,
    ) -> std::result::Result<(), String> {
        save_allocated(image, editor, opacity, Some(&self), || {
            crate::file_naming::allocate::create_exact(path).map(|file| (file, path.to_path_buf()))
        })
        .map(|_| ())
    }
    pub(in crate::desktop) fn save_incremented(
        self,
        directory: &Path,
        stem: &str,
        image: &Image,
        editor: &Editor,
        opacity: f32,
    ) -> std::result::Result<std::path::PathBuf, String> {
        save_allocated(image, editor, opacity, Some(&self), || {
            crate::file_naming::allocate::create_unique_file(
                directory,
                stem,
                Some("png"),
                &std::sync::atomic::AtomicBool::new(false),
            )
        })
    }
}

#[cfg(test)]
#[path = "export_style_tests.rs"]
mod tests;

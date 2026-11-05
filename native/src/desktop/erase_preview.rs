//! 实时擦除只改变栅格合成遮罩；不生成 PNG，不重算模糊，不触碰矢量标注。
use crate::{erase_job::Request, viewport::Viewport};
use std::sync::Arc;
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::E_INVALIDARG,
        Graphics::Direct2D::{Common::*, *},
    },
};

#[derive(PartialEq)]
struct Key {
    length: usize,
    width: f32,
    viewport: Viewport,
    bounds: [f32; 4],
}
#[derive(Default)]
pub(super) struct Cache {
    entry: Option<(Arc<()>, Key, ID2D1Geometry)>,
}
impl Cache {
    pub(super) fn clear(&mut self) {
        self.entry = None;
    }
    pub(super) unsafe fn geometry(
        &mut self,
        target: &ID2D1RenderTarget,
        bounds: &D2D_RECT_F,
        request: &Request,
        viewport: Viewport,
    ) -> Result<ID2D1Geometry> {
        let key = Key {
            length: request.points.len(),
            width: request.width,
            viewport,
            bounds: [bounds.left, bounds.top, bounds.right, bounds.bottom],
        };
        // 宿主在同一身份下只追加点；新手势必须分配新身份，不能原地改写旧路径。
        if let Some((identity, stored, geometry)) = &self.entry {
            if Arc::ptr_eq(identity, &request.preview_identity) && *stored == key {
                return Ok(geometry.clone());
            }
        }
        self.entry = None;
        let geometry = remaining(target, bounds, request, viewport)?;
        self.entry = Some((request.preview_identity.clone(), key, geometry.clone()));
        Ok(geometry)
    }
}

pub(super) unsafe fn remaining(
    target: &ID2D1RenderTarget,
    bounds: &D2D_RECT_F,
    request: &Request,
    viewport: Viewport,
) -> Result<ID2D1Geometry> {
    if request.points.is_empty() {
        return Err(Error::new(E_INVALIDARG, "Empty erase preview"));
    }
    let erased =
        super::annotation_mosaic::mask_points(target, &request.points, request.width, viewport)?;
    let factory = target.GetFactory()?;
    let image = factory.CreateRectangleGeometry(bounds)?;
    let remaining = factory.CreatePathGeometry()?;
    let sink = remaining.Open()?;
    image.CombineWithGeometry(&erased, D2D1_COMBINE_MODE_EXCLUDE, None, 0.25, &sink)?;
    sink.Close()?;
    remaining.cast()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        erase_layers::{Layers, Mode},
        viewport::Point,
    };
    use windows::Win32::{Graphics::Imaging::*, System::Com::*};
    #[test]
    fn cache_reuses_only_same_gesture_geometry_and_does_not_retain_source_pixels() {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        unsafe {
            let wic: IWICImagingFactory =
                CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
            let output = wic
                .CreateBitmap(32, 32, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
                .unwrap();
            let factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            let target = factory
                .CreateWicBitmapRenderTarget(&output, &D2D1_RENDER_TARGET_PROPERTIES::default())
                .unwrap();
            let mut request = Request {
                preview_identity: Arc::new(()),
                annotation_frame: None,
                base_projection: None,
                source: Layers {
                    annotation_size: None,
                    size: (32, 32),
                    base: vec![255; 32 * 32 * 4].into(),
                    annotations: None,
                },
                revision: 0,
                mode: Mode::Content,
                points: vec![Point { x: 8.0, y: 8.0 }],
                width: 8.0,
            };
            let mut bounds = D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: 32.0,
                bottom: 32.0,
            };
            let mut viewport = Viewport {
                origin: Point::default(),
                scale: 1.0,
            };
            let mut cache = Cache::default();
            let first = cache
                .geometry(&target, &bounds, &request, viewport)
                .unwrap();
            assert_eq!(
                first.as_raw(),
                cache
                    .geometry(&target, &bounds, &request.clone(), viewport)
                    .unwrap()
                    .as_raw()
            );
            request.points.push(Point { x: 16.0, y: 16.0 });
            let appended = cache
                .geometry(&target, &bounds, &request, viewport)
                .unwrap();
            assert_ne!(first.as_raw(), appended.as_raw());
            viewport.scale = 1.25;
            let scaled = cache
                .geometry(&target, &bounds, &request, viewport)
                .unwrap();
            assert_ne!(appended.as_raw(), scaled.as_raw());
            request.width = 12.0;
            let wider = cache
                .geometry(&target, &bounds, &request, viewport)
                .unwrap();
            assert_ne!(scaled.as_raw(), wider.as_raw());
            bounds.right = 30.0;
            let resized = cache
                .geometry(&target, &bounds, &request, viewport)
                .unwrap();
            assert_ne!(wider.as_raw(), resized.as_raw());
            request.preview_identity = Arc::new(());
            let renewed = cache
                .geometry(&target, &bounds, &request, viewport)
                .unwrap();
            assert_ne!(resized.as_raw(), renewed.as_raw());
            let pixels = request.source.base.downgrade();
            let identity = Arc::downgrade(&request.preview_identity);
            drop(request);
            assert!(pixels.upgrade().is_none());
            assert!(identity.upgrade().is_some());
            cache.clear();
            assert!(identity.upgrade().is_none());
        }
    }
}

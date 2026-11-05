//! 圆角子树裁剪几何；按 render target 的工厂惰性缓存，禁止跨工厂复用 COM 几何。
use std::cell::{Ref, RefCell};
use windows::{
    core::{Interface, Result},
    Win32::Graphics::Direct2D::*,
};
#[path = "surface_round_shape.rs"]
mod shape;
pub(crate) use shape::Shape;
pub(crate) struct Clips {
    shapes: Vec<Shape>,
    cache: RefCell<Option<Cache>>,
}

pub(crate) struct Cache {
    factory: ID2D1Factory,
    pub(crate) geometries: Vec<ID2D1Geometry>,
}
impl Clips {
    pub(crate) fn new(shapes: Vec<Shape>) -> Self {
        Self {
            shapes,
            cache: RefCell::new(None),
        }
    }
    pub(crate) fn prepare(&self, target: &ID2D1RenderTarget) -> Result<Option<Ref<'_, Cache>>> {
        if self.shapes.is_empty() {
            return Ok(None);
        }
        let factory = unsafe { target.GetFactory()? };
        let same = self
            .cache
            .borrow()
            .as_ref()
            .is_some_and(|cache| cache.factory.as_raw() == factory.as_raw());
        if !same {
            // 整批成功后才换缓存；失败不会留下半份几何或继续使用旧工厂对象。
            let geometries = self
                .shapes
                .iter()
                .map(|shape| shape.geometry(&factory))
                .collect::<Result<Vec<ID2D1Geometry>>>()?;
            *self.cache.borrow_mut() = Some(Cache {
                factory,
                geometries,
            });
        }
        Ok(Some(Ref::map(self.cache.borrow(), |cache| {
            cache.as_ref().unwrap()
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Graphics::{Direct2D::Common::*, Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM};
    fn target(factory: &ID2D1Factory) -> ID2D1DCRenderTarget {
        unsafe {
            factory
                .CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    ..Default::default()
                })
                .unwrap()
        }
    }
    #[test]
    fn rounded_geometry_is_reused_for_same_factory_and_rebuilt_for_another() {
        let first: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }.unwrap();
        let second: ID2D1Factory =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }.unwrap();
        let first_target = target(&first);
        let second_target = target(&second);
        let clips = Clips::new(vec![D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: 0.,
                top: 0.,
                right: 64.,
                bottom: 64.,
            },
            radiusX: 16.,
            radiusY: 16.,
        }
        .into()]);
        let first_pointer = clips.prepare(&first_target).unwrap().unwrap().geometries[0].as_raw();
        let same_pointer = clips.prepare(&first_target).unwrap().unwrap().geometries[0].as_raw();
        assert_eq!(first_pointer, same_pointer);
        let next = clips.prepare(&second_target).unwrap().unwrap();
        assert_ne!(next.geometries[0].as_raw(), first_pointer);
        assert_eq!(next.factory.as_raw(), second.as_raw());
    }
}

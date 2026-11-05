//! UI 线程的图片绘制源：像素共享，位图按精确 target 缓存，上传预算跨窗口共用。
use crate::{
    image_decode::Image, surface_clip::Rect, surface_layout_tree::Placed, surface_length::Context,
};
#[path = "surface_image_placeholder_paint.rs"]
pub(crate) mod placeholder;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, LazyLock},
};
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::{E_INVALIDARG, E_OUTOFMEMORY},
        Graphics::{
            Direct2D::{Common::*, *},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        },
    },
};
// 这是上传像素字节预算，不冒充驱动/纹理实际总显存测量。
static UPLOAD: LazyLock<Arc<crate::byte_budget::Budget>> =
    LazyLock::new(|| crate::byte_budget::Budget::new(128 * 1024 * 1024));
struct Cached {
    target: ID2D1RenderTarget,
    bitmap: ID2D1Bitmap,
    _budget: crate::byte_budget::Lease,
}
/// 已验证 PBGRA 像素的 UI 线程资源；不负责来源授权，不能替代 Ready 的当前挂载校验。
pub struct Bitmap {
    image: Image,
    logical_size: [f32; 2],
    cache: RefCell<Option<Cached>>,
}
impl Bitmap {
    #[cfg(test)]
    pub(crate) fn uploaded_for_test(&self) -> bool {
        self.cache.borrow().is_some()
    }
    pub(crate) fn natural_size(&self) -> [f32; 2] {
        self.logical_size
    }
    pub fn new(image: Image) -> Result<Rc<Self>> {
        let size = [image.width as f32, image.height as f32];
        Self::with_logical_size(image, size)
    }
    /// 当前栅格每逻辑单位一个像素，尾部 padding 不参与布局或 contain 比例。
    /// 这是 Surface 私有元数据；不改变截图、Relay 或通用 Image 的像素语义。
    pub(crate) fn with_logical_size(image: Image, logical_size: [f32; 2]) -> Result<Rc<Self>> {
        if logical_size
            .iter()
            .zip([image.width, image.height])
            .any(|(v, raster)| {
                !v.is_finite()
                    || *v <= 0.0
                    || !(32768. / *v).is_finite()
                    || *v > raster as f32
                    || v.ceil() as u32 != raster
            })
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid Surface image logical size",
            ));
        }
        if crate::viewport::checked_image_bytes(image.width, image.height)
            .filter(|n| *n <= 64 * 1024 * 1024)
            != Some(image.pixels.len())
        {
            return Err(Error::new(E_INVALIDARG, "Invalid Surface bitmap pixels"));
        }
        Ok(Rc::new(Self {
            image,
            logical_size,
            cache: RefCell::new(None),
        }))
    }
    pub(crate) fn clear(&self) {
        self.cache.borrow_mut().take();
    }
    fn bitmap(&self, target: &ID2D1RenderTarget) -> Result<ID2D1Bitmap> {
        let mut cache = self.cache.borrow_mut();
        if let Some(cached) = cache
            .as_ref()
            .filter(|cached| cached.target.as_raw() == target.as_raw())
        {
            return Ok(cached.bitmap.clone());
        }
        // 设备重建不可沿用同工厂的旧 target 位图；先释放旧租约，避免双份占用阻止恢复。
        cache.take();
        let budget = UPLOAD
            .reserve(self.image.pixels.len())
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Surface bitmap upload budget exhausted"))?;
        let bitmap = unsafe {
            target.CreateBitmap(
                D2D_SIZE_U {
                    width: self.image.width,
                    height: self.image.height,
                },
                Some(self.image.pixels.as_ptr().cast()),
                self.image.width * 4,
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
        *cache = Some(Cached {
            target: target.clone(),
            bitmap: bitmap.clone(),
            _budget: budget,
        });
        Ok(bitmap)
    }
}
pub(crate) struct Paint {
    source: Rc<Bitmap>,
    destination: D2D_RECT_F,
    clip: D2D_RECT_F,
}
impl Paint {
    pub(crate) fn new(
        placed: &Placed<'_>,
        clip: Rect,
        context: Context,
        source: Rc<Bitmap>,
    ) -> Option<Self> {
        let inset = crate::surface_box::inset(placed.node, placed.containing_width, context);
        let size = crate::surface_box::inner(placed.size, inset);
        if size[0] == 0. || size[1] == 0. {
            return None;
        }
        let origin = std::array::from_fn(|a| placed.origin[a] + inset.start()[a]);
        let clip = clip.intersect(Rect::from_box(origin, size));
        if clip.empty() {
            return None;
        }
        let natural = source.logical_size;
        let scale = (size[0] / natural[0]).min(size[1] / natural[1]);
        // Canvas 的缺省 object-fit 是 fill；普通声明式图片仍保留原 contain 行为。
        let fitted = if placed.node.canvas_size().is_some() {
            size
        } else {
            natural.map(|v| v * scale)
        };
        let origin = [
            origin[0] + (size[0] - fitted[0]) / 2.,
            origin[1] + (size[1] - fitted[1]) / 2.,
        ];
        Some(Self {
            source,
            destination: native(Rect::from_box(origin, fitted)),
            clip: native(clip),
        })
    }
    pub(crate) fn clear(&self) {
        self.source.clear();
    }
    pub(crate) fn draw(&self, target: &ID2D1RenderTarget) -> Result<()> {
        let bitmap = self.source.bitmap(target)?;
        unsafe {
            target.PushAxisAlignedClip(&self.clip, D2D1_ANTIALIAS_MODE_ALIASED);
            target.DrawBitmap(
                &bitmap,
                Some(&self.destination),
                1.,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                Some(&D2D_RECT_F {
                    left: 0.,
                    top: 0.,
                    right: self.source.logical_size[0],
                    bottom: self.source.logical_size[1],
                }),
            );
            target.PopAxisAlignedClip();
        }
        Ok(())
    }
}
fn native(rect: Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}
#[cfg(test)]
#[path = "surface_image_paint_tests.rs"]
mod tests;

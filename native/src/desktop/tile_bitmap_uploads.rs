//! Tile bitmap ownership is scoped to one render target; each upload holds its own budget lease.
use crate::{byte_budget::Lease, image_decode::Image, pixels::Storage};
use std::{collections::BTreeMap, sync::Weak};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::E_OUTOFMEMORY,
        Graphics::{
            Direct2D::{Common::*, *},
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        },
    },
};

struct Upload {
    bitmap: ID2D1Bitmap,
    dimensions: (u32, u32),
    pixels: Weak<Storage>,
    _lease: Lease,
}

#[derive(Default)]
pub(super) struct Cache(BTreeMap<String, Upload>);
impl Cache {
    pub(super) fn get(&self, id: &str) -> Option<&ID2D1Bitmap> {
        self.0.get(id).map(|upload| &upload.bitmap)
    }

    pub(super) fn sync(
        &mut self,
        target: &ID2D1RenderTarget,
        next: Option<&BTreeMap<String, Image>>,
        reserve: impl Fn(usize) -> Option<Lease>,
    ) -> Result<()> {
        let Some(next) = next else {
            self.0.clear();
            return Ok(());
        };
        // Release removed/replaced resources before requesting budget for new ones.
        self.0.retain(|id, upload| {
            next.get(id).is_some_and(|image| {
                upload.dimensions == (image.width, image.height)
                    && upload.pixels.ptr_eq(&image.pixels.downgrade())
            })
        });
        for (id, image) in next {
            if self.0.contains_key(id) {
                continue;
            }
            let lease = reserve(image.pixels.len())
                .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Tile bitmap upload budget"))?;
            let bitmap = unsafe {
                target.CreateBitmap(
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
            self.0.insert(
                id.clone(),
                Upload {
                    bitmap,
                    dimensions: (image.width, image.height),
                    pixels: image.pixels.downgrade(),
                    _lease: lease,
                },
            );
        }
        Ok(())
    }
}

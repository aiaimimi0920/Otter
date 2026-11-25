//! Immutable input images in the child's current session and per-target bitmaps all consume the original total logical Canvas budget.
//! Read/release original PNGs individually; decoding stays outside the JS heap, with no URL cache or ownership of networking, files, or windows.
use crate::{
    byte_budget::{Budget, Lease},
    image_decode::Image,
    surface_javascript_wire::{self as wire, images, invalid},
};
use std::{collections::BTreeMap, io, sync::Arc};
use windows::Win32::Graphics::{
    Direct2D::{Common::*, *},
    Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
};
struct Input {
    image: Image,
    header: images::Header,
    _lease: Lease,
}
#[derive(Default)]
pub(super) struct Inputs(BTreeMap<String, Input>);
pub(super) struct Bitmap {
    bitmap: ID2D1Bitmap,
    _lease: Lease,
}
pub(super) type Bitmaps = BTreeMap<String, Bitmap>;
impl Inputs {
    pub(super) fn install(
        &mut self,
        headers: &[images::Header],
        reader: &mut impl io::Read,
        budget: &Arc<Budget>,
    ) -> io::Result<()> {
        images::validate(headers)?;
        let mut combined: Vec<_> = self.0.values().map(|input| input.header.clone()).collect();
        combined.extend_from_slice(headers);
        // Input and service results share cumulative admission limits of 8 images / 8MiB PNG / 16MiB pixels; failures never change the old table.
        images::validate(&combined)?;
        let mut pending = BTreeMap::new();
        for header in headers {
            let lease = budget
                .reserve(wire::canvas::bytes(header.size)?)
                .ok_or_else(invalid)?;
            let png = images::read(reader, header)?;
            // Recheck declared dimensions before WIC pixel allocation; PNG is not arbitrary media open to unrestricted sniffing.
            let image = Image::from_projection_png(&png, header.size[0], header.size[1], &|| false)
                .map_err(|_| invalid())?;
            pending.insert(
                header.id.clone(),
                Input {
                    image,
                    header: header.clone(),
                    _lease: lease,
                },
            );
        }
        self.0.extend(pending);
        Ok(())
    }
    pub(super) fn size(&self, id: &str) -> Option<[u32; 2]> {
        self.0
            .get(id)
            .map(|input| [input.image.width, input.image.height])
    }
    pub(super) fn prepare(
        &self,
        target: &ID2D1RenderTarget,
        bitmaps: &mut Bitmaps,
        command: &super::commands::Draw,
        budget: &Arc<Budget>,
    ) -> io::Result<()> {
        let super::commands::Draw::Image {
            id,
            source,
            destination,
            ..
        } = command
        else {
            return Err(invalid());
        };
        let input = self.0.get(id).ok_or_else(invalid)?;
        // Verify identity first; zero-area/fully out-of-bounds regions are no-ops and must not allocate potentially large target bitmaps.
        if region(
            [input.image.width, input.image.height],
            *source,
            *destination,
        )
        .is_none()
        {
            return Ok(());
        }
        if bitmaps.contains_key(id) {
            return Ok(());
        }
        let image = &input.image;
        let lease = budget.reserve(image.pixels.len()).ok_or_else(invalid)?;
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
            )
        }
        .map_err(|_| invalid())?;
        bitmaps.insert(
            id.into(),
            Bitmap {
                bitmap,
                _lease: lease,
            },
        );
        Ok(())
    }
}
pub(super) fn draw(
    target: &ID2D1RenderTarget,
    images: &Bitmaps,
    command: &super::commands::Draw,
) -> windows::core::Result<()> {
    let super::commands::Draw::Image {
        id,
        source,
        destination,
        transform,
        alpha,
        smooth,
    } = command
    else {
        return Err(windows::core::Error::from(
            windows::Win32::Foundation::E_INVALIDARG,
        ));
    };
    // Renderer has already prepared the entire batch and verified every image identity; missing cache entries can only be empty regions requiring no drawing.
    let Some(image) = images.get(id) else {
        return Ok(());
    };
    let bitmap = &image.bitmap;
    let size = unsafe { bitmap.GetPixelSize() };
    let Some((src, dst)) = region([size.width, size.height], *source, *destination) else {
        return Ok(());
    };
    unsafe {
        let mut matrix = Default::default();
        target.GetTransform(&mut matrix);
        [
            matrix.M11, matrix.M12, matrix.M21, matrix.M22, matrix.M31, matrix.M32,
        ] = *transform;
        target.SetTransform(&matrix);
        target.DrawBitmap(
            bitmap,
            Some(&dst),
            *alpha,
            if *smooth {
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR
            } else {
                D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR
            },
            Some(&src),
        );
    }
    Ok(())
}
fn region(
    size: [u32; 2],
    source: [f32; 4],
    destination: [f32; 4],
) -> Option<(D2D_RECT_F, D2D_RECT_F)> {
    // Negative drawImage widths/heights extend rectangles without flipping; clip the destination proportionally when the source exceeds bounds.
    let normalize = |[mut x, mut y, mut w, mut h]: [f32; 4]| {
        if w < 0. {
            x += w;
            w = -w;
        }
        if h < 0. {
            y += h;
            h = -h;
        }
        [x, y, w, h]
    };
    let [sx, sy, sw, sh] = normalize(source);
    let [dx, dy, dw, dh] = normalize(destination);
    if sw == 0. || sh == 0. || dw == 0. || dh == 0. {
        return None;
    }
    let (left, top, right, bottom) = (
        sx.max(0.),
        sy.max(0.),
        (sx + sw).min(size[0] as f32),
        (sy + sh).min(size[1] as f32),
    );
    if left >= right || top >= bottom {
        return None;
    }
    let src = D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    };
    let dst = D2D_RECT_F {
        left: dx + (left - sx) / sw * dw,
        top: dy + (top - sy) / sh * dh,
        right: dx + (right - sx) / sw * dw,
        bottom: dy + (bottom - sy) / sh * dh,
    };
    Some((src, dst))
}

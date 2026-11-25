//! One-shot SDR readback: retain only the selected, tightly packed, opaque BGRA pixels.
use super::*;
use crate::{capture_color::SdrFrame, pixels::Pixels};

pub unsafe fn region(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    area: Region,
    cancel: &AtomicBool,
) -> Result<SdrFrame, Error> {
    read_mapped(
        context,
        source,
        Some(area),
        Format::Bgra8,
        cancel,
        crate::pixels::reserve,
        |data, width, height, stride| {
            copy(data, width, height, stride, cancel, crate::pixels::reserve)
        },
    )
}

fn copy(
    data: &[u8],
    width: u32,
    height: u32,
    stride: usize,
    cancel: &AtomicBool,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<SdrFrame, Error> {
    check(cancel)?;
    // read_mapped already established a non-empty crop, stride and mapped length.
    let row_bytes = width as usize * 4;
    let length = row_bytes * height as usize;
    let mut lease = reserve(length).ok_or(Error::Budget)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(|_| Error::Budget)?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::Budget);
    }
    for row in data.chunks_exact(stride) {
        check(cancel)?;
        let start = bytes.len();
        bytes.extend_from_slice(&row[..row_bytes]);
        for alpha in bytes[start + 3..].iter_mut().step_by(4) {
            *alpha = 255;
        }
    }
    check(cancel)?;
    Ok(SdrFrame {
        width,
        height,
        pixels: Pixels::from_reserved(bytes, lease),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_rows_preserve_sdr_color_and_normalize_alpha_without_retaining_padding() {
        let budget = crate::byte_budget::Budget::new(8);
        let data = [1, 2, 3, 0, 9, 9, 9, 9, 4, 5, 6, 128, 9, 9, 9, 9];
        let cancel = AtomicBool::new(false);
        let output = copy(&data, 1, 2, 8, &cancel, |n| budget.reserve(n)).unwrap();
        assert_eq!((output.width, output.height), (1, 2));
        assert_eq!(&*output.pixels, &[1, 2, 3, 255, 4, 5, 6, 255]);
        assert_eq!(budget.used(), 8);
        drop(output);
        assert!(matches!(
            copy(&data, 1, 2, 8, &cancel, |n| {
                cancel.store(true, Ordering::Release);
                budget.reserve(n)
            }),
            Err(Error::Cancelled)
        ));
        assert_eq!(budget.used(), 0);
    }
}

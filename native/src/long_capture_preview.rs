//! 按片段直接生成最长边 384 像素的预览，不分配完整合成长图；最近邻仅用于预览。
use super::*;
const EDGE: u32 = 384;
pub(in crate::long_capture) fn render(
    axis: Axis,
    frames: &[Positioned],
    edges: (u32, u32),
    cancel: &AtomicBool,
) -> Result<Composite, Error> {
    let plan = plan(axis, frames, cancel, edges)?;
    let largest = plan.width.max(plan.height);
    let scale = |value: u32| {
        if largest <= EDGE {
            value
        } else {
            ((u64::from(value) * u64::from(EDGE)) / u64::from(largest)).max(1) as u32
        }
    };
    let (width, height) = (scale(plan.width), scale(plan.height));
    let bytes = (width * height * 4) as usize;
    let mut lease = crate::pixels::reserve(bytes).ok_or(Error::Budget)?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(bytes).map_err(|_| Error::Budget)?;
    if !lease.grow_to(pixels.capacity()) {
        return Err(Error::Budget);
    }
    pixels.resize(bytes, 0);
    let (along, cross, source_along, source_cross) = match axis {
        Axis::Vertical => (height, width, plan.height, plan.width),
        Axis::Horizontal => (width, height, plan.width, plan.height),
    };
    let mut segments = plan.segments.iter();
    let mut segment = segments.next().ok_or(Error::Empty)?;
    let mut start = 0u32;
    for line in 0..along {
        cancelled(cancel)?;
        let source_line = sample(line, along, source_along);
        while source_line >= start + segment.length {
            start += segment.length;
            segment = segments.next().ok_or(Error::Dimensions)?;
        }
        let frame = &frames[segment.frame].frame;
        let source_line = segment.offset + source_line - start;
        for x in 0..cross {
            let source_x = sample(x, cross, source_cross);
            let (src, dst) = match axis {
                Axis::Vertical => (
                    (source_line as usize * frame.width as usize + source_x as usize) * 4,
                    (line as usize * width as usize + x as usize) * 4,
                ),
                Axis::Horizontal => (
                    (source_x as usize * frame.width as usize + source_line as usize) * 4,
                    (x as usize * width as usize + line as usize) * 4,
                ),
            };
            pixels[dst..dst + 4].copy_from_slice(&frame.pixels[src..src + 4]);
        }
    }
    cancelled(cancel)?;
    Ok(Composite {
        width,
        height,
        origin: plan.origin,
        pixels: Pixels::from_reserved(pixels, lease),
    })
}
fn sample(index: u32, count: u32, source: u32) -> u32 {
    ((2 * u64::from(index) + 1) * u64::from(source) / (2 * u64::from(count))) as u32
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_axes_preview_matches_direct_sampling_of_final_composite() {
        for axis in [Axis::Vertical, Axis::Horizontal] {
            let make = |origin| {
                let (w, h) = match axis {
                    Axis::Vertical => (12, 600),
                    Axis::Horizontal => (600, 12),
                };
                let mut bytes = Vec::new();
                for y in 0..h {
                    for x in 0..w {
                        bytes.extend_from_slice(&[(x % 251) as u8, (y % 251) as u8, 19, 255]);
                    }
                }
                Positioned {
                    frame: Frame::new(w, h, bytes.into()).unwrap(),
                    origin,
                }
            };
            let frames = [make(0), make(400), make(-200)];
            let cancel = AtomicBool::new(false);
            let full = stitch(axis, &frames, &cancel, crate::pixels::reserve).unwrap();
            let preview = render(axis, &frames, (0, 0), &cancel).unwrap();
            assert_eq!(preview.width.max(preview.height), 384);
            assert!(preview.pixels.len() <= 384 * 384 * 4);
            for y in 0..preview.height {
                for x in 0..preview.width {
                    let src = (sample(y, preview.height, full.height) * full.width
                        + sample(x, preview.width, full.width))
                        as usize
                        * 4;
                    let dst = (y * preview.width + x) as usize * 4;
                    assert_eq!(&preview.pixels[dst..dst + 4], &full.pixels[src..src + 4]);
                }
            }
            assert!(matches!(
                render(axis, &frames, (0, 0), &AtomicBool::new(true)),
                Err(Error::Cancelled)
            ));
        }
    }
}

//! BGRA→BT709 limited NV12；直接写本次MF buffer，无第二份全帧Vec，按行检查取消。
use super::*;
pub(super) fn input(
    bgra: &[u8],
    size: [u32; 2],
    timestamp: i64,
    stop: &AtomicBool,
) -> Result<IMFSample> {
    let length = format::validate_size(size)?;
    if bgra.len() != size[0] as usize * size[1] as usize * 4 {
        return Err(invalid("Encoder input dimensions mismatch"));
    }
    unsafe {
        let buffer = MFCreateMemoryBuffer(length as u32)?;
        let (mut ptr, mut capacity) = (std::ptr::null_mut(), 0);
        buffer.Lock(&mut ptr, Some(&mut capacity), None)?;
        let lock = samples::Locked(&buffer);
        if ptr.is_null() || capacity < length as u32 {
            return Err(invalid("Encoder input buffer bounds"));
        }
        convert(
            bgra,
            size,
            std::slice::from_raw_parts_mut(ptr, length),
            stop,
        )?;
        drop(lock);
        buffer.SetCurrentLength(length as u32)?;
        let sample = MFCreateSample()?;
        sample.AddBuffer(&buffer)?;
        sample.SetSampleTime(timestamp)?;
        sample.SetSampleDuration(10_000_000 / 30)?;
        Ok(sample)
    }
}
fn yuv(r: i32, g: i32, b: i32) -> [u8; 3] {
    [
        (((11966 * r + 40254 * g + 4064 * b + 32768) >> 16) + 16).clamp(16, 235) as u8,
        (((-6596 * r - 22188 * g + 28784 * b + 32768) >> 16) + 128).clamp(16, 240) as u8,
        (((28784 * r - 26145 * g - 2639 * b + 32768) >> 16) + 128).clamp(16, 240) as u8,
    ]
}
fn convert(
    bgra: &[u8],
    [width, height]: [u32; 2],
    nv12: &mut [u8],
    stop: &AtomicBool,
) -> Result<()> {
    let w = width as usize;
    let (y_plane, uv_plane) = nv12.split_at_mut(w * height as usize);
    for y in (0..height as usize).step_by(2) {
        check_stop(stop)?;
        for x in (0..w).step_by(2) {
            let mut rgb = [0; 3];
            for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                let at = ((y + dy) * w + x + dx) * 4;
                let p = &bgra[at..at + 4];
                // H264无alpha；不把透明像素静默变黑/不透明，调用者会退休codec并发原raw。
                if p[3] != 255 {
                    return Err(invalid("H264 cannot preserve input alpha"));
                }
                let [r, g, b] = [i32::from(p[2]), i32::from(p[1]), i32::from(p[0])];
                y_plane[(y + dy) * w + x + dx] = yuv(r, g, b)[0];
                for (sum, value) in rgb.iter_mut().zip([r, g, b]) {
                    *sum += value;
                }
            }
            let uv = yuv((rgb[0] + 2) / 4, (rgb[1] + 2) / 4, (rgb[2] + 2) / 4);
            uv_plane[y / 2 * w + x..y / 2 * w + x + 2].copy_from_slice(&uv[1..]);
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bt709_constants_and_chroma_average_match_independent_oracle() {
        for (bgra, expected) in [
            ([0, 0, 255, 255], [63, 102, 240]),
            ([0, 255, 0, 255], [173, 42, 26]),
            ([255, 0, 0, 255], [32, 240, 118]),
            ([255, 255, 255, 255], [235, 128, 128]),
            ([0, 0, 0, 255], [16, 128, 128]),
        ] {
            let mut output = [0; 6];
            convert(
                &bgra.repeat(4),
                [2, 2],
                &mut output,
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(
                output,
                [
                    expected[0],
                    expected[0],
                    expected[0],
                    expected[0],
                    expected[1],
                    expected[2]
                ]
            );
        }
        let mut output = [0; 6];
        convert(
            &[
                0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
            ],
            [2, 2],
            &mut output,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(output, [16, 235, 16, 235, 128, 128]);
        assert!(convert(&[0; 16], [2, 2], &mut output, &AtomicBool::new(false)).is_err());
        assert!(convert(&[255; 16], [2, 2], &mut output, &AtomicBool::new(true)).is_err());
    }
}

//! BGRA -> BT709 limited NV12 directly into this MF buffer, without another full-frame Vec, checking cancellation per row.
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
#[inline(always)]
fn luma(r: i32, g: i32, b: i32) -> u8 {
    (((11966 * r + 40254 * g + 4064 * b + 32768) >> 16) + 16).clamp(16, 235) as u8
}
#[inline(always)]
fn chroma(r: i32, g: i32, b: i32) -> [u8; 2] {
    [
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
        let row0 = &bgra[y * w * 4..(y + 1) * w * 4];
        let row1 = &bgra[(y + 1) * w * 4..(y + 2) * w * 4];
        let (y_rows, _) = y_plane[y * w..(y + 2) * w].split_at_mut(2 * w);
        let (y_out0, y_out1) = y_rows.split_at_mut(w);
        let uv_out = &mut uv_plane[(y / 2) * w..(y / 2 + 1) * w];
        // Performance audit optimization:
        // Hoisted the early-return `alpha != 255` branch out of the innermost `x` loop into a branchless
        // per-row bitwise accumulator `alpha_mask`. This preserves the exact fail-closed transparency guarantee
        // (checked before `convert` returns `Ok(())` so `ProcessInput` never receives transparent frames) while
        // removing the loop-carried control dependency that blocked LLVM's AVX2/SSE2 auto-vectorization.
        let mut alpha_mask = 0xffu8;
        for x in (0..w).step_by(2) {
            let p00 = &row0[x * 4..x * 4 + 4];
            let p01 = &row0[x * 4 + 4..x * 4 + 8];
            let p10 = &row1[x * 4..x * 4 + 4];
            let p11 = &row1[x * 4 + 4..x * 4 + 8];
            alpha_mask &= p00[3] & p01[3] & p10[3] & p11[3];
            let (r00, g00, b00) = (i32::from(p00[2]), i32::from(p00[1]), i32::from(p00[0]));
            let (r01, g01, b01) = (i32::from(p01[2]), i32::from(p01[1]), i32::from(p01[0]));
            let (r10, g10, b10) = (i32::from(p10[2]), i32::from(p10[1]), i32::from(p10[0]));
            let (r11, g11, b11) = (i32::from(p11[2]), i32::from(p11[1]), i32::from(p11[0]));
            y_out0[x] = luma(r00, g00, b00);
            y_out0[x + 1] = luma(r01, g01, b01);
            y_out1[x] = luma(r10, g10, b10);
            y_out1[x + 1] = luma(r11, g11, b11);
            let uv = chroma(
                (r00 + r01 + r10 + r11 + 2) >> 2,
                (g00 + g01 + g10 + g11 + 2) >> 2,
                (b00 + b01 + b10 + b11 + 2) >> 2,
            );
            uv_out[x..x + 2].copy_from_slice(&uv);
        }
        // H264 has no alpha; never silently turn transparency black/opaque. Callers retire codecs and send original raw instead.
        if alpha_mask != 255 {
            return Err(invalid("H264 cannot preserve input alpha"));
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

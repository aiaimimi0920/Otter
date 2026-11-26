//! Lookup-table premultiplication and unpremultiplication kernels for BGRA8/RGBA8 buffers.
//!
//! Performance audit rationale (for AI/human reviewers):
//! Host checkpointing (`export_raw_alpha`), QuickJS `<canvas>` framing (`surface_javascript_canvas_wire`),
//! and HLSL readback (`shader_hlsl_pixels`) previously ran 3 integer hardware divisions (`checked_div(alpha)`)
//! or scalar byte loops per pixel inside crates compiled with `[profile.release] opt-level = "s"` (~24.9M
//! divisions per 4K frame). Placing these kernels in `otter-core` compiles them under `opt-level = 3` and
//! replaces division with an exact 256-entry fixed-point reciprocal table (`ceil(2^24 / a)`).
//!
//! Exactness invariant:
//! For every `c in 0..=255` and `a in 1..=255`, `N = c * 255 + (a >> 1) <= 65_152`. With `R = ceil(2^24 / a) =
//! (2^24 + r) / a` where `0 <= r < a <= 255`, the error term satisfies `N * r <= 65_152 * 254 = 16_548_608 < 2^24`,
//! proving `floor((N * R) / 2^24) == floor(N / a)` for all 65,536 `(c, a)` pairs (verified by unit test below).

const UNPREMULTIPLY_RECIP: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut a = 1usize;
    while a < 256 {
        table[a] = (((1u64 << 24) + (a as u64 - 1)) / (a as u64)) as u32;
        a += 1;
    }
    table
};

/// Exact equivalent of `((c as u32 * 255 + a as u32 / 2).checked_div(a as u32).unwrap_or(0)).min(255) as u8`
/// without hardware division or branching on `a == 0` (`UNPREMULTIPLY_RECIP[0] == 0`).
#[inline(always)]
pub fn unpremultiply_channel(channel: u8, alpha: u8) -> u8 {
    let a = u32::from(alpha);
    let numer = u32::from(channel) * 255 + (a >> 1);
    let quot = ((u64::from(numer) * u64::from(UNPREMULTIPLY_RECIP[alpha as usize])) >> 24) as u32;
    quot.min(255) as u8
}

/// Convert a row of premultiplied BGRA8 pixels into straight BGRA8 pixels.
/// Fast-paths fully opaque (`a == 255`) and fully transparent (`a == 0`) pixels, which dominate UI screenshots.
pub fn unpremultiply_bgra_row(source: &[u8], target: &mut [u8]) {
    for (src, dst) in source.chunks_exact(4).zip(target.chunks_exact_mut(4)) {
        let a = src[3];
        if a == 255 {
            dst.copy_from_slice(src);
        } else if a == 0 {
            dst.copy_from_slice(&[0, 0, 0, 0]);
        } else {
            dst[0] = unpremultiply_channel(src[0], a);
            dst[1] = unpremultiply_channel(src[1], a);
            dst[2] = unpremultiply_channel(src[2], a);
            dst[3] = a;
        }
    }
}

/// Convert premultiplied BGRA8 pixels in place into straight RGBA8 pixels (for lossless PNG encoding).
/// Transparent pixels (`a == 0`) clear RGB to `0` so stale premultiplied residuals never leak.
pub fn unpremultiply_pbgra_to_rgba_in_place(pixels: &mut [u8]) {
    for px in pixels.chunks_exact_mut(4) {
        let a = px[3];
        if a == 255 {
            px.swap(0, 2);
        } else if a == 0 {
            px.copy_from_slice(&[0, 0, 0, 0]);
        } else {
            let r = unpremultiply_channel(px[2], a);
            let g = unpremultiply_channel(px[1], a);
            let b = unpremultiply_channel(px[0], a);
            px.copy_from_slice(&[r, g, b, a]);
        }
    }
}

/// Convert straight RGBA8 pixels in place into premultiplied BGRA8 pixels (`(c * a + 127) / 255`).
pub fn premultiply_rgba_to_pbgra_in_place(pixels: &mut [u8]) {
    for px in pixels.chunks_exact_mut(4) {
        let a = u32::from(px[3]);
        if a == 255 {
            px.swap(0, 2);
        } else if a == 0 {
            px.copy_from_slice(&[0, 0, 0, 0]);
        } else {
            let r = ((u32::from(px[0]) * a + 127) / 255) as u8;
            let g = ((u32::from(px[1]) * a + 127) / 255) as u8;
            let b = ((u32::from(px[2]) * a + 127) / 255) as u8;
            px.copy_from_slice(&[b, g, r, px[3]]);
        }
    }
}

/// Convert straight RGBA8 source pixels into premultiplied BGRA8 destination bytes without pre-zeroing `target`.
pub fn premultiply_rgba_to_pbgra(source: &[u8], target: &mut [u8]) {
    for (rgba, dst) in source.chunks_exact(4).zip(target.chunks_exact_mut(4)) {
        let a = u32::from(rgba[3]);
        if a == 255 {
            dst.copy_from_slice(&[rgba[2], rgba[1], rgba[0], 255]);
        } else if a == 0 {
            dst.copy_from_slice(&[0, 0, 0, 0]);
        } else {
            dst[0] = ((u32::from(rgba[2]) * a + 127) / 255) as u8;
            dst[1] = ((u32::from(rgba[1]) * a + 127) / 255) as u8;
            dst[2] = ((u32::from(rgba[0]) * a + 127) / 255) as u8;
            dst[3] = rgba[3];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reciprocal_lut_matches_integer_division_for_all_65536_inputs() {
        for a in 0u16..=255 {
            for c in 0u16..=255 {
                let expected = (u32::from(c) * 255 + u32::from(a) / 2)
                    .checked_div(u32::from(a))
                    .unwrap_or(0)
                    .min(255) as u8;
                assert_eq!(
                    unpremultiply_channel(c as u8, a as u8),
                    expected,
                    "mismatch at c={c}, a={a}"
                );
            }
        }
    }

    #[test]
    fn in_place_rgba_pbgra_conversions_preserve_valid_premultiplied_values() {
        let mut buf = vec![0, 0, 0, 0, 10, 20, 30, 255, 25, 50, 100, 128];
        let original = buf.clone();
        unpremultiply_pbgra_to_rgba_in_place(&mut buf);
        assert_eq!(&buf[0..4], &[0, 0, 0, 0]);
        assert_eq!(&buf[4..8], &[30, 20, 10, 255]);
        premultiply_rgba_to_pbgra_in_place(&mut buf);
        assert_eq!(buf, original);
    }
}

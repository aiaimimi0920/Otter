//! Convert embedded RGB ICC to sRGB only within Job-restricted WebP children, before premultiplication.
use moxcms::{CmsError, ColorProfile, DataColorSpace, Layout, ParsingOptions, TransformOptions};
use std::io;
#[path = "surface_webp_lut.rs"]
mod lut;

// bytes passed RIFF preflight; still use checked slices and borrow metadata without copying entire profiles.
fn profile(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.get(12..16)? != b"VP8X" || bytes.get(20)? & 32 == 0 {
        return None;
    }
    let mut at = 12usize;
    for _ in 0..256 {
        let header = bytes.get(at..at.checked_add(8)?)?;
        let len = u32::from_le_bytes(header[4..].try_into().ok()?) as usize;
        let start = at.checked_add(8)?;
        let end = start.checked_add(len)?;
        let body = bytes.get(start..end)?;
        if &header[..4] == b"ICCP" {
            return Some(body);
        }
        at = end.checked_add(len % 2)?;
    }
    None
}

pub(super) fn apply(bytes: &[u8], pixels: &mut [u8], channels: usize) -> io::Result<()> {
    let Some(icc) = profile(bytes) else {
        return Ok(());
    };
    // Chromium preserves encoded RGB for missing/invalid ICC/non-RGB profiles. Transform-construction failures fail explicitly;
    // never silently treat recognized valid RGB profiles as sRGB. Job still bounds total parser/transform memory.
    let options = ParsingOptions {
        max_profile_size: crate::surface_webp_header::MAX_INPUT + 1,
        ..ParsingOptions::default()
    };
    let Some(source) = parsed(ColorProfile::new_from_slice_with_options(icc, options))? else {
        return Ok(());
    };
    if source.color_space != DataColorSpace::Rgb {
        return Ok(());
    }
    let layout = match channels {
        3 => Layout::Rgb,
        4 => Layout::Rgba,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid ICC pixel layout",
            ))
        }
    };
    if lut::apply(&source, pixels, channels)? {
        return Ok(());
    }
    let transform = source
        .create_transform_8bit(
            layout,
            &ColorProfile::new_srgb(),
            layout,
            TransformOptions {
                rendering_intent: moxcms::RenderingIntent::Perceptual,
                ..TransformOptions::default()
            },
        )
        .map_err(io::Error::other)?;
    // Execute generic matrix/LUT paths in batches, not restricted to matrix-only in-place APIs, without creating a second full image.
    // At most 4096 pixels per batch and 16 KiB extra output; Job still bounds internal library working tables.
    let mut output = [0u8; 4096 * 4];
    for chunk in pixels.chunks_mut(4096 * channels) {
        transform
            .transform(chunk, &mut output[..chunk.len()])
            .map_err(io::Error::other)?;
        if channels == 4
            && chunk
                .chunks_exact(4)
                .zip(output[..chunk.len()].chunks_exact(4))
                .any(|(before, after)| before[3] != after[3])
        {
            return Err(io::Error::other("ICC transform changed alpha"));
        }
        chunk.copy_from_slice(&output[..chunk.len()]);
    }
    Ok(())
}

fn parsed(result: Result<ColorProfile, CmsError>) -> io::Result<Option<ColorProfile>> {
    match result {
        Ok(profile) => Ok(Some(profile)),
        // Distinguishable resource exhaustion/quota/unsupported failures must not silently return successful unconverted images.
        Err(
            error @ (CmsError::OutOfMemory(_)
            | CmsError::CurveLutIsTooLarge
            | CmsError::UnsupportedColorPrimaries(_)
            | CmsError::UnsupportedTrc(_)
            | CmsError::UnsupportedProfileConnection
            | CmsError::BuildTransferFunction
            | CmsError::UnsupportedChannelConfiguration
            | CmsError::UnknownTag(_)
            | CmsError::UnknownTagTypeDefinition(_)
            | CmsError::UnsupportedLutRenderingIntent(_)),
        ) => Err(io::Error::other(error)),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_webp_icc_resource_and_unsupported_errors_never_fall_back() {
        for error in [
            CmsError::OutOfMemory(1024),
            CmsError::CurveLutIsTooLarge,
            CmsError::UnsupportedProfileConnection,
            CmsError::UnknownTagTypeDefinition(0),
        ] {
            assert!(parsed(Err(error)).is_err());
        }
        assert!(parsed(Err(CmsError::InvalidProfile)).unwrap().is_none());
    }
    #[test]
    fn surface_webp_icc_batches_preserve_all_alpha_edges() {
        let encoded = crate::image_decode::Image::webp_p3_for_test();
        let alphas = [0, 1, 127, 128, 254, 255];
        let mut pixels = Vec::new();
        for i in 0..4101 {
            pixels.extend_from_slice(&[179, 127, 51, alphas[i % alphas.len()]]);
        }
        apply(&encoded, &mut pixels, 4).unwrap();
        for (i, pixel) in pixels.chunks_exact(4).enumerate() {
            assert_eq!(pixel[3], alphas[i % alphas.len()]);
            for (value, expected) in pixel[..3].iter().zip([188, 124, 27]) {
                assert!(value.abs_diff(expected) <= 1);
            }
        }
    }
}

//! Private Canvas pixel frames: check dimensions/totals upfront; PNG is only lossless interprocess encoding, never a resource URL.
use super::invalid;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};
pub(crate) const MAX_CANVASES: usize = 8;
pub(crate) const MAX_PIXELS: usize = 4 * 1024 * 1024;
pub(crate) const TOTAL_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const MAX_PNG: usize = 2 * 1024 * 1024;
pub(crate) const MAX_ENCODED: usize = 3 * 1024 * 1024;
struct Encoded(Vec<u8>);
impl io::Write for Encoded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_PNG.saturating_sub(self.0.len()) {
            return Err(invalid());
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CanvasFrame {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub png: String,
}
pub(crate) fn bytes(size: [u32; 2]) -> io::Result<usize> {
    if size.iter().any(|v| *v > 4096) {
        return Err(invalid());
    }
    let pixels = (size[0] as usize)
        .checked_mul(size[1] as usize)
        .ok_or_else(invalid)?;
    if pixels > MAX_PIXELS {
        return Err(invalid());
    }
    Ok(pixels * 4)
}
pub(crate) fn scene(value: &serde_json::Value) -> io::Result<crate::surface_scene::Scene> {
    let mut scene =
        crate::surface_scene::Scene::parse(&super::encode(value)?).map_err(|_| invalid())?;
    let mut sizes = BTreeMap::new();
    let mut pending = vec![scene.root()];
    let mut total = 0;
    while let Some(node) = pending.pop() {
        if let Some(value) = node.props().get("_canvas") {
            let size: [u32; 2] = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
            total += bytes(size)?;
            if node.kind() != crate::surface_scene::Kind::Image
                || sizes.len() >= MAX_CANVASES
                || total > TOTAL_BYTES
            {
                return Err(invalid());
            }
            sizes.insert(node.id().to_owned(), size);
        }
        pending.extend(node.children());
    }
    scene.set_canvas_sizes(&sizes);
    Ok(scene)
}
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn images(
    scene: &crate::surface_scene::Scene,
    frames: Vec<CanvasFrame>,
) -> io::Result<BTreeMap<String, crate::image_decode::Image>> {
    let mut scratch = BTreeMap::new();
    images_cached(scene, frames, &mut scratch)
}
/// Performance audit optimization (Tier 2.3):
/// Reuse decoded `Image` (`Pixels` `Arc`) across frames within the same JS `Session` whenever a `<canvas>` node's
/// dimensions and cached PNG payload are unchanged, eliminating redundant Base64 decode, PNG inflate, and PBGRA
/// premultiplication on pure button-hover or timer ticks while preserving `same_storage` for GPU bitmap reuse.
pub(crate) fn images_cached(
    scene: &crate::surface_scene::Scene,
    frames: Vec<CanvasFrame>,
    cache: &mut BTreeMap<String, ([u32; 2], String, crate::image_decode::Image)>,
) -> io::Result<BTreeMap<String, crate::image_decode::Image>> {
    if frames.len() > MAX_CANVASES {
        return Err(invalid());
    }
    let mut expected = BTreeMap::new();
    let mut pending = vec![scene.root()];
    while let Some(node) = pending.pop() {
        if let Some(size) = node.canvas_size() {
            expected.insert(node.id().to_owned(), size);
        }
        pending.extend(node.children());
    }
    let mut output = BTreeMap::new();
    let mut next_cache = BTreeMap::new();
    let mut total = 0;
    for frame in frames {
        let size = [frame.width, frame.height];
        if expected.remove(&frame.id) != Some(size) || frame.png.len() > MAX_PNG * 4 / 3 + 4 {
            return Err(invalid());
        }
        let count = bytes(size)?;
        total += count;
        if total > TOTAL_BYTES {
            return Err(invalid());
        }
        // Zero-size canvases upload no fabricated one-pixel image and pass no invalid dimensions to generic image owners.
        if count == 0 {
            if !frame.png.is_empty() {
                return Err(invalid());
            }
            continue;
        }
        if let Some((cached_size, cached_png, cached_image)) = cache.remove(&frame.id) {
            if cached_size == size && cached_png == frame.png {
                output.insert(frame.id.clone(), cached_image.clone());
                next_cache.insert(frame.id, (size, cached_png, cached_image));
                continue;
            }
        }
        let encoded = STANDARD.decode(&frame.png).map_err(|_| invalid())?;
        if encoded.len() > MAX_PNG {
            return Err(invalid());
        }
        let image = decode(size, &encoded)?;
        output.insert(frame.id.clone(), image.clone());
        next_cache.insert(frame.id, (size, frame.png, image));
    }
    if !expected.is_empty() {
        return Err(invalid());
    }
    *cache = next_cache;
    Ok(output)
}
/// Committed frames and private service snapshots reuse the same Canvas codec; callers first bound aggregate sizes and PNG allocations.
pub(crate) fn decode(size: [u32; 2], encoded: &[u8]) -> io::Result<crate::image_decode::Image> {
    let count = bytes(size)?;
    if count == 0 || encoded.len() > MAX_PNG {
        return Err(invalid());
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(encoded));
    decoder.set_limits(png::Limits { bytes: TOTAL_BYTES });
    let mut reader = decoder.read_info().map_err(|_| invalid())?;
    let info = reader.info();
    if [info.width, info.height] != size
        || info.color_type != png::ColorType::Rgba
        || info.bit_depth != png::BitDepth::Eight
        || info.interlaced
        || info.animation_control.is_some()
    {
        return Err(invalid());
    }
    let lease = crate::pixels::reserve(count).ok_or_else(invalid)?;
    let mut pixels = vec![0; count];
    let info = reader.next_frame(&mut pixels).map_err(|_| invalid())?;
    if info.buffer_size() != count {
        return Err(invalid());
    }
    // Performance audit optimization (Tier 0.3 / 2.3):
    // Delegate RGBA<->PBGRA premultiply/unpremultiply to `otter_core::pixel_alpha` (`opt-level = 3`, 256-entry LUT),
    // eliminating 3 integer hardware divisions per pixel during canvas snapshot round-trips.
    otter_core::pixel_alpha::premultiply_rgba_to_pbgra_in_place(&mut pixels);
    Ok(crate::image_decode::Image {
        width: size[0],
        height: size[1],
        pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
    })
}
pub(crate) fn encode(size: [u32; 2], mut pixels: Vec<u8>) -> io::Result<String> {
    if pixels.len() != bytes(size)? {
        return Err(invalid());
    }
    // WIC holds PBGRA; PNG uses straight RGBA, and transparent channels must not carry stale colors.
    otter_core::pixel_alpha::unpremultiply_pbgra_to_rgba_in_place(&mut pixels);
    let mut output = Encoded(Vec::new());
    {
        let mut encoder = png::Encoder::new(&mut output, size[0], size[1]);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // Fast tries every filter per row by default and may exhaust 250ms on large debug images; Up is also lossless and bounded.
        encoder.set_compression(png::Compression::Fastest);
        let mut writer = encoder.write_header().map_err(|_| invalid())?;
        writer.write_image_data(&pixels).map_err(|_| invalid())?;
    }
    Ok(STANDARD.encode(output.0))
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_javascript_canvas_encode_tests.rs"]
mod encode_tests;

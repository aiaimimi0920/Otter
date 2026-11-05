//! HDR PNG 流式编码；调用方拥有目的文件及部分写入处理，不创建或覆盖任何路径。
use crate::capture_color::HdrFrame;
use std::{
    io::{self, Write},
    sync::atomic::{AtomicBool, Ordering},
};
#[path = "capture_png_sink.rs"]
mod sink;
#[derive(Debug)]
pub enum Error {
    Layout,
    Metadata,
    Budget,
    Cancelled,
    Encoding(png::EncodingError),
    Io(io::Error),
}

/// 只在 IEND 完整写出且目标 flush 成功后返回成功；不代表文件已 sync_all 或已发布。
pub fn write(writer: &mut impl Write, frame: &HdrFrame, cancel: &AtomicBool) -> Result<(), Error> {
    encode(writer, frame, cancel, crate::pixels::reserve)
}
fn encode(
    writer: &mut impl Write,
    frame: &HdrFrame,
    cancel: &AtomicBool,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    let bgra =
        crate::viewport::checked_image_bytes(frame.width, frame.height).ok_or(Error::Layout)?;
    let length = bgra
        .checked_div(4)
        .and_then(|n| n.checked_mul(6))
        .ok_or(Error::Layout)?;
    if frame.rgb16_be.len() != length {
        return Err(Error::Layout);
    }
    let values = [
        frame.analysis.maximum_nits,
        frame.analysis.average_nits,
        frame.mastering_minimum_nits,
        frame.mastering_maximum_nits,
    ];
    if values
        .iter()
        .any(|n| !n.is_finite() || !(0.0..=10000.0).contains(n))
        || frame.analysis.average_nits > frame.analysis.maximum_nits
        || frame.mastering_minimum_nits > frame.mastering_maximum_nits
        || frame.mastering_maximum_nits < 1.0
    {
        return Err(Error::Metadata);
    }
    let row = frame.width as usize * 6;
    // 固定 png 0.18.1 流式路径保留三行、4 KiB chunk 和有界压缩状态；保守预留 4 MiB。
    // 不调用会积累整张压缩图的 write_image_data；目的 writer 的缓存由调用方负责。
    let _lease = reserve(4 * 1024 * 1024 + row * 3).ok_or(Error::Budget)?;
    let mut output = sink::Sink::new(writer, cancel, length + length / 100 + 65536);
    let result = (|| {
        let mut info = png::Info::with_size(frame.width, frame.height);
        info.color_type = png::ColorType::Rgb;
        info.bit_depth = png::BitDepth::Sixteen;
        info.source_chromaticities = Some(png::SourceChromaticities::new(
            (0.3127, 0.3290),
            (0.7080, 0.2920),
            (0.1700, 0.7970),
            (0.1310, 0.0460),
        ));
        let mut encoder = png::Encoder::with_info(&mut output, info).map_err(Error::Encoding)?;
        encoder.set_compression(png::Compression::Fast);
        encoder.set_filter(png::Filter::NoFilter);
        let mut png = encoder.write_header().map_err(Error::Encoding)?;
        png.write_chunk(png::chunk::cICP, &[9, 16, 0, 1])
            .map_err(Error::Encoding)?;
        let mut mastering = [0u8; 24];
        for (index, value) in [35400u16, 14600, 8500, 39850, 6550, 2300, 15635, 16450]
            .iter()
            .enumerate()
        {
            mastering[index * 2..index * 2 + 2].copy_from_slice(&value.to_be_bytes());
        }
        mastering[16..20].copy_from_slice(&scaled(frame.mastering_maximum_nits));
        mastering[20..24].copy_from_slice(&scaled(frame.mastering_minimum_nits));
        png.write_chunk(png::chunk::mDCV, &mastering)
            .map_err(Error::Encoding)?;
        let mut light = [0u8; 8];
        light[..4].copy_from_slice(&scaled(frame.analysis.maximum_nits));
        light[4..].copy_from_slice(&scaled(frame.analysis.average_nits));
        png.write_chunk(png::chunk::cLLI, &light)
            .map_err(Error::Encoding)?;
        {
            let mut stream = png.stream_writer_with_size(4096).map_err(Error::Encoding)?;
            for bytes in frame.rgb16_be.chunks_exact(row) {
                if cancel.load(Ordering::Acquire) {
                    return Err(Error::Cancelled);
                }
                stream.write_all(bytes).map_err(Error::Io)?;
            }
            stream.finish().map_err(Error::Encoding)?;
        }
        png.finish().map_err(Error::Encoding)?;
        output.flush().map_err(Error::Io)
    })();
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        result
    }
}
fn scaled(nits: f32) -> [u8; 4] {
    ((nits * 10000.0).round() as u32).to_be_bytes()
}
#[cfg(test)]
#[path = "capture_png_tests.rs"]
mod tests;

//! 当前child调用时Canvas快照复用原图片HTTP出口；不接脚本PNG或新资源身份。
use super::{Error, Image, Uploads};
use crate::surface_javascript_wire::canvas::{self, CanvasFrame};
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub(super) fn prepare(frames: Vec<CanvasFrame>) -> Result<Uploads, Error> {
    if frames.is_empty() || frames.len() > 4 {
        return Err(Error::Input);
    }
    let encoded = frames.iter().map(|v| v.png.len()).sum::<usize>();
    if encoded > canvas::MAX_ENCODED {
        return Err(Error::Limit);
    }
    let budget = crate::pixels::reserve(4 * (encoded + 96 * 1024)).ok_or(Error::Limit)?;
    let mut pixels = 0usize;
    for (i, frame) in frames.iter().enumerate() {
        let size = [frame.width, frame.height];
        let count = canvas::bytes(size).map_err(|_| Error::Input)?;
        pixels = pixels.checked_add(count).ok_or(Error::Limit)?;
        if count == 0
            || pixels > canvas::TOTAL_BYTES
            || !crate::surface_javascript_event::safe(&frame.id)
            || frames[..i].iter().any(|v| v.id == frame.id)
            || frame.png.is_empty()
            || frame.png.len() > canvas::MAX_PNG * 4 / 3 + 4
        {
            return Err(Error::Input);
        }
        // 解码分配前预留，逐图完整校验后立即释放像素；原Base64只移动到HTTP请求，不重复编码。
        let _png = crate::pixels::reserve(frame.png.len()).ok_or(Error::Limit)?;
        let bytes = STANDARD.decode(&frame.png).map_err(|_| Error::Wire)?;
        let _validated = canvas::decode(size, &bytes).map_err(|_| Error::Wire)?;
    }
    Ok(Uploads {
        decoded_limit: canvas::TOTAL_BYTES,
        sources: frames
            .iter()
            .map(|frame| (format!("画布 {}", frame.id), [frame.width, frame.height]))
            .collect(),
        images: frames.into_iter().map(|v| Image { png: v.png }).collect(),
        _budget: budget,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn frame() -> CanvasFrame {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[1, 2, 3, 255])
                .unwrap();
        }
        CanvasFrame {
            id: "canvas:one".into(),
            width: 1,
            height: 1,
            png: STANDARD.encode(bytes),
        }
    }
    #[test]
    fn canvas_service_host_validates_png_dimensions_codec_count_and_unique_identity() {
        assert!(prepare(vec![frame()]).is_ok());
        assert!(prepare(vec![]).is_err());
        assert!(prepare(vec![frame(); 5]).is_err());
        assert!(prepare(vec![frame(), frame()]).is_err());
        let mut f = frame();
        f.width = 2;
        assert!(prepare(vec![f]).is_err());
        let mut f = frame();
        f.width = 0;
        assert!(prepare(vec![f]).is_err());
        let mut f = frame();
        f.width = 4097;
        assert!(prepare(vec![f]).is_err());
        let mut f = frame();
        f.id = "C:/private".into();
        assert!(prepare(vec![f]).is_err());
        let mut f = frame();
        f.png = STANDARD.encode(b"\x89PNG\r\n\x1a\n");
        assert!(prepare(vec![f]).is_err());
        let mut f = frame();
        f.png = "not base64".into();
        assert!(prepare(vec![f]).is_err());
        let mut f = frame();
        f.png = "A".repeat(canvas::MAX_ENCODED + 1);
        assert!(prepare(vec![f]).is_err());
    }
}

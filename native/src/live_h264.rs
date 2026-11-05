//! 与 main baseline H264 约束一致的 codec 前置检查；不是完整语法/像素解码器。
#[path = "live_h264_bits.rs"]
mod bits;
#[path = "live_h264_sps.rs"]
mod sps;
pub type Result<T> = std::result::Result<T, &'static str>;
fn require(condition: bool, error: &'static str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(error)
    }
}
fn prefix(bytes: &[u8], at: usize) -> Option<usize> {
    if bytes.get(at..at + 4) == Some(&[0, 0, 0, 1]) {
        Some(4)
    } else if bytes.get(at..at + 3) == Some(&[0, 0, 1]) {
        Some(3)
    } else {
        None
    }
}
/// 仅在完整有界 AU 上调用；逐单元借用，不建立 NAL 列表或复制压缩帧。
pub fn validate(bytes: &[u8], width: u32, height: u32, keyframe: bool) -> Result<()> {
    require(
        !bytes.is_empty() && bytes.len() <= 1024 * 1024,
        "access unit size invalid",
    )?;
    require(
        width >= 2
            && height >= 2
            && width <= 4096
            && height <= 4096
            && width.is_multiple_of(2)
            && height.is_multiple_of(2)
            && u64::from(width) * u64::from(height) <= 8_294_400,
        "wire dimensions invalid",
    )?;
    let (mut cursor, mut count) = (0, 0);
    let (mut has_sps, mut has_pps, mut idr, mut delta) = (false, false, false, false);
    while cursor < bytes.len() {
        let begin = cursor + prefix(bytes, cursor).ok_or("Annex-B start code missing")?;
        let end = (begin..bytes.len())
            .find(|&at| prefix(bytes, at).is_some())
            .unwrap_or(bytes.len());
        require(end > begin, "empty NAL unit")?;
        let nal = &bytes[begin..end];
        require(nal[0] & 0x80 == 0, "forbidden NAL bit")?;
        count += 1;
        require(count <= 256, "NAL count budget")?;
        match nal[0] & 31 {
            7 => {
                require(!idr && !delta, "late SPS")?;
                sps::validate(nal, [width, height])?;
                has_sps = true;
            }
            8 => {
                require(!idr && !delta && nal.len() >= 2, "late or empty PPS")?;
                let mut bits = bits::Bits::new(&nal[1..])?;
                require(bits.ue()? <= 255 && bits.ue()? <= 31, "PPS identity bounds")?;
                require(bits.read(1)? == 0, "CABAC unsupported")?;
                bits.read(1)?;
                require(bits.ue()? == 0, "slice groups unsupported")?;
                require(bits.ue()? <= 3 && bits.ue()? <= 3, "PPS reference budget")?;
                has_pps = true;
            }
            5 => {
                require(
                    !delta && has_sps && has_pps && nal.len() > 1,
                    "IDR requires SPS/PPS",
                )?;
                idr = true;
            }
            1 => {
                require(!idr && nal.len() > 1, "mixed or empty picture")?;
                delta = true;
            }
            6 | 9 | 10 | 11 | 12 => {}
            _ => return Err("unsupported NAL type"),
        }
        cursor = end;
    }
    require(
        keyframe == idr && (idr || delta) && !(delta && (has_sps || has_pps)),
        "video keyframe contract",
    )
}
#[cfg(test)]
#[path = "live_h264_tests.rs"]
mod tests;

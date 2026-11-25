//! baseline SPS/VUI 的分配及颜色边界；不规范化来自网络的不合法色彩标记。
use super::{bits::Bits, require, Result};
pub(super) fn validate(nal: &[u8], expected: [u32; 2]) -> Result<()> {
    require(nal.len() <= 1024, "SPS size budget")?;
    let mut bits = Bits::new(&nal[1..])?;
    require(bits.read(8)? == 66, "baseline 8-bit 4:2:0 required")?;
    require(bits.read(8)? & 3 == 0, "SPS reserved bits")?;
    require(bits.read(8)? <= 52, "SPS level budget")?;
    require(
        bits.ue()? <= 31 && bits.ue()? <= 12,
        "SPS identity/frame number bounds",
    )?;
    match bits.ue()? {
        0 => require(bits.ue()? <= 12, "POC width budget")?,
        2 => {}
        _ => return Err("POC mode unsupported"),
    }
    let refs = bits.ue()?;
    require(refs <= 4, "reference picture budget")?;
    bits.read(1)?;
    let width = bits
        .ue()?
        .checked_add(1)
        .and_then(|v| v.checked_mul(16))
        .ok_or("coded width overflow")?;
    let height = bits
        .ue()?
        .checked_add(1)
        .and_then(|v| v.checked_mul(16))
        .ok_or("coded height overflow")?;
    require(
        width <= 4096 && height <= 4096 && u64::from(width) * u64::from(height) <= 8_294_400,
        "coded surface budget",
    )?;
    require(bits.read(1)? == 1, "interlaced video unsupported")?;
    bits.read(1)?;
    let (mut crop_x, mut crop_y) = (0, 0);
    if bits.read(1)? != 0 {
        crop_x = bits
            .ue()?
            .checked_add(bits.ue()?)
            .and_then(|v| v.checked_mul(2))
            .ok_or("horizontal crop overflow")?;
        crop_y = bits
            .ue()?
            .checked_add(bits.ue()?)
            .and_then(|v| v.checked_mul(2))
            .ok_or("vertical crop overflow")?;
    }
    require(
        width.checked_sub(crop_x) == Some(expected[0])
            && height.checked_sub(crop_y) == Some(expected[1]),
        "SPS/wire dimensions mismatch",
    )?;
    if bits.read(1)? != 0 {
        vui(&mut bits, refs)?;
    }
    Ok(())
}
fn vui(bits: &mut Bits, refs: u32) -> Result<()> {
    if bits.read(1)? != 0 && bits.read(8)? == 255 {
        bits.read(16)?;
        bits.read(16)?;
    }
    if bits.read(1)? != 0 {
        bits.read(1)?;
    }
    if bits.read(1)? != 0 {
        bits.read(3)?;
        require(bits.read(1)? == 0, "full-range YUV unsupported")?;
        if bits.read(1)? != 0 {
            let primaries = bits.read(8)?;
            let transfer = bits.read(8)?;
            let matrix = bits.read(8)?;
            require(
                [1, 2].contains(&primaries)
                    && [2, 13].contains(&transfer)
                    && [1, 2].contains(&matrix),
                "colorimetry outside BT709 contract",
            )?;
        }
    }
    if bits.read(1)? != 0 {
        require(bits.ue()? <= 5 && bits.ue()? <= 5, "chroma location bounds")?;
    }
    if bits.read(1)? != 0 {
        require(
            bits.read(32)? > 0 && bits.read(32)? > 0,
            "VUI timing invalid",
        )?;
        bits.read(1)?;
    }
    let nal_hrd = bits.read(1)? != 0;
    if nal_hrd {
        hrd(bits)?;
    }
    let vcl_hrd = bits.read(1)? != 0;
    if vcl_hrd {
        hrd(bits)?;
    }
    if nal_hrd || vcl_hrd {
        bits.read(1)?;
    }
    bits.read(1)?;
    if bits.read(1)? != 0 {
        bits.read(1)?;
        for _ in 0..4 {
            require(bits.ue()? <= 16, "VUI denominator/vector budget")?;
        }
        let reorder = bits.ue()?;
        let buffering = bits.ue()?;
        require(
            reorder <= refs && buffering >= refs && buffering <= 4,
            "decoded picture buffer budget",
        )?;
    }
    Ok(())
}
fn hrd(bits: &mut Bits) -> Result<()> {
    let count = bits.ue()?;
    require(count <= 31, "HRD count budget")?;
    bits.read(8)?;
    for _ in 0..=count {
        bits.ue()?;
        bits.ue()?;
        bits.read(1)?;
    }
    bits.read(20)?;
    Ok(())
}

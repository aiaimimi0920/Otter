//! 系统baseline/NV12低延迟协商；不裁切、不改输入尺寸，不配置实验已拒绝的B帧/GOP属性。
use super::*;
pub(super) fn validate_size([width, height]: [u32; 2]) -> Result<usize> {
    if width < 48
        || height < 48
        || width > 4096
        || height > 4096
        || !width.is_multiple_of(2)
        || !height.is_multiple_of(2)
        || u64::from(width.div_ceil(16) * 16) * u64::from(height.div_ceil(16) * 16) > 8_294_400
    {
        return Err(invalid(
            "Encoder requires bounded even dimensions of at least 48x48",
        ));
    }
    Ok(width as usize * height as usize * 3 / 2)
}
pub(super) fn configure(transform: &IMFTransform, [width, height]: [u32; 2]) -> Result<u32> {
    unsafe {
        let api: ICodecAPI = transform.cast()?;
        // 此encoder要求VT_BOOL；decoder的VT_UI4不适用于这个系统MFT。
        api.SetValue(&CODECAPI_AVLowLatencyMode, &VARIANT::from(true))?;
        for (output, subtype) in [(true, MFVideoFormat_H264), (false, MFVideoFormat_NV12)] {
            let media = MFCreateMediaType()?;
            media.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            media.SetGUID(&MF_MT_SUBTYPE, &subtype)?;
            media.SetUINT64(
                &MF_MT_FRAME_SIZE,
                (u64::from(width) << 32) | u64::from(height),
            )?;
            media.SetUINT64(&MF_MT_FRAME_RATE, (30u64 << 32) | 1)?;
            media.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1u64 << 32) | 1)?;
            media.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            for (id, value) in [
                (MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_16_235.0),
                (MF_MT_YUV_MATRIX, MFVideoTransferMatrix_BT709.0),
                (MF_MT_VIDEO_PRIMARIES, MFVideoPrimaries_BT709.0),
                (MF_MT_TRANSFER_FUNCTION, MFVideoTransFunc_sRGB.0),
            ] {
                media.SetUINT32(&id, value as u32)?;
            }
            if output {
                let bitrate = (width * height)
                    .saturating_mul(4)
                    .clamp(500_000, 16_000_000);
                media.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
                media.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_Base.0 as u32)?;
                transform.SetOutputType(0, &media, 0)?;
            } else {
                transform.SetInputType(0, &media, 0)?;
            }
        }
        let info = transform.GetOutputStreamInfo(0)?;
        if info.cbSize.max(4096) > samples::MAX_BUFFER
            || info.dwFlags & MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32 != 0
        {
            return Err(invalid("Encoder output allocation contract unsupported"));
        }
        transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)?;
        transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)?;
        Ok(info.cbSize.max(4096))
    }
}

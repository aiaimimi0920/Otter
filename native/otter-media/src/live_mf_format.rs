//! 同步 H264→NV12 媒体类型协商；失败消费整个 owner，不暴露半初始化 transform。
use super::{DecoderOwner, Layout};
use windows::{
    core::{Error, Interface, Result},
    Win32::{Foundation::E_INVALIDARG, Media::MediaFoundation::*, System::Variant::VARIANT},
};
pub struct ConfiguredDecoder {
    pub(super) owner: DecoderOwner,
    layout: Layout,
}
impl ConfiguredDecoder {
    pub fn layout(&self) -> Layout {
        self.layout
    }
    /// 验证 owner 仍持有已协商的输出；不把缓存 layout 当作未来 stream-change 的结果。
    pub fn current_layout(&self) -> Result<Layout> {
        let media = unsafe { self.owner._transform.transform.GetOutputCurrentType(0)? };
        Layout::read(
            &media,
            self.layout.visible_width,
            self.layout.visible_height,
        )
    }
}
impl DecoderOwner {
    pub fn configure(self, width: u32, height: u32) -> Result<ConfiguredDecoder> {
        validate_size(width, height)?;
        let transform = &self._transform.transform;
        unsafe {
            transform.GetAttributes()?.SetUINT32(&MF_LOW_LATENCY, 1)?;
            let codec: ICodecAPI = transform.cast()?;
            codec.SetValue(&CODECAPI_AVLowLatencyMode, &VARIANT::from(1_u32))?;
            let media = MFCreateMediaType()?;
            media.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            media.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
            media.SetUINT64(
                &MF_MT_FRAME_SIZE,
                (u64::from(width) << 32) | u64::from(height),
            )?;
            media.SetUINT64(&MF_MT_FRAME_RATE, (30_u64 << 32) | 1)?;
            media.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            transform.SetInputType(0, &media, 0)?;
            let layout = select_output(transform, width, height)?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)?;
            Ok(ConfiguredDecoder {
                owner: self,
                layout,
            })
        }
    }
}
fn validate_size(width: u32, height: u32) -> Result<()> {
    // 系统 H264 decoder 的最小分辨率为 48×48；协商成功不代表较小尺寸会出图。
    // wire 仍允许小图，连接 owner 应将 codec 拒绝映射为 JPEG/raw fallback。
    if width < 48
        || height < 48
        || width > 4096
        || height > 4096
        || !width.is_multiple_of(2)
        || !height.is_multiple_of(2)
        || u64::from(width) * u64::from(height) > 8_294_400
    {
        return Err(Error::new(
            E_INVALIDARG,
            "System H264 decoder requires bounded even dimensions of at least 48x48",
        ));
    }
    Ok(())
}
pub(super) fn select_output(transform: &IMFTransform, width: u32, height: u32) -> Result<Layout> {
    unsafe {
        for index in 0..32 {
            let media = transform.GetOutputAvailableType(0, index)?;
            if media.GetGUID(&MF_MT_SUBTYPE)? != MFVideoFormat_NV12 {
                continue;
            }
            let layout = Layout::read(&media, width, height)?;
            for (attribute, expected) in [
                (MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_16_235.0 as u32),
                (MF_MT_YUV_MATRIX, MFVideoTransferMatrix_BT709.0 as u32),
                (MF_MT_VIDEO_PRIMARIES, MFVideoPrimaries_BT709.0 as u32),
                (MF_MT_TRANSFER_FUNCTION, MFVideoTransFunc_sRGB.0 as u32),
            ] {
                match media.GetUINT32(&attribute) {
                    Ok(value) if value == 0 || value == expected => {}
                    Ok(_) => {
                        return Err(Error::new(
                            E_INVALIDARG,
                            "Decoder output color contract mismatch",
                        ))
                    }
                    Err(error) if error.code() == MF_E_ATTRIBUTENOTFOUND => {}
                    Err(error) => return Err(error),
                }
            }
            transform.SetOutputType(0, &media, 0)?;
            if transform.GetOutputStreamInfo(0)?.cbSize > 32 * 1024 * 1024 {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Decoder output allocation exceeds limit",
                ));
            }
            return Ok(layout);
        }
    }
    Err(Error::new(
        E_INVALIDARG,
        "Decoder has no bounded NV12 output",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_size_is_bounded_before_media_configuration() {
        for size in [
            (0, 2),
            (3, 2),
            (32, 32),
            (46, 64),
            (64, 46),
            (4096, 4096),
            (u32::MAX, 2),
        ] {
            assert!(validate_size(size.0, size.1).is_err());
        }
        assert!(validate_size(3840, 2160).is_ok());
        assert!(validate_size(48, 48).is_ok());
    }
    #[test]
    #[ignore = "requires installed system H264 decoder; negotiates only, no media or network"]
    fn actual_decoder_negotiates_nv12_and_configuration_failure_returns_permit() {
        assert!(DecoderOwner::create().unwrap().configure(3, 2).is_err());
        for (width, height) in [(64, 64), (680, 430)] {
            let configured = DecoderOwner::create()
                .unwrap()
                .configure(width, height)
                .unwrap();
            let layout = configured.layout();
            assert_eq!(
                (layout.visible_width, layout.visible_height),
                (width, height)
            );
            assert_eq!(configured.current_layout().unwrap(), layout);
            eprintln!(
                "native-mf-format: visible={}x{} storage={}x{}",
                width, height, layout.width, layout.height
            );
            drop(configured);
        }
        eprintln!(
            "native-mf-format: nv12=true current-type-verified=true rejected-size-released=true"
        );
    }
}

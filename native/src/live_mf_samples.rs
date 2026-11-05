//! 同步 MFT 的 sample 边界；所有输出事件及失败 sample 均在本次调用内释放。
use std::mem::ManuallyDrop;
use windows::{
    core::{Error, Result},
    Win32::{Foundation::E_INVALIDARG, Media::MediaFoundation::*},
};
pub(super) const MAX_BUFFER: u32 = 32 * 1024 * 1024;
pub(super) enum Output {
    NeedInput,
    FormatChanged,
    Frame(IMFSample),
}
pub(super) struct Locked<'a>(pub &'a IMFMediaBuffer);
impl Drop for Locked<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.Unlock();
        }
    }
}
pub(super) fn input(bytes: &[u8], timestamp: i64, keyframe: bool) -> Result<IMFSample> {
    if bytes.is_empty() || bytes.len() > 1024 * 1024 || timestamp < 0 {
        return Err(Error::new(E_INVALIDARG, "Invalid decoder sample input"));
    }
    unsafe {
        let sample = MFCreateSample()?;
        let buffer = MFCreateMemoryBuffer(bytes.len() as u32)?;
        let mut ptr = std::ptr::null_mut();
        let mut capacity = 0;
        buffer.Lock(&mut ptr, Some(&mut capacity), None)?;
        let lock = Locked(&buffer);
        if ptr.is_null() || capacity < bytes.len() as u32 {
            return Err(Error::new(E_INVALIDARG, "Decoder input buffer bounds"));
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        drop(lock);
        buffer.SetCurrentLength(bytes.len() as u32)?;
        sample.AddBuffer(&buffer)?;
        sample.SetSampleTime(timestamp)?;
        sample.SetSampleDuration(10_000_000 / 30)?;
        sample.SetUINT32(&MFSampleExtension_CleanPoint, u32::from(keyframe))?;
        Ok(sample)
    }
}
pub(super) fn receive(transform: &IMFTransform) -> Result<Output> {
    receive_bounded(transform, MAX_BUFFER)
}
pub(super) fn receive_bounded(transform: &IMFTransform, maximum: u32) -> Result<Output> {
    unsafe {
        let info = transform.GetOutputStreamInfo(0)?;
        if info.cbSize.max(4096) > maximum {
            return Err(Error::new(E_INVALIDARG, "Decoder output allocation budget"));
        }
        let sample = if info.dwFlags & MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32 == 0 {
            let sample = MFCreateSample()?;
            sample.AddBuffer(&MFCreateMemoryBuffer(info.cbSize.max(4096))?)?;
            Some(sample)
        } else {
            None
        };
        let mut buffer = MFT_OUTPUT_DATA_BUFFER {
            pSample: ManuallyDrop::new(sample),
            ..Default::default()
        };
        let mut status = 0;
        let result = transform.ProcessOutput(0, std::slice::from_mut(&mut buffer), &mut status);
        // 在匹配 HRESULT 前取回所有 COM 所有权，包含 NEED_MORE_INPUT/STREAM_CHANGE 分支。
        drop(ManuallyDrop::take(&mut buffer.pEvents));
        let sample = ManuallyDrop::take(&mut buffer.pSample);
        match result {
            Ok(()) => {
                Ok(Output::Frame(sample.ok_or_else(|| {
                    Error::new(E_INVALIDARG, "Decoder output missing")
                })?))
            }
            Err(e) if e.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => Ok(Output::NeedInput),
            Err(e) if e.code() == MF_E_TRANSFORM_STREAM_CHANGE => Ok(Output::FormatChanged),
            Err(e) => Err(e),
        }
    }
}

//! 线程绑定的同步系统H264编码器；单输入/输出，失败即退休，不跨帧积压sample。
use super::{samples, Runtime};
use crate::byte_budget::{Budget, Lease};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use windows::{
    core::{Error, Interface, Result},
    Win32::{
        Foundation::E_INVALIDARG,
        Media::MediaFoundation::*,
        System::{Com::*, Variant::VARIANT},
    },
};
#[path = "live_mf_encode_format.rs"]
mod format;
#[path = "live_mf_encode_pixels.rs"]
mod pixels;
#[cfg(test)]
#[path = "live_mf_encode_tests.rs"]
mod tests;
const MAX_AU: usize = 1024 * 1024;
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Permit;
impl Permit {
    fn acquire() -> Result<Self> {
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 4).then_some(n + 1)
            })
            .map_err(|_| invalid("Native encoder capacity exhausted"))?;
        Ok(Self)
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Transform(IMFTransform);
impl Drop for Transform {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0);
            let _ = self.0.ProcessMessage(MFT_MESSAGE_NOTIFY_END_STREAMING, 0);
        }
    }
}
struct Configured {
    transform: Transform,
    output_max: u32,
    _runtime: Runtime,
    _permit: Permit,
    _buffers: Lease,
}
pub(crate) struct VideoEncoder {
    configured: Option<Configured>,
    budget: Arc<Budget>,
    size: [u32; 2],
    last_time: Option<i64>,
}
/// 编码Vec自己的租约晚于字节释放，不能随codec提前归还。
pub(crate) struct Encoded {
    pub bytes: Vec<u8>,
    pub keyframe: bool,
    _lease: Lease,
}
impl VideoEncoder {
    pub fn create(size: [u32; 2], budget: &Arc<Budget>) -> Result<Self> {
        let nv12 = format::validate_size(size)?;
        // 为显式NV12、最多四份参考面及有界MFT输出留额度；不声称控制系统codec的所有内部堆。
        let mut buffers = budget
            .reserve(nv12 * 5 + MAX_AU)
            .ok_or_else(|| invalid("Encoder buffer budget exhausted"))?;
        let permit = Permit::acquire()?;
        let runtime = Runtime::start()?;
        let transform =
            Transform(unsafe { CoCreateInstance(&CMSH264EncoderMFT, None, CLSCTX_INPROC_SERVER)? });
        let output_max = format::configure(&transform.0, size)?;
        // 系统1080p声明约3.9MiB输出buffer，和1MiB wire AU不是同一个上限。
        if !buffers.grow_to(nv12 * 5 + output_max as usize) {
            return Err(invalid("Encoder reported output buffer budget exhausted"));
        }
        Ok(Self {
            configured: Some(Configured {
                transform,
                output_max,
                _runtime: runtime,
                _permit: permit,
                _buffers: buffers,
            }),
            budget: budget.clone(),
            size,
            last_time: None,
        })
    }
    pub fn encode(
        &mut self,
        bgra: &[u8],
        timestamp: i64,
        force: bool,
        stop: &AtomicBool,
    ) -> Result<Encoded> {
        let result = self.encode_inner(bgra, timestamp, force, stop);
        #[cfg(test)]
        if let Err(error) = &result {
            eprintln!("source-video-encode-error: {error:?}");
        }
        if result.is_err() {
            self.configured.take();
        }
        result
    }
    fn encode_inner(
        &mut self,
        bgra: &[u8],
        timestamp: i64,
        force: bool,
        stop: &AtomicBool,
    ) -> Result<Encoded> {
        check_stop(stop)?;
        if timestamp < 0 || self.last_time.is_some_and(|t| timestamp <= t) {
            return Err(invalid("Encoder timestamp not monotonic"));
        }
        let configured = self
            .configured
            .as_ref()
            .ok_or_else(|| invalid("Encoder retired"))?;
        let transform = &configured.transform.0;
        let force = force || self.last_time.is_none();
        let mut lease = self
            .budget
            .reserve(MAX_AU)
            .ok_or_else(|| invalid("Encoded AU budget exhausted"))?;
        unsafe {
            if force {
                let api: ICodecAPI = transform.cast()?;
                api.SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &VARIANT::from(1u32))?;
            }
            let sample = pixels::input(bgra, self.size, timestamp, stop)?;
            check_stop(stop)?;
            transform.ProcessInput(0, &sample, 0)?;
            self.last_time = Some(timestamp);
            // bool low-latency必须得到同一timestamp的同步输出；无输出不缓存/错配成下一帧。
            let samples::Output::Frame(output) =
                samples::receive_bounded(transform, configured.output_max)?
            else {
                return Err(invalid("Encoder did not produce immediate bounded output"));
            };
            if output.GetSampleTime()? != timestamp || output.GetBufferCount()? != 1 {
                return Err(invalid("Encoder output identity mismatch"));
            }
            let keyframe = output.GetUINT32(&MFSampleExtension_CleanPoint).unwrap_or(0) != 0;
            if force && !keyframe {
                return Err(invalid("Encoder ignored IDR request"));
            }
            let buffer = output.GetBufferByIndex(0)?;
            let (mut ptr, mut capacity, mut length) = (std::ptr::null_mut(), 0, 0);
            buffer.Lock(&mut ptr, Some(&mut capacity), Some(&mut length))?;
            let _lock = samples::Locked(&buffer);
            if ptr.is_null()
                || length == 0
                || length > capacity
                || length as usize > MAX_AU
                || capacity > configured.output_max
            {
                return Err(invalid("Encoder output buffer bounds"));
            }
            let source = std::slice::from_raw_parts(ptr, length as usize);
            crate::live_h264::validate(source, self.size[0], self.size[1], keyframe)
                .map_err(invalid)?;
            check_stop(stop)?;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(source.len())
                .map_err(|_| invalid("Encoded AU allocation failed"))?;
            if !lease.grow_to(bytes.capacity()) {
                return Err(invalid("Encoded AU capacity budget exhausted"));
            }
            bytes.extend_from_slice(source);
            Ok(Encoded {
                bytes,
                keyframe,
                _lease: lease,
            })
        }
    }
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}
fn check_stop(stop: &AtomicBool) -> Result<()> {
    if stop.load(Ordering::Acquire) {
        Err(invalid("Video encoding cancelled"))
    } else {
        Ok(())
    }
}

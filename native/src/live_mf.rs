//! 线程绑定的系统 H264 decoder owner；先释放 transform，再平衡 MF/COM 和进程名额。
use std::{
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::E_FAIL, Media::MediaFoundation::*, System::Com::*},
};
static DECODER_ACTIVE: AtomicBool = AtomicBool::new(false);
#[path = "live_mf_decode.rs"]
mod decode;
#[path = "live_mf_encode.rs"]
mod encode;
#[path = "live_mf_format.rs"]
mod format;
#[path = "live_mf_layout.rs"]
mod layout;
#[path = "live_mf_pixels.rs"]
mod pixels;
#[path = "live_mf_samples.rs"]
mod samples;
pub use decode::{Decoded, VideoDecoder};
pub(crate) use encode::{Encoded, VideoEncoder};
#[cfg(test)]
#[path = "live_mf_sequence_tests.rs"]
mod sequence_tests;
pub use format::ConfiguredDecoder;
pub use layout::Layout;
// E_PENDING；不为一个 HRESULT 启用无关 Urlmon feature。
const E_PENDING: windows::core::HRESULT = windows::core::HRESULT(0x8000_000Au32 as i32);
struct Permit;
impl Permit {
    fn acquire() -> Result<Self> {
        DECODER_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| Error::new(E_PENDING, "Native video decoder already in use"))?;
        Ok(Self)
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        DECODER_ACTIVE.store(false, Ordering::Release);
    }
}

struct Runtime(PhantomData<Rc<()>>);
impl Runtime {
    fn start() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            if let Err(error) = MFStartup(MF_VERSION, MFSTARTUP_FULL) {
                CoUninitialize();
                return Err(error);
            }
        }
        Ok(Self(PhantomData))
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe {
            let _ = MFShutdown();
            CoUninitialize();
        }
    }
}
#[derive(Default)]
struct Activations {
    ptr: *mut Option<IMFActivate>,
    count: u32,
}
impl Drop for Activations {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                // ptr/count 为 Windows MFTEnumEx 返回的成对分配，不来自远程媒体。
                for slot in std::slice::from_raw_parts_mut(self.ptr, self.count as usize) {
                    drop(slot.take());
                }
                CoTaskMemFree(Some(self.ptr.cast()));
            }
        }
    }
}
struct Transform {
    transform: IMFTransform,
    activation: IMFActivate,
}
impl Drop for Transform {
    fn drop(&mut self) {
        unsafe {
            let _ = self.transform.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0);
            let _ = self
                .transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_END_STREAMING, 0);
            let _ = self.activation.ShutdownObject();
        }
    }
}
/// 必须在独立 codec 线程上创建与销毁；Rc marker 禁止跨线程搬运 COM/MF 生命周期。
pub struct DecoderOwner {
    _transform: Transform,
    _runtime: Runtime,
    _permit: Permit,
}
impl DecoderOwner {
    /// 仅枚举并激活同步系统 decoder；后续必须协商输入/输出格式再提交 AU。
    pub fn create() -> Result<Self> {
        let permit = Permit::acquire()?;
        let runtime = Runtime::start()?;
        let mut list = Activations::default();
        unsafe {
            let input = MFT_REGISTER_TYPE_INFO {
                guidMajorType: MFMediaType_Video,
                guidSubtype: MFVideoFormat_H264,
            };
            let output = MFT_REGISTER_TYPE_INFO {
                guidMajorType: MFMediaType_Video,
                guidSubtype: MFVideoFormat_NV12,
            };
            MFTEnumEx(
                MFT_CATEGORY_VIDEO_DECODER,
                MFT_ENUM_FLAG_SYNCMFT | MFT_ENUM_FLAG_SORTANDFILTER,
                Some(&input),
                Some(&output),
                &mut list.ptr,
                &mut list.count,
            )?;
            if list.ptr.is_null() || list.count == 0 || list.count > 64 {
                return Err(Error::new(
                    E_FAIL,
                    "No bounded synchronous H264 decoder inventory",
                ));
            }
            let activation = (*list.ptr)
                .as_ref()
                .ok_or_else(|| Error::new(E_FAIL, "Empty decoder activation"))?
                .clone();
            let transform = match activation.ActivateObject() {
                Ok(transform) => transform,
                Err(error) => {
                    let _ = activation.ShutdownObject();
                    return Err(error);
                }
            };
            Ok(Self {
                _transform: Transform {
                    transform,
                    activation,
                },
                _runtime: runtime,
                _permit: permit,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decoder_permit_rejects_overlap_and_releases_on_unwind() {
        let permit = Permit::acquire().unwrap();
        assert_eq!(Permit::acquire().err().unwrap().code(), E_PENDING);
        drop(permit);
        let _ = std::panic::catch_unwind(|| {
            let _permit = Permit::acquire().unwrap();
            panic!("fixture permit unwind");
        });
        drop(Permit::acquire().unwrap());
    }
    #[test]
    #[ignore = "requires installed Windows Media Foundation H264 decoder; no screen capture or network"]
    fn actual_system_decoder_activation_drop_and_permit_reuse() {
        for _ in 0..3 {
            let owner = DecoderOwner::create().unwrap();
            assert_eq!(DecoderOwner::create().err().unwrap().code(), E_PENDING);
            let (mut input, mut output) = (0, 0);
            unsafe {
                owner
                    ._transform
                    .transform
                    .GetStreamCount(&mut input, &mut output)
                    .unwrap();
            }
            assert_eq!((input, output), (1, 1));
            drop(owner);
        }
        eprintln!("native-mf-decoder-owner: activated=3 dropped=3 overlap-rejected=true");
    }
}

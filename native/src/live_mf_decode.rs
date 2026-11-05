//! 有状态 AU 提交与同步输出；失败立即销毁 codec，返回值只含受预算约束的像素。
use super::{
    format, pixels,
    samples::{self, Output},
    ConfiguredDecoder,
};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};
pub struct Decoded {
    pub timestamp: i64,
    pub width: u32,
    pub height: u32,
    pub pixels: crate::pixels::Pixels,
}
pub struct VideoDecoder {
    configured: Option<ConfiguredDecoder>,
    last_input: Option<i64>,
    pending: VecDeque<i64>,
}
impl ConfiguredDecoder {
    pub fn into_video(self) -> VideoDecoder {
        VideoDecoder {
            configured: Some(self),
            last_input: None,
            pending: VecDeque::new(),
        }
    }
}
impl VideoDecoder {
    pub fn decode(
        &mut self,
        bytes: &[u8],
        timestamp: i64,
        keyframe: bool,
        stop: &AtomicBool,
    ) -> Result<Option<Decoded>> {
        let result = self.decode_inner(bytes, timestamp, keyframe, stop);
        if result.is_err() {
            // 失效的参考链绝不复用；COM/MF 及进程名额在当前 owner 线程立即归还。
            self.configured.take();
            self.pending.clear();
        }
        result
    }
    fn decode_inner(
        &mut self,
        bytes: &[u8],
        timestamp: i64,
        keyframe: bool,
        stop: &AtomicBool,
    ) -> Result<Option<Decoded>> {
        check_stop(stop)?;
        let configured = self
            .configured
            .as_ref()
            .ok_or_else(|| invalid("Decoder reference chain failed"))?;
        if (self.last_input.is_none() && !keyframe)
            || timestamp < 0
            || self.last_input.is_some_and(|v| timestamp <= v)
            || self.pending.len() >= 4
        {
            return Err(invalid("Decoder input timeline or queue invalid"));
        }
        let layout = configured.layout();
        crate::live_h264::validate(bytes, layout.visible_width, layout.visible_height, keyframe)
            .map_err(invalid)?;
        let transform = &configured.owner._transform.transform;
        let sample = samples::input(bytes, timestamp, keyframe)?;
        unsafe {
            transform.ProcessInput(0, &sample, 0)?;
        }
        self.last_input = Some(timestamp);
        self.pending.push_back(timestamp);
        for _ in 0..3 {
            check_stop(stop)?;
            match samples::receive(transform)? {
                Output::NeedInput => return Ok(None),
                Output::FormatChanged => {
                    format::select_output(transform, layout.visible_width, layout.visible_height)?;
                }
                Output::Frame(sample) => unsafe {
                    let timestamp = sample.GetSampleTime()?;
                    if self.pending.pop_front() != Some(timestamp) {
                        return Err(invalid("Decoder output reordered"));
                    }
                    let media = transform.GetOutputCurrentType(0)?;
                    let current =
                        super::Layout::read(&media, layout.visible_width, layout.visible_height)?;
                    let pixels = pixels::decode(&sample, &media, current)?;
                    check_stop(stop)?;
                    return Ok(Some(Decoded {
                        timestamp,
                        width: layout.visible_width,
                        height: layout.visible_height,
                        pixels,
                    }));
                },
            }
        }
        Err(invalid("Decoder repeated format changes"))
    }
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}
fn check_stop(stop: &AtomicBool) -> Result<()> {
    if stop.load(Ordering::Acquire) {
        Err(invalid("Video decoding cancelled"))
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires explicit synthetic H264 fixture directory and system decoder"]
    fn actual_h264_fixture_decodes_pixels_and_rejects_replayed_timestamp() {
        let root = std::env::var("OTTER_NATIVE_H264_FIXTURES")
            .expect("explicit synthetic fixture directory");
        for (name, width, height) in [("red64.h264", 64, 64), ("red680.h264", 680, 430)] {
            let bytes = std::fs::read(std::path::Path::new(&root).join(name)).unwrap();
            let mut decoder = super::super::DecoderOwner::create()
                .unwrap()
                .configure(width, height)
                .unwrap()
                .into_video();
            let stop = AtomicBool::new(false);
            // 同步 decoder 可延迟首帧；仍限定最多四个在途描述符，不用 drain 伪装实时输出。
            let mut output = None;
            for timestamp in 1..=4 {
                output = decoder.decode(&bytes, timestamp, true, &stop).unwrap();
                if output.is_some() {
                    break;
                }
            }
            let output = output.expect("bounded IDR output");
            assert_eq!(
                (output.timestamp, output.width, output.height),
                (1, width, height)
            );
            assert_eq!(output.pixels.len(), width as usize * height as usize * 4);
            for pixel in output.pixels.chunks_exact(4) {
                assert!(
                    pixel[2] > 200 && pixel[0] < 40 && pixel[1] < 40 && pixel[3] == 255,
                    "unexpected red pixel: {pixel:?}"
                );
            }
            assert!(decoder.decode(&bytes, 1, true, &stop).is_err());
            drop(super::super::DecoderOwner::create().unwrap());
            // 输出只保留像素租约，在 codec 已销毁后仍然有效。
            assert_eq!(output.pixels[3], 255);
            eprintln!("native-mf-decode: {width}x{height} red-pixels=true replay-rejected=true owner-released=true");
        }
    }
    #[test]
    #[ignore = "requires system H264 decoder; verifies fail-closed and cancellation, not valid video"]
    fn actual_invalid_au_and_cancel_release_decoder_immediately() {
        for cancelled in [false, true] {
            let mut decoder = super::super::DecoderOwner::create()
                .unwrap()
                .configure(64, 64)
                .unwrap()
                .into_video();
            let stop = AtomicBool::new(cancelled);
            assert!(decoder.decode(&[0, 0, 1, 0x65, 0], 1, true, &stop).is_err());
            assert!(decoder.configured.is_none());
            let replacement = super::super::DecoderOwner::create().unwrap();
            assert!(decoder
                .decode(&[0, 0, 1, 0x65, 0], 2, true, &AtomicBool::new(false))
                .is_err());
            drop(replacement);
        }
    }
}

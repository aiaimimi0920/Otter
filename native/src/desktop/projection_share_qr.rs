//! QR 编码只读签名 JSON；PNG 解码单槽保有 worker 至真实退出，取消不无限增殖后台任务。
use super::*;
use crate::desktop::loom_execution_signal::{reap_deadline, CompletionSignal, Notify};
use std::{io::Read, path::PathBuf, thread::JoinHandle};
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "projection_share_qr_tests.rs"]
mod tests;
pub(super) struct Code {
    pub size: usize,
    pub colors: Vec<qrcode::Color>,
}
pub(super) fn encode_bytes(bytes: &[u8]) -> std::result::Result<Code, String> {
    let code = qrcode::QrCode::with_error_correction_level(bytes, qrcode::EcLevel::M)
        .map_err(|_| "邀请超出二维码容量，请使用链接".to_owned())?;
    Ok(Code {
        size: code.width(),
        colors: code.into_colors(),
    })
}
pub(in crate::desktop) struct Decode {
    thread: Option<JoinHandle<std::result::Result<String, &'static str>>>,
    target: Weak<Model>,
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    epoch: u64,
}
impl Decode {
    pub(super) fn start(path: PathBuf, target: &Rc<Model>) -> Result<Self> {
        let thread_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
        Self::start_with_notify(
            path,
            target,
            Arc::new(move || unsafe {
                let _ = PostThreadMessageW(thread_id, WM_NULL, WPARAM(0), LPARAM(0));
            }),
        )
    }
    pub(super) fn start_with_notify(
        path: PathBuf,
        target: &Rc<Model>,
        notify: Notify,
    ) -> Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let complete = ready.clone();
        let thread = std::thread::Builder::new()
            .name("otter-share-qr".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(complete, notify);
                decode(path, &signal)
            })
            .map_err(|_| Error::new(E_FAIL, "Cannot start QR decoder"))?;
        *target.decode_cancel.borrow_mut() = Some(cancel.clone());
        Ok(Self {
            thread: Some(thread),
            target: Rc::downgrade(target),
            cancel,
            ready,
            epoch: target.input_epoch.get(),
        })
    }
    pub(in crate::desktop) fn deadline(&self) -> Option<std::time::Instant> {
        self.thread.as_ref()?;
        reap_deadline(&self.ready)
    }
    pub(super) fn poll(
        &mut self,
    ) -> Option<(Weak<Model>, u64, std::result::Result<String, &'static str>)> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        Some((
            self.target.clone(),
            self.epoch,
            self.thread.take()?.join().unwrap_or(Err("二维码解码失败")),
        ))
    }
}
impl Drop for Decode {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.cancel.store(true, Ordering::Release);
            let _ = thread.join();
        }
    }
}
fn decode(path: PathBuf, cancel: &AtomicBool) -> std::result::Result<String, &'static str> {
    use rxing::{
        common::HybridBinarizer, qrcode::QRCodeReader, BinaryBitmap, DecodeHints, Luma8Source,
        Reader,
    };
    let check = || {
        if cancel.load(Ordering::Acquire) {
            Err("二维码识别已取消")
        } else {
            Ok(())
        }
    };
    check()?;
    let file = std::fs::File::open(path).map_err(|_| "无法读取二维码图片")?;
    const LIMIT: usize = 4 * 1024 * 1024;
    if file.metadata().map_err(|_| "无法读取图片信息")?.len() > LIMIT as u64 {
        return Err("二维码 PNG 不能超过 4 MiB");
    }
    let _input_budget = crate::pixels::reserve(LIMIT + 1).ok_or("图像内存不足")?;
    let mut bytes = Vec::with_capacity(LIMIT + 1);
    file.take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取二维码图片")?;
    if bytes.len() > LIMIT
        || bytes.len() < 33
        || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || &bytes[12..16] != b"IHDR"
    {
        return Err("请选择有效的 PNG 图片");
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err("二维码图片边长不能超过 4096");
    }
    let _com = crate::desktop::Apartment::new().map_err(|_| "无法初始化图像解码")?;
    let image = crate::image_decode::Image::from_projection_png(&bytes, width, height, &|| {
        cancel.load(Ordering::Acquire)
    })
    .map_err(|_| "PNG 图片解码失败")?;
    let ratio = f64::from(width.max(height)).max(1536.0) / 1536.0;
    let w = (f64::from(width) / ratio).round().max(1.0) as u32;
    let h = (f64::from(height) / ratio).round().max(1.0) as u32;
    // 额外预留 QR 矩阵/阈值工作空间；库内部无可中断接口，单槽直到完成才归还。
    let _decode_budget =
        crate::pixels::reserve(w as usize * h as usize * 8).ok_or("图像内存不足")?;
    let mut luma = Vec::with_capacity(w as usize * h as usize);
    for y in 0..h {
        check()?;
        for x in 0..w {
            let sx = u64::from(x) * u64::from(width) / u64::from(w);
            let sy = u64::from(y) * u64::from(height) / u64::from(h);
            let i = (sy * u64::from(width) + sx) as usize * 4;
            let p = &image.pixels[i..i + 4];
            let gray =
                (77 * u32::from(p[2]) + 150 * u32::from(p[1]) + 29 * u32::from(p[0]) + 128) >> 8;
            luma.push((gray + 255 - u32::from(p[3])).min(255) as u8);
        }
    }
    check()?;
    let source = Luma8Source::new_with_slice(&luma, w, h).map_err(|_| "二维码像素无效")?;
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));
    let hints = DecodeHints {
        CharacterSet: Some("UTF-8".into()),
        ..Default::default()
    };
    let result = match QRCodeReader::new().decode_with_hints(&mut bitmap, &hints) {
        Ok(result) => result,
        Err(_) => {
            check()?;
            // 导出的密集方形码可能被定位器漏掉；仅再尝试库内的无旋转纯码提取。
            let hints = DecodeHints {
                PureBarcode: Some(true),
                ..hints
            };
            QRCodeReader::new()
                .decode_with_hints(&mut bitmap, &hints)
                .map_err(|_| "图片中未识别到投射二维码")?
        }
    };
    check()?;
    if result.getText().len() > MAX_TEXT {
        return Err("二维码内容过长");
    }
    if crate::projection_account::Envelope::parse(result.getText()).is_err()
        && crate::projection_delivery::share::parse(result.getText()).is_err()
    {
        return Err("不是有效的投射邀请");
    }
    Ok(result.getText().to_owned())
}

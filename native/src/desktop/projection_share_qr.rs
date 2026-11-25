//! QR encoding reads only signed JSON; single-slot PNG decoding retains its worker until actual exit, so cancellation cannot spawn unbounded background tasks.
use super::*;
use crate::desktop::loom_execution_signal::{reap_deadline, CompletionSignal, Notify};
use crate::ui_text::text as tr;
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
        .map_err(|_| tr("share-qr-capacity").to_owned())?;
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
            self.thread
                .take()?
                .join()
                .unwrap_or(Err(tr("share-qr-decode-failed"))),
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
            Err(tr("share-qr-cancelled"))
        } else {
            Ok(())
        }
    };
    check()?;
    let file = std::fs::File::open(path).map_err(|_| tr("share-qr-read-failed"))?;
    const LIMIT: usize = 4 * 1024 * 1024;
    if file
        .metadata()
        .map_err(|_| tr("share-qr-metadata-failed"))?
        .len()
        > LIMIT as u64
    {
        return Err(tr("share-qr-size-limit"));
    }
    let _input_budget = crate::pixels::reserve(LIMIT + 1).ok_or(tr("share-image-memory"))?;
    let mut bytes = Vec::with_capacity(LIMIT + 1);
    file.take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| tr("share-qr-read-failed"))?;
    if bytes.len() > LIMIT
        || bytes.len() < 33
        || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || &bytes[12..16] != b"IHDR"
    {
        return Err(tr("share-qr-invalid-png"));
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(tr("share-qr-dimensions"));
    }
    let _com = crate::desktop::Apartment::new().map_err(|_| tr("share-image-init-failed"))?;
    let image = crate::image_decode::Image::from_projection_png(&bytes, width, height, &|| {
        cancel.load(Ordering::Acquire)
    })
    .map_err(|_| tr("share-png-decode-failed"))?;
    let ratio = f64::from(width.max(height)).max(1536.0) / 1536.0;
    let w = (f64::from(width) / ratio).round().max(1.0) as u32;
    let h = (f64::from(height) / ratio).round().max(1.0) as u32;
    // Reserve extra QR-matrix/threshold workspace; the library has no interruptible interface, so the slot remains occupied until completion.
    let _decode_budget =
        crate::pixels::reserve(w as usize * h as usize * 8).ok_or(tr("share-image-memory"))?;
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
    let source =
        Luma8Source::new_with_slice(&luma, w, h).map_err(|_| tr("share-qr-pixels-invalid"))?;
    let mut bitmap = BinaryBitmap::new(HybridBinarizer::new(source));
    let hints = DecodeHints {
        CharacterSet: Some("UTF-8".into()),
        ..Default::default()
    };
    let result = match QRCodeReader::new().decode_with_hints(&mut bitmap, &hints) {
        Ok(result) => result,
        Err(_) => {
            check()?;
            // The locator may miss exported dense square codes; retry only the library's unrotated pure-code extraction.
            let hints = DecodeHints {
                PureBarcode: Some(true),
                ..hints
            };
            QRCodeReader::new()
                .decode_with_hints(&mut bitmap, &hints)
                .map_err(|_| tr("share-qr-not-found"))?
        }
    };
    check()?;
    if result.getText().len() > MAX_TEXT {
        return Err(tr("share-qr-text-limit"));
    }
    if crate::projection_account::Envelope::parse(result.getText()).is_err()
        && crate::projection_delivery::share::parse(result.getText()).is_err()
    {
        return Err(tr("share-invalid-invitation"));
    }
    Ok(result.getText().to_owned())
}

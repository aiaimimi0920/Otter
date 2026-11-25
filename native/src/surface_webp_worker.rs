//! Private WebP renderer: format parsing/ICC/PBGRA; the image worker host owns restricted pipes and frame validation.
use crate::{
    surface_image_worker_host::{self as host, Decoded},
    surface_webp_header::{inspect, invalid, MAX_INPUT},
};
use std::{ffi::OsString, io};
pub const FLAG: &str = "--native-webp-worker";
pub const SESSION_FLAG: &str = "--native-webp-session-worker";
#[path = "surface_webp_color.rs"]
mod color;
pub fn parse(args: &[OsString]) -> io::Result<bool> {
    if !args.iter().any(|arg| arg == FLAG) {
        return Ok(false);
    }
    if args.len() != 1 || args[0] != FLAG {
        return Err(invalid());
    }
    Ok(true)
}
pub fn parse_session(args: &[OsString]) -> io::Result<bool> {
    if !args.iter().any(|arg| arg == SESSION_FLAG) {
        return Ok(false);
    }
    if args.len() != 1 || args[0] != SESSION_FLAG {
        return Err(invalid());
    }
    Ok(true)
}
pub fn run_inherited() -> io::Result<()> {
    host::run(decode)
}
pub fn run_session_inherited() -> io::Result<()> {
    host::run_session_for(crate::surface_image_worker_wire::Codec::WebP, decode)
}
fn decode(bytes: &[u8]) -> io::Result<Decoded> {
    let (width, height) = inspect(bytes)?;
    // Ordinary Surface's Chromium WebP baseline ignores EXIF orientation; do not apply JPEG/TIFF rotation rules.
    let mut decoder =
        image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).map_err(io::Error::other)?;
    if decoder.is_animated() || decoder.dimensions() != (width, height) {
        return Err(invalid());
    }
    let channels = if decoder.has_alpha() { 4 } else { 3 };
    let count = crate::viewport::checked_image_bytes(width, height).ok_or_else(invalid)?;
    let len = (count / 4) * channels;
    if decoder.output_buffer_size() != Some(len) {
        return Err(invalid());
    }
    // This API does not limit static working memory; the parent process Job provides the actual hard boundary.
    decoder.set_memory_limit(MAX_INPUT);
    let mut pixels = vec![0u8; count];
    decoder
        .read_image(&mut pixels[..len])
        .map_err(io::Error::other)?;
    color::apply(bytes, &mut pixels[..len], channels)?;
    // Expand RGB from the tail or premultiply RGBA in place without another full-image PBGRA allocation.
    for index in (0..count / 4).rev() {
        let at = index * channels;
        let (r, g, b) = (pixels[at], pixels[at + 1], pixels[at + 2]);
        let alpha = if channels == 4 { pixels[at + 3] } else { 255 };
        let premul = |c: u8| ((u16::from(c) * u16::from(alpha) + 127) / 255) as u8;
        pixels[index * 4..index * 4 + 4].copy_from_slice(&[premul(b), premul(g), premul(r), alpha]);
    }
    Ok(Decoded {
        logical_size: [width as f32, height as f32],
        width,
        height,
        pixels,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::live_recovery_pipe::Pipe;
    use std::{
        io::Write,
        os::windows::io::AsHandle,
        time::{Duration, Instant},
    };
    #[test]
    fn surface_webp_private_mode_is_exclusive() {
        assert!(parse(&[FLAG.into()]).unwrap());
        assert!(!parse(&["--smoke-test".into()]).unwrap());
        assert!(parse(&[FLAG.into(), "--settings".into(), "secret".into()]).is_err());
    }
    #[test]
    #[ignore = "private bounded memory probe invoked by parent"]
    fn webp_memory_entry() {
        host::require_limits().unwrap();
        use windows::Win32::System::Memory::*;
        let denied = unsafe {
            let value = VirtualAlloc(
                None,
                129 * 1024 * 1024,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            );
            if value.is_null() {
                true
            } else {
                let _ = VirtualFree(value, 0, MEM_RELEASE);
                false
            }
        };
        let mut pipe =
            Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned().unwrap()).unwrap();
        pipe.io(Some(Instant::now() + Duration::from_secs(5)), None)
            .write_all(if denied { b"L" } else { b"F" })
            .unwrap();
        std::process::exit(if denied { 0 } else { 1 });
    }
    #[test]
    #[ignore = "private stalled peer invoked by parent cancellation tests"]
    fn webp_stall_entry() {
        host::require_limits().unwrap();
        std::thread::sleep(Duration::from_secs(60));
        std::process::exit(1);
    }
    #[test]
    #[ignore = "private pipe child entry; invoked by bounded parent tests"]
    fn webp_worker_entry() {
        std::process::exit(if super::run_inherited().is_ok() { 0 } else { 1 });
    }
    #[test]
    #[ignore = "private session pipe child entry; invoked by bounded parent tests"]
    fn webp_session_worker_entry() {
        std::process::exit(if super::run_session_inherited().is_ok() {
            0
        } else {
            1
        });
    }
}

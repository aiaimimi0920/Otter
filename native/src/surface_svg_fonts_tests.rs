//! 字体来源沿实际共享 launcher 进入 Job，私有测试入口不进入正式 CLI。
use super::*;
#[path = "surface_svg_font_fallback_tests.rs"]
mod fallback_checks;
use crate::live_recovery_pipe::Pipe;
use std::{
    io::{Read, Write},
    os::windows::io::{AsHandle, AsRawHandle},
    time::{Duration, Instant},
};
#[test]
fn svg_fonts_require_job_and_load_bounded_faces_in_real_child() {
    assert!(Fonts::new().is_err());
    let (mut pipe, endpoint) = Pipe::pair().unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let child = crate::surface_image_worker_process::start(
        endpoint.into_handle(),
        deadline,
        &|| false,
        " --exact surface_svg_worker::fonts::tests::svg_font_entry --ignored --nocapture",
    )
    .unwrap();
    let handle = child.handle().try_clone_to_owned().unwrap();
    let mut result = [0; 4];
    pipe.io(Some(deadline), Some(child.handle()))
        .read_exact(&mut result)
        .unwrap();
    assert_eq!(&result, b"FONT");
    let mut length = [0; 4];
    pipe.io(Some(deadline), Some(child.handle()))
        .read_exact(&mut length)
        .unwrap();
    let length = u32::from_le_bytes(length) as usize;
    assert!((1..=8192).contains(&length));
    let mut bytes = vec![0; length];
    pipe.io(Some(deadline), Some(child.handle()))
        .read_exact(&mut bytes)
        .unwrap();
    let records: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(records.len(), 8);
    println!(
        "svg-font-fallback-baseline={}",
        String::from_utf8(bytes).unwrap()
    );
    child.finish(deadline, &|| false).unwrap();
    drop(child);
    assert_eq!(
        unsafe {
            windows::Win32::System::Threading::WaitForSingleObject(
                windows::Win32::Foundation::HANDLE(handle.as_raw_handle()),
                0,
            )
        },
        windows::Win32::Foundation::WAIT_OBJECT_0
    );
}
fn child_check() -> io::Result<()> {
    let records = fallback_checks::run()?;
    let mut fonts = Fonts::new()?;
    assert_eq!(fonts.bytes, 0);
    assert!(fonts
        .select("Otter missing font sentinel", 400, 5, 0)?
        .is_none());
    let normal = fonts
        .select("Microsoft YaHei", 400, 5, 0)?
        .ok_or_else(file::invalid)?;
    let ui = fonts
        .select("Microsoft YaHei UI", 400, 5, 0)?
        .ok_or_else(file::invalid)?;
    assert_eq!(normal.index, 0);
    assert_eq!(ui.index, 1);
    assert!(Arc::ptr_eq(&normal.bytes, &ui.bytes));
    let sun = fonts
        .select("SimSun", 400, 5, 0)?
        .ok_or_else(file::invalid)?;
    let bold = fonts
        .select("SimSun", 700, 5, 0)?
        .ok_or_else(file::invalid)?;
    assert!(Arc::ptr_eq(&sun.bytes, &bold.bytes));
    assert_eq!(bold.simulations, DWRITE_FONT_SIMULATIONS_BOLD.0 as u32);
    assert_eq!(fonts.files.len(), 2);
    assert!(fonts.bytes <= MAX_BYTES);
    for (family, weight, stretch, style) in [
        ("a\0b", 400, 5, 0),
        ("", 400, 5, 0),
        ("Arial", 0, 5, 0),
        ("Arial", 400, 0, 0),
        ("Arial", 400, 5, 3),
    ] {
        assert!(fonts.select(family, weight, stretch, style).is_err());
    }
    fonts.queries = MAX_QUERIES;
    assert!(fonts.select("Arial", 400, 5, 0).is_err());
    // 人为耗尽剩余预算，再请求尚未加载的字体，不能分配后才报超限。
    fonts.queries = 0;
    fonts.bytes = MAX_BYTES;
    assert!(fonts.select("Arial", 400, 5, 0).is_err());
    let mut pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let bytes = serde_json::to_vec(&records)?;
    assert!(bytes.len() <= 8192);
    let mut channel = pipe.io(Some(Instant::now() + Duration::from_secs(5)), None);
    channel.write_all(b"FONT")?;
    channel.write_all(&(bytes.len() as u32).to_le_bytes())?;
    channel.write_all(&bytes)
}
#[test]
#[ignore = "private font child invoked by bounded parent test"]
fn svg_font_entry() {
    std::process::exit(if child_check().is_ok() { 0 } else { 1 });
}

//! 实际受限 peer 注入 SVG v2 尺寸故障，复用生产接收门并验证进程回收。
use super::*;
use std::os::windows::io::{AsHandle, AsRawHandle};
use windows::Win32::{Foundation::*, System::Threading::WaitForSingleObject};
#[test]
fn surface_svg_metadata_rejects_untrusted_sizes_and_reaps_child() {
    for mode in 0..=9 {
        let mut handle = None;
        let result = run_sized(
            &[mode],
            wire::Codec::Svg,
            None,
            &|| false,
            |endpoint, deadline| {
                let child = process::start(endpoint, deadline, &|| false,
                " --exact image_decode::surface::worker_client::tests::svg_metadata_peer --ignored --nocapture")?;
                handle = Some(child.handle().try_clone_to_owned()?);
                Ok(child)
            },
        );
        assert_eq!(result.is_ok(), mode == 0, "mode={mode}");
        if let Ok((image, size)) = result {
            assert_eq!(size, [2.5, 1.5]);
            assert_eq!((image.width, image.height), (3, 2));
            assert_eq!(&*image.pixels, &[255; 24]);
        }
        let handle = handle.unwrap();
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 0) },
            WAIT_OBJECT_0
        );
    }
}
#[test]
#[ignore = "private SVG malformed metadata peer invoked by parent"]
fn svg_metadata_peer() {
    std::process::exit(if reply().is_ok() { 0 } else { 1 });
}
fn reply() -> io::Result<()> {
    crate::surface_image_worker_host::require_limits()?;
    let mut pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let mut channel = pipe.io(Some(Instant::now() + Duration::from_secs(5)), None);
    let mut request = [0; 8];
    channel.read_exact(&mut request)?;
    if wire::request_for(wire::Codec::Svg, &request)? != 1 {
        return Err(invalid());
    }
    let mut mode = [0];
    channel.read_exact(&mut mode)?;
    let mode = mode[0];
    channel.write_all(if mode == 8 { b"OSP1" } else { b"OSP2" })?;
    for value in [3u32, 2, 24] {
        channel.write_all(&value.to_le_bytes())?;
    }
    if mode == 9 {
        return Ok(());
    }
    let size: [f32; 2] = match mode {
        1 => [f32::NAN, 1.5],
        2 => [f32::INFINITY, 1.5],
        3 => [0., 1.5],
        4 => [-2.5, 1.5],
        5 => [3.1, 1.5],
        6 => [1.5, 1.5],
        7 => [f32::from_bits(1), 1.5],
        _ => [2.5, 1.5],
    };
    for value in size {
        channel.write_all(&value.to_le_bytes())?;
    }
    channel.write_all(&[255; 24])
}

//! 实际 Job child 故障响应经过生产接收链；不读取用户配置或伪造成功结果。
use super::*;
use std::os::windows::io::{AsHandle, AsRawHandle};
use windows::Win32::{Foundation::*, System::Threading::WaitForSingleObject};

#[test]
fn surface_webp_bad_wire_and_failed_child_never_publish_and_always_reap() {
    for mode in 0..=8u8 {
        let mut bytes = Image::webp_for_test();
        // 有界未知 RIFF chunk 仅携带夹具模式，不扩展生产启动参数或协议。
        bytes.extend_from_slice(b"TEST\x02\0\0\0");
        bytes.extend_from_slice(&[mode, 0]);
        let len = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&len.to_le_bytes());
        let mut process_handle = None;
        let result = run_with(&bytes, &|| false, |endpoint, deadline| {
            let child = process::start(
                endpoint,
                deadline,
                &|| false,
                " --exact image_decode::surface::webp::wire_tests::webp_fault_child --ignored --nocapture",
            )?;
            process_handle = Some(child.handle().try_clone_to_owned()?);
            Ok(child)
        });
        assert_eq!(result.is_ok(), mode == 0, "fault mode {mode}");
        if let Ok(image) = result {
            assert_eq!(&*image.pixels, &[0, 0, 255, 255, 0, 128, 0, 128]);
        }
        let handle = process_handle.expect("child was launched");
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 0) },
            WAIT_OBJECT_0,
            "unreaped child for fault mode {mode}"
        );
    }
}

#[test]
#[ignore = "private malformed wire peer invoked by bounded parent test"]
fn webp_fault_child() {
    std::process::exit(fault_reply().unwrap_or(1));
}

fn fault_reply() -> io::Result<i32> {
    let mut pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let mut channel = pipe.io(Some(Instant::now() + Duration::from_secs(5)), None);
    let mut request = [0; 8];
    channel.read_exact(&mut request)?;
    let len = u32::from_le_bytes(request[4..].try_into().unwrap()) as usize;
    if &request[..4] != b"OWI1" || !(2..=1024).contains(&len) {
        return Err(invalid());
    }
    let mut bytes = vec![0; len];
    channel.read_exact(&mut bytes)?;
    let mode = bytes[len - 2];
    if mode == 8 {
        // 已消费请求但尚未发出响应即异常退出。
        return Ok(9);
    }
    let mut response = b"OWP1".to_vec();
    for n in [2u32, 1, 8] {
        response.extend_from_slice(&n.to_le_bytes());
    }
    response.extend_from_slice(&[0, 0, 255, 255, 0, 128, 0, 128]);
    match mode {
        1 => response[0] = b'X',
        2 => response[4..8].copy_from_slice(&3u32.to_le_bytes()),
        3 => response[12..16].copy_from_slice(&12u32.to_le_bytes()),
        4 => response.truncate(7),
        5 => response.truncate(20),
        6 => response.push(0),
        _ => {}
    }
    channel.write_all(&response)?;
    // 完整合法输出仍不能掩盖 child 的非零退出。
    Ok(if mode == 7 { 9 } else { 0 })
}

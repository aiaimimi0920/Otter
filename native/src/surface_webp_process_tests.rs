//! 真实 Job 限额、迟到容量和停滞 child 回收；只启动当前测试 EXE 的固定私有入口。
use super::*;
use crate::live_recovery_pipe::Pipe;
use std::os::windows::io::AsRawHandle;
use std::{io::Read, time::Duration};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
#[test]
fn surface_webp_job_rejects_over_limit_allocation() {
    let (mut pipe, endpoint) = Pipe::pair().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let child = start(
        endpoint.into_handle(),
        deadline,
        &|| false,
        " --exact surface_webp_worker::tests::webp_memory_entry --ignored --nocapture",
    )
    .unwrap();
    let mut result = [0];
    pipe.io(Some(deadline), Some(child.handle()))
        .read_exact(&mut result)
        .unwrap();
    assert_eq!(&result, b"L");
    child.finish(deadline, &|| false).unwrap();
    let peak = child.peak_job_bytes().unwrap();
    assert!(peak <= CHILD_LIMIT);
    println!("webp-job-proof: peak-bytes={} allocation-denied=true", peak);
}
#[test]
fn surface_webp_stalled_child_timeout_cancel_and_drop_reap() {
    for cancel in [false, true] {
        let (mut pipe, endpoint) = Pipe::pair().unwrap();
        let start_time = Instant::now();
        let deadline = start_time + Duration::from_millis(200);
        let child = start(
            endpoint.into_handle(),
            deadline,
            &|| false,
            " --exact surface_webp_worker::tests::webp_stall_entry --ignored --nocapture",
        )
        .unwrap();
        let handle = child.handle().try_clone_to_owned().unwrap();
        let stopped = || cancel && start_time.elapsed() >= Duration::from_millis(40);
        assert!(pipe
            .cancellable_io(deadline, child.handle(), &stopped)
            .read_exact(&mut [0])
            .is_err());
        drop(child);
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 0) },
            WAIT_OBJECT_0
        );
        assert!(start_time.elapsed() < Duration::from_secs(3));
    }
}
#[test]
fn surface_webp_capacity_waits_for_release_and_honors_cancel() {
    let occupied = CAPACITY.reserve(256 * 1024 * 1024).unwrap();
    let now = Instant::now();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        drop(occupied);
    });
    let lease = capacity(now + Duration::from_secs(3), &|| false).unwrap();
    release.join().unwrap();
    assert!(now.elapsed() >= Duration::from_millis(60));
    drop(lease);
    let _occupied = CAPACITY.reserve(256 * 1024 * 1024).unwrap();
    assert!(capacity(Instant::now() + Duration::from_secs(3), &|| true).is_err());
    for cancel in [false, true] {
        let now = Instant::now();
        let stopped = || cancel && now.elapsed() >= Duration::from_millis(40);
        let limit = if cancel {
            Duration::from_secs(5)
        } else {
            Duration::from_millis(100)
        };
        assert!(capacity(now + limit, &stopped).is_err());
        assert!(
            now.elapsed()
                >= if cancel {
                    Duration::from_millis(40)
                } else {
                    limit
                }
        );
        // 取消必须明显早于五秒 deadline 返回，不能把等到超时误记为取消成功。
        assert!(now.elapsed() < Duration::from_secs(3));
    }
}

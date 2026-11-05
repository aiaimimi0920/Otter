//! 真实受限 child 的坏帧、停滞、容量和取消门；不需要 ANGLE 夹具的测试默认执行。
use super::*;
use crate::{
    bounded_worker_process as process,
    shader_angle::{Texture, Uniform},
};
use std::{
    os::windows::io::{AsRawHandle, OwnedHandle},
    time::Instant,
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
fn invoke<T>(mode: f32, use_request: impl FnOnce(Invocation<'_>) -> T) -> T {
    let identity = angle::Identity {
        egl: angle::Fingerprint {
            bytes: 1,
            sha256: [0; 32],
        },
        gles: angle::Fingerprint {
            bytes: 1,
            sha256: [0; 32],
        },
    };
    let bundle = TrustedBundle::new(std::path::Path::new("."), identity).unwrap();
    let pixels = [64, 32, 16, 128];
    let textures = [Texture {
        name: "input",
        width: 1,
        height: 1,
        rgba: &pixels,
    }];
    let uniforms = [Uniform {
        name: "mode",
        value: mode,
    }];
    use_request(Invocation {
        bundle: &bundle,
        backend: Backend::Warp,
        width: 1,
        height: 1,
        request: Request {
            vertex: "v",
            fragment: "f",
            textures: &textures,
            uniforms: &uniforms,
        },
    })
}
fn spawn(
    endpoint: OwnedHandle,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
) -> io::Result<process::Child> {
    process::start(
        endpoint,
        deadline,
        stopped,
        process::Policy {
            suffix: " --exact shader_worker::fixtures::shader_fault_entry --ignored --nocapture",
            maximum: CHILD_LIMIT,
            pool: &CAPACITY,
            environment: process::Environment::SystemRoot,
        },
    )
}
fn exited(handle: OwnedHandle) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 0) },
        WAIT_OBJECT_0
    );
}
#[test]
fn shader_child_bad_frames_exit_status_and_extra_bytes_never_publish_and_always_reap() {
    for mode in [1., 2., 3., 4., 9., 10.] {
        let mut handle = None;
        let result = invoke(mode, |value| {
            client::run(
                value,
                Instant::now() + Duration::from_secs(3),
                &|| false,
                |endpoint, deadline| {
                    let child = spawn(endpoint, deadline, &|| false)?;
                    handle = Some(child.handle().try_clone_to_owned()?);
                    Ok(child)
                },
            )
        });
        assert!(matches!(result, Err(Failure::Transport(_))), "mode={mode}");
        exited(handle.unwrap());
    }
    let result = invoke(0., |value| {
        client::run(
            value,
            Instant::now() + Duration::from_secs(3),
            &|| false,
            |endpoint, deadline| spawn(endpoint, deadline, &|| false),
        )
    })
    .unwrap();
    assert_eq!(&*result.rgba, &[64, 32, 16, 128]);
    assert!(matches!(
        invoke(6., |value| client::run(
            value,
            Instant::now() + Duration::from_secs(3),
            &|| false,
            |endpoint, deadline| spawn(endpoint, deadline, &|| false)
        )),
        Err(Failure::Backend(angle::Error::Compile(_)))
    ));
}
#[test]
fn shader_child_stall_before_and_after_frame_obeys_timeout_cancel_and_reaps() {
    for mode in [5., 7.] {
        for cancel in [false, true] {
            let start = Instant::now();
            let deadline = start
                + if cancel {
                    Duration::from_secs(5)
                } else {
                    Duration::from_millis(220)
                };
            let stopped = || cancel && start.elapsed() >= Duration::from_millis(50);
            let mut handle = None;
            let result = invoke(mode, |value| {
                client::run(value, deadline, &stopped, |endpoint, limit| {
                    let child = spawn(endpoint, limit, &stopped)?;
                    handle = Some(child.handle().try_clone_to_owned()?);
                    Ok(child)
                })
            });
            assert!(matches!(result, Err(Failure::Transport(_))));
            exited(handle.unwrap());
            assert!(start.elapsed() < Duration::from_secs(3));
        }
    }
}
#[test]
fn shader_capacity_waits_without_expanding_image_pool_and_checks_cancellation() {
    let occupied = CAPACITY.reserve(CHILD_LIMIT).unwrap();
    let now = Instant::now();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(80));
        drop(occupied);
    });
    let lease = process::capacity(
        &CAPACITY,
        CHILD_LIMIT,
        now + Duration::from_secs(3),
        &|| false,
    )
    .unwrap();
    release.join().unwrap();
    assert!(now.elapsed() >= Duration::from_millis(60));
    drop(lease);
    let _occupied = CAPACITY.reserve(CHILD_LIMIT).unwrap();
    for cancel in [false, true] {
        let now = Instant::now();
        let stopped = || cancel && now.elapsed() >= Duration::from_millis(40);
        assert!(process::capacity(
            &CAPACITY,
            CHILD_LIMIT,
            now + if cancel {
                Duration::from_secs(5)
            } else {
                Duration::from_millis(100)
            },
            &stopped
        )
        .is_err());
        assert!(now.elapsed() < Duration::from_secs(3));
    }
    assert!(invoke(0., |value| client::run(
        value,
        Instant::now() + Duration::from_secs(1),
        &|| true,
        |_, _| panic!("cancelled request must not spawn")
    ))
    .is_err());
}

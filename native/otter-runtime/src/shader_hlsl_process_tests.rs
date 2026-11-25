//! 真实 Job/pipe child，坏帧/退出/停滞与取消均核验已回收；不把这些合成故障当驱动 hang。
use super::*;
use crate::shader_hlsl::contract_tests::with_request;
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[test]
#[ignore = "private HLSL child entry; invoked by bounded parent"]
fn hlsl_worker_entry() {
    std::process::exit(if run_inherited().is_ok() { 0 } else { 1 });
}
#[test]
#[ignore = "private fixed fault child; never a product entry"]
fn hlsl_fault_entry() {
    bounded::require_limits(CHILD_LIMIT).unwrap();
    let environment = std::env::vars_os().collect::<Vec<_>>();
    assert_eq!(environment.len(), 1);
    assert!(environment[0]
        .0
        .to_str()
        .unwrap()
        .eq_ignore_ascii_case("SystemRoot"));
    let mut pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned().unwrap()).unwrap();
    let mut channel = pipe.io(Some(Instant::now() + TIMEOUT), None);
    let request = wire::read_request(&mut channel).unwrap();
    let mode = request.with_request(|r| r.source.parse::<u32>().unwrap());
    if mode == 5 {
        std::thread::sleep(Duration::from_secs(60));
    }
    let frame = Frame {
        width: 1,
        height: 1,
        rgba: vec![64, 32, 16, 128].into(),
        backend: SelectedBackend::D3d12,
    };
    let mut bytes = Vec::new();
    wire::write_response(
        &mut bytes,
        &if mode == 6 {
            Err(Error::Compile("synthetic compile error".into()))
        } else {
            Ok(frame)
        },
    )
    .unwrap();
    match mode {
        1 => bytes[28..32].copy_from_slice(&u32::MAX.to_le_bytes()),
        2 => bytes.truncate(34),
        3 => bytes.push(1),
        9 => bytes[20..24].copy_from_slice(&2u32.to_le_bytes()),
        10 => bytes[8..12].copy_from_slice(&3u32.to_le_bytes()),
        _ => {}
    }
    std::io::Write::write_all(&mut channel, &bytes).unwrap();
    if mode == 7 {
        std::thread::sleep(Duration::from_secs(60));
    }
    std::process::exit(if mode == 4 { 7 } else { 0 });
}
fn spawn(
    endpoint: OwnedHandle,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
) -> io::Result<bounded::Child> {
    bounded::start(
        endpoint,
        deadline,
        stopped,
        bounded::Policy {
            suffix: " --exact shader_hlsl::process::tests::hlsl_fault_entry --ignored --nocapture",
            maximum: CHILD_LIMIT,
            pool: &CAPACITY,
            environment: bounded::Environment::SystemRoot,
        },
    )
}
#[test]
fn fault_child_bad_frames_extra_bytes_exit_and_backend_failure_do_not_publish() {
    let _lock = TEST_LOCK.lock().unwrap();
    for mode in [1, 2, 3, 4, 9, 10] {
        let mut probe = None;
        let result = with_request(&mode.to_string(), |r| {
            run(
                r,
                Backend::D3d12,
                Instant::now() + Duration::from_secs(3),
                &|| false,
                |endpoint, deadline| {
                    let child = spawn(endpoint, deadline, &|| false)?;
                    probe = Some(child.probe()?);
                    Ok(child)
                },
            )
        });
        assert!(
            matches!(result, Err(Failure::Transport(_))),
            "mode={mode}: {result:?}"
        );
        assert!(probe.unwrap().exited());
    }
    with_request("0", |r| {
        assert_eq!(
            &*run(
                r,
                Backend::D3d12,
                Instant::now() + Duration::from_secs(3),
                &|| false,
                |e, d| spawn(e, d, &|| false)
            )
            .unwrap()
            .rgba,
            &[64, 32, 16, 128]
        )
    });
    with_request("6", |r| {
        assert!(matches!(
            run(
                r,
                Backend::D3d12,
                Instant::now() + Duration::from_secs(3),
                &|| false,
                |e, d| spawn(e, d, &|| false)
            ),
            Err(Failure::Backend(Error::Compile(_)))
        ))
    });
}
#[test]
fn stalled_child_before_after_frame_is_cancelled_timed_out_reaped_and_capacity_returned() {
    let _lock = TEST_LOCK.lock().unwrap();
    for mode in [5, 7] {
        for cancelled in [false, true] {
            let start = Instant::now();
            let stopped = || cancelled && start.elapsed() > Duration::from_millis(80);
            let deadline = start
                + if cancelled {
                    Duration::from_secs(5)
                } else {
                    Duration::from_millis(400)
                };
            let mut probe = None;
            let result = with_request(&mode.to_string(), |r| {
                run(r, Backend::D3d12, deadline, &stopped, |endpoint, limit| {
                    let child = spawn(endpoint, limit, &stopped)?;
                    probe = Some(child.probe()?);
                    Ok(child)
                })
            });
            assert!(result.is_err());
            let probe = probe.unwrap();
            assert!(probe.exited());
            assert!(probe.peak_job_bytes().unwrap() <= CHILD_LIMIT);
            assert!(start.elapsed() < Duration::from_secs(3));
            assert_eq!(CAPACITY.used(), 0);
        }
    }
}
#[test]
fn capacity_and_prespan_cancel_are_bounded_without_spawning() {
    let _lock = TEST_LOCK.lock().unwrap();
    with_request("unused", |r| {
        assert!(matches!(
            run(
                r,
                Backend::Auto,
                Instant::now() + TIMEOUT,
                &|| true,
                |_, _| panic!("must not spawn")
            ),
            Err(Failure::Backend(Error::Cancelled))
        ))
    });
    let _occupied = CAPACITY.reserve(CHILD_LIMIT).unwrap();
    let start = Instant::now();
    assert!(bounded::capacity(
        &CAPACITY,
        CHILD_LIMIT,
        start + Duration::from_millis(80),
        &|| false
    )
    .is_err());
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
#[ignore = "explicit same-target product EXE private HLSL entry gate; build product first"]
fn actual_product_private_entry_backends_pixels_peak_and_reaping() {
    let current = std::env::current_exe().unwrap();
    let executable = current
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("otter-native.exe");
    assert!(
        executable.is_file(),
        "Build same target/profile product first"
    );
    for backend in [Backend::D3d12, Backend::D3d11, Backend::Warp] {
        let mut probe = None;
        let result = with_request(
            "float4 otter_main(float2 uv){return u_input.Sample(otter_sampler,uv);}",
            |r| {
                run(
                    r,
                    backend,
                    Instant::now() + TIMEOUT,
                    &|| false,
                    |endpoint, deadline| {
                        let child = bounded::start_at(
                            endpoint,
                            deadline,
                            &|| false,
                            bounded::Policy {
                                suffix: " --native-hlsl-worker",
                                maximum: CHILD_LIMIT,
                                pool: &CAPACITY,
                                environment: bounded::Environment::SystemRoot,
                            },
                            &executable,
                        )?;
                        probe = Some(child.probe()?);
                        Ok(child)
                    },
                )
            },
        )
        .unwrap();
        assert_eq!(&*result.rgba, &[64, 32, 16, 128]);
        assert_eq!(
            result.backend,
            match backend {
                Backend::D3d12 => SelectedBackend::D3d12,
                Backend::D3d11 => SelectedBackend::D3d11,
                _ => SelectedBackend::Warp,
            }
        );
        let probe = probe.unwrap();
        assert!(probe.exited());
        let peak = probe.peak_job_bytes().unwrap();
        assert!(peak > 0 && peak <= CHILD_LIMIT);
        assert_eq!(CAPACITY.used(), 0);
        eprintln!("hlsl-product: backend={backend:?} peak-job-bytes={peak} exited=true");
    }
    for name in [
        windows::core::w!("libEGL.dll"),
        windows::core::w!("libGLESv2.dll"),
        windows::core::w!("d3dcompiler_47.dll"),
    ] {
        assert!(
            unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(name) }.is_err(),
            "parent must not load renderer/compiler DLL"
        );
    }
}

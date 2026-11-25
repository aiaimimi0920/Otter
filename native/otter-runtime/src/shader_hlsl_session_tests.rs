//! 热会话复用、满池交接、线程失败和编译失败后的真实子进程退出回归。
use super::*;
use crate::shader_hlsl::contract_tests::with_request;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::Win32::System::Threading::GetProcessId;
const SESSION_ENTRY: &str =
    " --exact shader_hlsl::session::tests::hlsl_session_worker_entry --ignored --nocapture";
const COLD_ENTRY: &str =
    " --exact shader_hlsl::process::tests::hlsl_worker_entry --ignored --nocapture";
const SHADER: &str = "float4 otter_main(float2 uv){return u_input.Sample(otter_sampler,uv);}";

#[test]
#[ignore = "private warm HLSL session worker entry; invoked by bounded parent"]
fn hlsl_session_worker_entry() {
    std::process::exit(if run_inherited().is_ok() { 0 } else { 1 });
}

#[test]
#[ignore = "private short-idle HLSL session worker entry"]
fn hlsl_short_idle_session_worker_entry() {
    std::process::exit(if run_with_idle(Duration::from_millis(250)).is_ok() {
        0
    } else {
        1
    });
}

fn parked_pid(scope: &str) -> Option<u32> {
    let guard = POOL.0.lock().unwrap();
    guard
        .entries
        .iter()
        .find(|e| e.scope == scope)
        .map(|e| unsafe { GetProcessId(HANDLE(e.child.handle().as_raw_handle())) })
}

#[test]
fn hlsl_session_keeps_pixels_and_rotates_after_32_requests() {
    let mut first_pid = None;
    let mut first_handle = None;
    with_request(SHADER, |request| {
        for index in 0..=MAX_REQUESTS {
            let frame = render_with_entry(
                "rotation",
                request,
                Backend::Warp,
                &|| false,
                IDLE_TIMEOUT,
                SESSION_ENTRY,
            )
            .unwrap();
            assert_eq!(&*frame.rgba, &[64, 32, 16, 128]);
            let pid = parked_pid("rotation");
            if index == 0 {
                first_pid = pid;
                assert!(pid.is_some_and(|pid| pid != 0));
                first_handle = Some(
                    POOL.0.lock().unwrap().entries[0]
                        .child
                        .handle()
                        .as_handle()
                        .try_clone_to_owned()
                        .unwrap(),
                );
            } else if index < MAX_REQUESTS - 1 {
                assert_eq!(pid, first_pid);
            } else if index == MAX_REQUESTS - 1 {
                assert_eq!(pid, None);
                assert_eq!(CAPACITY.used(), 0);
            } else {
                assert!(first_handle.is_some(), "keep old PID reserved");
                assert!(pid.is_some_and(|pid| pid != 0));
                assert_ne!(pid, first_pid);
            }
        }
    });
    drop(take_matching("rotation", Backend::Warp, SESSION_ENTRY));
    assert_eq!(CAPACITY.used(), 0);
}

#[test]
fn same_scope_reuses_warm_worker_isolated_from_other_scope_and_auto_reaps() {
    with_request(SHADER, |req| {
        let first = render_with_entry(
            "unit-a",
            req,
            Backend::Warp,
            &|| false,
            Duration::from_millis(250),
            SESSION_ENTRY,
        )
        .unwrap();
        assert_eq!(&*first.rgba, &[64, 32, 16, 128]);
        let pid_a1 = parked_pid("unit-a").expect("parked unit-a");
        let second = render_with_entry(
            "unit-a",
            req,
            Backend::Warp,
            &|| false,
            Duration::from_millis(250),
            SESSION_ENTRY,
        )
        .unwrap();
        assert_eq!(&*second.rgba, &[64, 32, 16, 128]);
        let pid_a2 = parked_pid("unit-a").expect("still parked unit-a");
        assert_eq!(pid_a1, pid_a2, "same scope must reuse warm worker");
        // 不同 scope 不得复用 unit-a 的 worker，且 256MiB 池满时驱逐旧 scope。
        let other = render_with_entry(
            "unit-b",
            req,
            Backend::Warp,
            &|| false,
            Duration::from_millis(120),
            SESSION_ENTRY,
        )
        .unwrap();
        assert_eq!(&*other.rgba, &[64, 32, 16, 128]);
        let pid_b = parked_pid("unit-b").expect("parked unit-b");
        assert_ne!(pid_a1, pid_b, "cross-scope worker reuse is forbidden");
    });
    std::thread::sleep(Duration::from_millis(220));
    assert_eq!(CAPACITY.used(), 0, "idle reaper must release capacity");
}

#[test]
fn hlsl_hot_and_cold_waiters_reclaim_newly_parked_active_worker() {
    for cold in [false, true] {
        with_request(SHADER, |request| {
            render_with_entry(
                "active",
                request,
                Backend::Warp,
                &|| false,
                IDLE_TIMEOUT,
                SESSION_ENTRY,
            )
            .unwrap();
            let active = take_matching("active", Backend::Warp, SESSION_ENTRY).unwrap();
            let (waiting, observed) = std::sync::mpsc::channel();
            std::thread::scope(|threads| {
                let waiter = threads.spawn(|| {
                    let checks = AtomicUsize::new(0);
                    let stopped = || {
                        if checks.fetch_add(1, Ordering::Relaxed) == 4 {
                            waiting.send(()).unwrap();
                        }
                        false
                    };
                    if cold {
                        process::render_with_entry(request, Backend::Warp, &stopped, COLD_ENTRY)
                    } else {
                        render_with_entry(
                            "waiting",
                            request,
                            Backend::Warp,
                            &stopped,
                            IDLE_TIMEOUT,
                            SESSION_ENTRY,
                        )
                    }
                });
                observed.recv_timeout(Duration::from_secs(3)).unwrap();
                assert_eq!(CAPACITY.used(), CHILD_LIMIT);
                assert!(
                    active.alive(),
                    "capacity reclamation must not touch active owner"
                );
                let parked_at = Instant::now();
                park(active);
                let frame = waiter.join().unwrap().unwrap();
                assert_eq!(&*frame.rgba, &[64, 32, 16, 128]);
                assert!(parked_at.elapsed() < Duration::from_secs(5));
            });
        });
        reclaim_idle();
        assert_eq!(CAPACITY.used(), 0);
    }
}

#[test]
fn hlsl_first_compile_error_exits_cleanly_at_request_boundary() {
    let entry =
        " --exact shader_hlsl::session::tests::hlsl_short_idle_session_worker_entry --ignored --nocapture";
    with_request("float4 otter_main(float2 uv){invalid shader;}", |request| {
        assert!(matches!(
            render_with_entry(
                "first-compile-error",
                request,
                Backend::Warp,
                &|| false,
                IDLE_TIMEOUT,
                entry,
            ),
            Err(Failure::Backend(Error::Compile(_)))
        ));
    });
    let Parked { pipe, child, .. } =
        take_matching("first-compile-error", Backend::Warp, entry).unwrap();
    // 子端空闲到期走正常析构；finish 真实读取 0 exit code，不以 Drop 的 Job 强杀冒充。
    child
        .finish(Instant::now() + Duration::from_secs(5), &|| false)
        .unwrap();
    drop(pipe);
    drop(child);
    assert_eq!(CAPACITY.used(), 0);
}

#[test]
fn hlsl_reaper_spawn_failure_releases_parked_capacity_and_resets_reservation() {
    let (pipe, endpoint) = Pipe::pair().unwrap();
    let child = bounded::start(
        endpoint.into_handle(),
        Instant::now() + TIMEOUT,
        &|| false,
        bounded::Policy {
            suffix: SESSION_ENTRY,
            maximum: CHILD_LIMIT,
            pool: &CAPACITY,
            environment: bounded::Environment::SystemRoot,
        },
    )
    .unwrap();
    park_with(
        Parked {
            scope: "failed-reaper".into(),
            backend: Backend::Warp,
            suffix: SESSION_ENTRY,
            pipe,
            child,
            requests: 0,
            expires_at: Instant::now() + IDLE_TIMEOUT,
        },
        || Err(io::Error::other("controlled reaper spawn failure")),
    );
    let state = POOL.0.lock().unwrap();
    assert!(!state.reaper_running);
    assert!(state.entries.is_empty());
    assert_eq!(CAPACITY.used(), 0);
}

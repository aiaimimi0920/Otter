use super::*;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
#[test]
fn live_owner_requires_explicit_apartment_before_workers_can_start() {
    use windows::Win32::System::Com::*;
    assert!(require_owner_apartment().is_err());
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        let result = require_owner_apartment();
        CoUninitialize();
        assert!(result.is_ok());
    }
    assert!(require_owner_apartment().is_err());
}
#[test]
fn implicit_mta_from_another_worker_cannot_authorize_live_owner() {
    use windows::Win32::System::Com::*;
    let (ready, entered) = mpsc::channel();
    let (release, wait) = mpsc::channel::<()>();
    let worker = std::thread::spawn(move || unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
        ready.send(()).unwrap();
        // Sender 析构也解除等待，失败断言不会把夹具线程永久留在 MTA。
        let _ = wait.recv();
        CoUninitialize();
    });
    entered.recv_timeout(Duration::from_secs(3)).unwrap();
    let result = require_owner_apartment();
    drop(release);
    worker.join().unwrap();
    assert!(
        result.is_err(),
        "another worker must not supply the owner's COM lifetime"
    );
}

#[test]
fn region_binding_requires_owner_apartment_and_preserves_exact_binding_error() {
    use windows::Win32::Foundation::RECT;
    let region = RECT {
        left: 0,
        top: 0,
        right: 32,
        bottom: 32,
    };
    // 未准入时绝不执行窗口绑定，也不解析显示器或开始真实捕获。
    assert!(LiveJob::start_region_bound(region, |_| -> io::Result<()> {
        panic!("uninitialized owner must not bind")
    })
    .is_err());
    use windows::Win32::System::Com::*;
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _apartment = Apartment;
    let mut bound = false;
    let rejected = LiveJob::start_region_bound(region, |_| {
        bound = true;
        Err::<(), _>(io::Error::other("injected region binding failure"))
    });
    assert!(bound);
    assert_eq!(
        rejected.err().unwrap().to_string(),
        "injected region binding failure"
    );
}
fn result<T>(job: &mut LiveJob<T>) -> Result<(), String> {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = job.take_ready() {
            return result;
        }
        assert!(Instant::now() < end, "Live fixture did not finish");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn real_worker_publishes_shared_frame_and_drop_wakes_and_joins() {
    let active = Arc::new(AtomicUsize::new(0));
    let (entered, ready) = mpsc::channel();
    let (job, mut consumer) = LiveJob::spawn(&active, move |frames, _| {
        let (wake, wait) = mpsc::channel();
        frames
            .on_idle(Arc::new(move || {
                let _ = wake.send(());
            }))
            .unwrap();
        frames.publish(42u8).unwrap();
        entered.send(()).unwrap();
        wait.recv().unwrap();
        Ok(())
    })
    .unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let mut second = job.subscribe().unwrap();
    let first = consumer.try_next().unwrap().unwrap();
    let other = second.try_next().unwrap().unwrap();
    assert!(Arc::ptr_eq(&first.frame, &other.frame));
    assert_eq!(active.load(Ordering::Acquire), 1);
    drop(job);
    assert_eq!(active.load(Ordering::Acquire), 0);
    assert!(matches!(
        consumer.try_next(),
        Err(crate::live_frames::Error::Closed)
    ));
    assert_eq!(first.frame.resource, 42);
}
#[test]
fn worker_error_and_panic_close_consumers_and_release_admission() {
    for panic in [false, true] {
        let active = Arc::new(AtomicUsize::new(0));
        let (mut job, mut consumer) = LiveJob::<u8>::spawn(&active, move |_, _| {
            if panic {
                panic!("injected Live worker panic");
            }
            Err("injected Live worker error".into())
        })
        .unwrap();
        assert!(result(&mut job).is_err());
        assert!(!job.pending());
        assert!(job.take_ready().is_none());
        assert!(matches!(
            consumer.try_next(),
            Err(crate::live_frames::Error::Closed)
        ));
        assert_eq!(active.load(Ordering::Acquire), 0);
    }
}
#[test]
fn last_consumer_stops_idle_worker_and_cancel_never_reports_success() {
    for cancel in [false, true] {
        let active = Arc::new(AtomicUsize::new(0));
        let (entered, ready) = mpsc::channel();
        let (mut job, consumer) = LiveJob::<u8>::spawn(&active, move |frames, _| {
            let (wake, wait) = mpsc::channel();
            frames
                .on_idle(Arc::new(move || {
                    let _ = wake.send(());
                }))
                .unwrap();
            entered.send(()).unwrap();
            wait.recv().unwrap();
            Ok(())
        })
        .unwrap();
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        if cancel {
            job.cancel();
        }
        drop(consumer);
        assert_eq!(result(&mut job).is_err(), cancel);
        assert_eq!(active.load(Ordering::Acquire), 0);
    }
}
#[test]
fn session_admission_is_bounded_and_reusable_without_real_capture() {
    let active = Arc::new(AtomicUsize::new(0));
    let mut slots = (0..MAX_SESSIONS)
        .map(|_| Slot::acquire(&active).unwrap())
        .collect::<Vec<_>>();
    assert!(Slot::acquire(&active).is_err());
    assert!(LiveJob::<u8>::spawn(&active, |_, _| panic!("over capacity")).is_err());
    slots.pop();
    slots.push(Slot::acquire(&active).unwrap());
    drop(slots);
    assert_eq!(active.load(Ordering::Acquire), 0);
    assert!(LiveJob::start_monitor(HMONITOR::default()).is_err());
    assert!(LiveJob::start_region(windows::Win32::Foundation::RECT::default()).is_err());
}

#[test]
fn binding_runs_on_caller_before_worker_and_failure_does_not_start_capture() {
    let active = Arc::new(AtomicUsize::new(0));
    let caller = std::thread::current().id();
    let ran = Arc::new(AtomicBool::new(false));
    let worker_ran = ran.clone();
    let mut retained = None;
    let rejected = LiveJob::<u8>::spawn_bound(
        &active,
        |consumer| {
            assert_eq!(std::thread::current().id(), caller);
            retained = Some(consumer);
            Err::<(), _>(io::Error::other("injected binding failure"))
        },
        move |_, _| {
            worker_ran.store(true, Ordering::Release);
            Ok(())
        },
    );
    assert!(rejected.is_err());
    assert!(!ran.load(Ordering::Acquire));
    assert_eq!(active.load(Ordering::Acquire), 0);
    assert!(matches!(
        retained.as_mut().unwrap().try_next(),
        Err(crate::live_frames::Error::Closed)
    ));

    let bound = Arc::new(AtomicBool::new(false));
    let observed = bound.clone();
    let (mut job, _consumer) = LiveJob::<u8>::spawn_bound(
        &active,
        |consumer| {
            bound.store(true, Ordering::Release);
            Ok(consumer)
        },
        move |_, _| {
            assert_ne!(std::thread::current().id(), caller);
            assert!(observed.load(Ordering::Acquire));
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(result(&mut job), Ok(()));
    assert_eq!(active.load(Ordering::Acquire), 0);
}

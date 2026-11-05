use super::*;
use std::time::{Duration, Instant};

fn finish(runtime: &mut Runtime) -> Result<Output, Error> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = runtime.try_join() {
            return result;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn completed_unreaped_tasks_hold_capacity_until_join() {
    let mut runtimes: Vec<_> = (0..4)
        .map(|_| {
            Runtime::spawn(|_| {
                Ok(Output::Pairing {
                    device: "fixture".into(),
                    changed: false,
                })
            })
            .unwrap()
        })
        .collect();
    let deadline = Instant::now() + Duration::from_secs(3);
    while runtimes
        .iter()
        .any(|r| !r.thread.as_ref().unwrap().is_finished())
    {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        matches!(Runtime::spawn(|_| Err(Error::Stopped)), Err(e) if e.kind() == io::ErrorKind::WouldBlock)
    );
    assert!(runtimes[0].wait_handle().is_some());
    assert!(finish(&mut runtimes[0]).is_ok());
    assert!(runtimes[0].wait_handle().is_none());
    assert!(runtimes[0].try_join().is_none());
    let mut replacement = Runtime::spawn(|_| {
        Ok(Output::Pairing {
            device: "new".into(),
            changed: true,
        })
    })
    .unwrap();
    assert!(finish(&mut replacement).is_ok());
    for runtime in &mut runtimes[1..] {
        assert!(finish(runtime).is_ok());
    }
}

#[test]
fn cancelled_completed_result_is_not_published_and_panic_is_reaped() {
    let mut runtime = Runtime::spawn(|_| {
        Ok(Output::Pairing {
            device: "old".into(),
            changed: true,
        })
    })
    .unwrap();
    runtime.request_stop();
    assert!(matches!(finish(&mut runtime), Err(Error::Stopped)));
    let mut panicked = Runtime::spawn(|_| panic!("fixture panic")).unwrap();
    assert!(matches!(finish(&mut panicked), Err(Error::Panicked)));
    assert!(panicked.wait_handle().is_none());
}

#[test]
fn stop_wakes_task_and_drop_joins_instead_of_detaching() {
    let (tx, rx) = std::sync::mpsc::channel();
    let runtime = Runtime::spawn(move |cancel| {
        cancel.wait(Duration::from_secs(30));
        tx.send(()).unwrap();
        Err(Error::Stopped)
    })
    .unwrap();
    let start = Instant::now();
    drop(runtime);
    rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn accepted_offline_clear_commits_even_if_execution_is_already_cancelled() {
    let root = std::env::temp_dir().join(format!(
        "offline-clear-{:?}",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    let origin = "https://loom.example";
    crate::art_offline_store::Slot::invalidate_origin(&root, origin, 1, &|| false).unwrap();
    // 无缓存时不会创建缓存目录；构造已有目录再核实不可取消的原子减权。
    std::fs::create_dir(root.join("art-offline-cache")).unwrap();
    let cancel = Cancel::new();
    cancel.cancel();
    let output = run(
        Task::ClearOffline {
            config: crate::device_authorization_config::Config::new(origin, &root, "fixture")
                .unwrap(),
        },
        &cancel,
    )
    .unwrap();
    assert!(matches!(output, Output::OfflineCleared));
    assert!(crate::art_offline_store::Slot::cutoff(&root, origin).unwrap() > 1);
    assert!(!root.join("device-identity.json").exists());
}

//! 实际签名线程/child 的同版本 VIEW 更新、单槽与撤权；不代替编辑栏 HWND 接线。
use super::*;
use crate::surface_javascript::fixture::text;
const SOURCE: &str = include_str!("../tests/fixtures/javascript-view.js");

fn viewport() -> crate::surface_viewport::Viewport {
    crate::surface_viewport::Viewport::new([640, 480], 96).unwrap()
}

#[test]
fn signed_view_uses_existing_worker_backpressure_state_and_revocation_cleanup() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut runtime, owner, probe) = start_source(SOURCE);
    let current = runtime.binding.clone();
    assert_eq!(
        runtime.select_view(&current, "main", viewport()),
        Err(Error::Io(io::ErrorKind::WouldBlock))
    );
    let frame = reply(&mut runtime);
    let button = frame.scene.root().children()[1].id().to_owned();
    let probe = probe.recv_timeout(Duration::from_secs(5)).unwrap();
    runtime
        .submit(&current, Operation::Click(button.clone()))
        .unwrap();
    assert_eq!(text(&reply(&mut runtime).scene), "none:1:0:1:0:1");
    runtime
        .select_view(&current, "detail/large:1", viewport())
        .unwrap();
    assert_eq!(
        runtime.select_view(&current, "other", viewport()),
        Err(Error::Io(io::ErrorKind::WouldBlock))
    );
    assert_eq!(text(&reply(&mut runtime).scene), "detail/large:1:1:1:1:0:1");
    runtime.submit(&current, Operation::Click(button)).unwrap();
    assert_eq!(text(&reply(&mut runtime).scene), "detail/large:1:2:1:1:0:1");
    assert_eq!(
        runtime.select_view(&current, "bad space", viewport()),
        Err(Error::Io(io::ErrorKind::InvalidInput))
    );
    assert!(!runtime.pending());
    runtime.select_view(&current, "main", viewport()).unwrap();
    assert_eq!(text(&reply(&mut runtime).scene), "main:2:2:1:0:1");
    drop(owner);
    exit(&mut runtime);
    assert!(probe.exited());
    assert_eq!(crate::surface_javascript::CAPACITY.used(), 0);
}

#[test]
fn view_never_accepts_replaced_host_and_revoked_ready_reply() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    for replaced in [false, true] {
        let (mut runtime, owner, probe) = start_source(SOURCE);
        reply(&mut runtime);
        let probe = probe.recv_timeout(Duration::from_secs(5)).unwrap();
        let mut current = runtime.binding.clone();
        if replaced {
            current.request_id = "new-request".into();
            assert_eq!(
                runtime.select_view(&current, "main", viewport()),
                Err(Error::Stopped)
            );
        } else {
            runtime.select_view(&current, "main", viewport()).unwrap();
            let handle = [HANDLE(runtime.mailbox.reply_handle().as_raw_handle())];
            assert_eq!(
                unsafe { WaitForMultipleObjects(&handle, false, 8000) },
                WAIT_OBJECT_0
            );
            drop(owner);
            assert!(matches!(
                runtime.take_reply(&current),
                Some(Err(Error::Stopped))
            ));
        }
        exit(&mut runtime);
        assert!(probe.exited());
    }
    assert_eq!(crate::surface_javascript::CAPACITY.used(), 0);
}

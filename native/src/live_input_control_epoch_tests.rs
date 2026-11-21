//! 后端固定原控制周期与原 held 清理；只使用自有 fixture，不安装 DLL 或操作外部应用。
use super::*;
use crate::live_input::Backend as _;
use crate::live_input_worker::{Command, Reason, Worker};
use crate::live_window_target::recovery::popup_layer::fixture::{pump, Fixture};
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

#[test]
fn revoked_worker_rejects_preflight_send_and_native_preparation_before_installing_hooks() {
    let fixture = Fixture::new();
    let original = fixture.owner.identity().capture_control().unwrap();
    let root = Target::bind(fixture.root.0).unwrap();
    let target = Destination::bind(&root, root.hwnd()).unwrap();
    let mut backend = Backend::new_local(root.clone(), None)
        .unwrap()
        .with_hover(Some(original));
    backend.preflight().unwrap();
    fixture.owner.control(false).unwrap();
    fixture.owner.control(true).unwrap();
    assert!(!original.valid());
    assert_eq!(backend.preflight(), Err(Error::ControlRevoked));
    assert_eq!(
        backend.send(
            target.clone(),
            Event::Key {
                code: 65,
                down: true
            },
            0,
            250
        ),
        Err(Error::ControlRevoked)
    );
    assert_eq!(backend.prepare_native(), Err(Error::ControlRevoked));
    assert!(backend.hover_binding.is_none());
    assert!(backend.hovered.is_none());
    assert!(backend.mouse.is_none());
    // 清理资格不随控制周期一起丢弃：原目标仍必须能接收欠下的释放。
    assert!(backend.cleanup_target(target).is_some());
    assert_eq!(backend.leave().failed, 0);
    let current = fixture.owner.identity().capture_control().unwrap();
    let mut next = Backend::new_local(root, None)
        .unwrap()
        .with_hover(Some(current));
    next.preflight().unwrap();
}

#[test]
fn remote_backend_cannot_use_a_local_control_snapshot() {
    let fixture = Fixture::new();
    let control = fixture.owner.identity().capture_control().unwrap();
    let mut backend = Backend::new(Target::bind(fixture.root.0).unwrap(), None)
        .unwrap()
        .with_hover(Some(control));
    assert_eq!(backend.preflight(), Err(Error::InvalidDelivery));
    assert_eq!(backend.prepare_native(), Err(Error::InvalidDelivery));
    assert!(backend.hover_binding.is_none());
}

#[test]
fn replacement_controller_identity_is_not_a_recoverable_cycle_change() {
    let fixture = Fixture::new();
    let control = fixture.owner.identity().capture_control().unwrap();
    let replacement = crate::live_hover_owner::Owner::new(fixture.b.0).unwrap();
    replacement.control(true).unwrap();
    let mut backend = Backend::new_local(Target::bind(fixture.root.0).unwrap(), None)
        .unwrap()
        .with_hover(Some(control));
    assert!(matches!(backend.preflight(), Err(Error::NativePopup(_))));
    assert!(backend.hover_binding.is_none());
}

thread_local! { static EDGES: Cell<(u8, u8)> = const { Cell::new((0, 0)) }; }
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    EDGES.with(|slot| {
        let (down, up) = slot.get();
        if message == WM_KEYDOWN {
            slot.set((down + 1, up));
        }
        if message == WM_KEYUP {
            slot.set((down, up + 1));
        }
    });
    DefWindowProcW(hwnd, message, wp, lp)
}
fn dispatch(
    worker: &mut Worker<Error>,
    command: Command,
) -> Result<live_input::Delivery, live_input::Error<Error>> {
    worker.submit(command).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        pump();
        if let Some(reply) = worker.poll_reply().unwrap() {
            return reply.result;
        }
        assert!(Instant::now() < deadline, "control epoch reply deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn revoked_worker_still_releases_its_original_held_key() {
    let fixture = Fixture::new();
    unsafe {
        SetWindowLongPtrW(
            fixture.root.0,
            GWLP_WNDPROC,
            procedure as *const () as isize,
        );
    }
    EDGES.with(|slot| slot.set((0, 0)));
    let control = fixture.owner.identity().capture_control().unwrap();
    let root = Target::bind(fixture.root.0).unwrap();
    let mut worker = Worker::start(move || {
        Backend::new_local(root, None)
            .map(|backend| backend.with_hover(Some(control)))
            .map_err(|error| format!("{error:?}"))
    })
    .unwrap();
    dispatch(&mut worker, Command::Enable).unwrap();
    dispatch(
        &mut worker,
        Command::Input {
            sequence: 1,
            event: Event::Key {
                code: 65,
                down: true,
            },
        },
    )
    .unwrap();
    fixture.owner.control(false).unwrap();
    fixture.owner.control(true).unwrap();
    assert!(matches!(
        dispatch(
            &mut worker,
            Command::Input {
                sequence: 2,
                event: Event::Key {
                    code: 66,
                    down: true
                }
            }
        ),
        Err(live_input::Error::Backend(Error::ControlRevoked))
    ));
    let deadline = Instant::now() + Duration::from_secs(3);
    let exit = loop {
        pump();
        if let Some(exit) = worker.take_exit() {
            break exit;
        }
        assert!(Instant::now() < deadline, "control epoch cleanup deadline");
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(exit.reason, Reason::InputFailed);
    assert_eq!(
        exit.cleanup,
        Some(live_input::Cleanup {
            released: 1,
            failed: 0,
            skipped: 0
        })
    );
    EDGES.with(|slot| assert_eq!(slot.get(), (1, 1)));
}

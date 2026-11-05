use super::*;
use crate::{
    live_recovery_service::Service,
    live_recovery_wire::{Command, Request, Status, SCHEMA},
};
use windows::core::w;
struct Window(HWND);
impl Window {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("recovery binding fixture"),
                WS_POPUP,
                20,
                30,
                40,
                50,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot::read(&Target::bind(self.0).unwrap()).unwrap()
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

#[test]
fn actual_process_identity_round_trip_and_restore_preserve_hidden_window() {
    let window = Window::new();
    let snapshot = window.snapshot();
    let record = snapshot.recovery_record().unwrap();
    assert_eq!(record.process_id, std::process::id());
    assert_ne!(record.process_created, 0);
    let bound = Binding::bind(&record).unwrap();
    assert_eq!(bound.snapshot.recovery_record().unwrap(), record);
    drop(snapshot);
    unsafe {
        SetWindowLongPtrW(
            window.0,
            GWL_EXSTYLE,
            record.extended_style as isize | WS_EX_TRANSPARENT.0 as isize,
        );
    }
    bound.restore().unwrap();
    assert_eq!(window.snapshot().recovery_record().unwrap(), record);
    assert!(!unsafe { IsWindowVisible(window.0) }.as_bool());
}

#[test]
fn process_identity_mismatch_and_destroyed_targets_are_rejected() {
    let window = Window::new();
    let record = window.snapshot().recovery_record().unwrap();
    let mut wrong = record.clone();
    wrong.process_created += 1;
    assert!(Binding::bind(&wrong).is_err());
    let mut wrong = record.clone();
    wrong.process_id = wrong.process_id.wrapping_add(1);
    assert!(Binding::bind(&wrong).is_err());
    let mut wrong = record.clone();
    wrong.thread_id = wrong.thread_id.wrapping_add(1);
    assert!(Binding::bind(&wrong).is_err());
    let other = Window::new();
    let mut wrong = record.clone();
    wrong.hwnd = other.0 .0 as u64;
    // 同一存活 PID/TID 下的另一个 HWND 不能靠创建时间区分；这不是传输授权校验。
    assert!(Binding::bind(&wrong).is_ok());
    let bound = Binding::bind(&record).unwrap();
    drop(window);
    assert!(Binding::bind(&record).is_err());
    assert!(bound.restore().is_err());
    bound.restore_or_retire().unwrap();
}

#[test]
fn obsolete_identity_is_retired_without_writing_to_other_window() {
    let window = Window::new();
    let record = window.snapshot().recovery_record().unwrap();
    let mut bound = Binding::bind(&record).unwrap();
    bound.snapshot.target.identity.thread = record.thread_id.wrapping_add(1);
    bound.snapshot.extended_style |= WS_EX_TRANSPARENT.0 as isize;
    bound.restore_or_retire().unwrap();
    assert_eq!(window.snapshot().recovery_record().unwrap(), record);
}

#[test]
fn recovery_service_holds_real_binding_before_ack_and_restores_original_state() {
    let window = Window::new();
    let record = window.snapshot().recovery_record().unwrap();
    let mut service = Service::new([7; 16]).unwrap();
    let ack = service
        .apply(
            Request {
                schema: SCHEMA,
                session: [7; 16],
                sequence: 1,
                command: Command::Register {
                    id: 1,
                    record: record.clone(),
                },
            },
            |record| Binding::bind(record).map_err(std::io::Error::other),
        )
        .unwrap();
    assert_eq!(ack.status, Status::Accepted);
    unsafe {
        SetWindowLongPtrW(
            window.0,
            GWL_EXSTYLE,
            record.extended_style as isize | WS_EX_TRANSPARENT.0 as isize,
        );
    }
    service
        .recover(|_, binding| binding.restore().map_err(std::io::Error::other))
        .unwrap();
    assert_eq!(service.pending(), 0);
    assert_eq!(window.snapshot().recovery_record().unwrap(), record);
}

use super::*;
use std::{cell::Cell, rc::Rc};
fn record(hwnd: u64) -> Record {
    Record {
        hwnd,
        process_id: 2,
        thread_id: 3,
        process_created: 4,
        rect: [0, 0, 32, 32],
        placement_flags: 0,
        show_command: 1,
        min_position: [0, 0],
        max_position: [0, 0],
        normal_position: [0, 0, 32, 32],
        extended_style: 0,
        layered: None,
        iconic: false,
        visible: true,
    }
}
fn req(sequence: u64, command: Command) -> Request {
    Request {
        schema: SCHEMA,
        session: [7; 16],
        sequence,
        command,
    }
}
fn register(sequence: u64, id: u64, hwnd: u64) -> Request {
    req(
        sequence,
        Command::Register {
            id,
            record: record(hwnd),
        },
    )
}
struct Owner(Rc<Cell<usize>>);
impl Drop for Owner {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn lost_register_ack_replays_without_rebinding_or_replacing_snapshot() {
    let dropped = Rc::new(Cell::new(0));
    let mut service = Service::new([7; 16]).unwrap();
    let request = register(1, 1, 10);
    let ack = service
        .apply(request.clone(), |_| Ok(Owner(dropped.clone())))
        .unwrap();
    assert_eq!(ack.status, Status::Accepted);
    assert_eq!(
        service
            .apply(request, |_| panic!("retry must not bind"))
            .unwrap(),
        ack
    );
    let mut changed = register(1, 1, 10);
    if let Command::Register { record, .. } = &mut changed.command {
        record.rect[2] = 64;
    }
    assert!(service.apply(changed, |_| panic!("changed retry")).is_err());
    assert_eq!(service.pending(), 1);
    assert_eq!(dropped.get(), 0);
    let retire = req(2, Command::Retire { id: 1 });
    let ack = service
        .apply(retire.clone(), |_| panic!("retire must not bind"))
        .unwrap();
    assert_eq!(
        service.apply(retire, |_| panic!("retire retry")).unwrap(),
        ack
    );
    assert_eq!(service.pending(), 0);
    assert_eq!(dropped.get(), 1);
}

#[test]
fn late_retirement_cannot_delete_new_generation_and_old_id_cannot_resurrect() {
    let mut service = Service::new([7; 16]).unwrap();
    assert_eq!(
        service
            .apply(register(1, 1, 10), |_| Ok(()))
            .unwrap()
            .status,
        Status::Accepted
    );
    assert_eq!(
        service
            .apply(register(2, 2, 10), |_| panic!("duplicate source"))
            .unwrap()
            .status,
        Status::Rejected
    );
    service
        .apply(req(3, Command::Retire { id: 1 }), |_| Ok(()))
        .unwrap();
    assert_eq!(
        service
            .apply(register(4, 3, 10), |_| Ok(()))
            .unwrap()
            .status,
        Status::Accepted
    );
    service
        .apply(req(5, Command::Retire { id: 1 }), |_| Ok(()))
        .unwrap();
    assert_eq!(service.pending(), 1);
    assert_eq!(service.entries[0].id, 3);
    assert_eq!(
        service
            .apply(register(6, 1, 11), |_| panic!("reused id"))
            .unwrap()
            .status,
        Status::Rejected
    );
}

#[test]
fn shutdown_requires_no_recovery_responsibility_and_ack_can_be_retried() {
    let mut service = Service::new([7; 16]).unwrap();
    service.apply(register(1, 1, 10), |_| Ok(())).unwrap();
    assert_eq!(
        service
            .apply(req(2, Command::Shutdown), |_| Ok(()))
            .unwrap()
            .status,
        Status::Rejected
    );
    assert!(!service.shutdown_accepted());
    assert_eq!(service.pending(), 1);
    service
        .apply(req(3, Command::Retire { id: 1 }), |_| Ok(()))
        .unwrap();
    let request = req(4, Command::Shutdown);
    let ack = service.apply(request.clone(), |_| Ok(())).unwrap();
    assert!(service.shutdown_accepted());
    assert_eq!(service.apply(request, |_| Ok(())).unwrap(), ack);
    assert!(service.apply(register(5, 2, 10), |_| Ok(())).is_err());
}

#[test]
fn source_budget_and_failed_bind_are_bounded_and_replayable() {
    let mut service = Service::new([7; 16]).unwrap();
    for id in 1..=MAX_RECORDS as u64 {
        assert_eq!(
            service
                .apply(register(id, id, id), |_| Ok(()))
                .unwrap()
                .status,
            Status::Accepted
        );
    }
    let full = register(17, 17, 17);
    let ack = service
        .apply(full.clone(), |_| panic!("capacity must precede bind"))
        .unwrap();
    assert_eq!(ack.status, Status::Rejected);
    assert_eq!(
        service.apply(full, |_| panic!("retry capacity")).unwrap(),
        ack
    );
    service
        .apply(req(18, Command::Retire { id: 1 }), |_| Ok(()))
        .unwrap();
    let failed = register(19, 18, 18);
    assert_eq!(
        service
            .apply(failed.clone(), |_| Err(io::Error::other("bind failed")))
            .unwrap()
            .status,
        Status::Rejected
    );
    assert_eq!(
        service
            .apply(failed, |_| panic!("failed bind retry"))
            .unwrap()
            .status,
        Status::Rejected
    );
    assert_eq!(service.pending(), 15);
    assert_eq!(
        service
            .apply(register(20, 19, 19), |_| Ok(()))
            .unwrap()
            .status,
        Status::Accepted
    );
}

#[test]
fn recovery_preserves_failed_owners_and_retries_only_remaining_records() {
    let dropped = Rc::new(Cell::new(0));
    let mut service = Service::new([7; 16]).unwrap();
    for id in 1..=3 {
        service
            .apply(register(id, id, id), |_| Ok(Owner(dropped.clone())))
            .unwrap();
    }
    let mut seen = Vec::new();
    assert!(service
        .recover(|record, _| {
            seen.push(record.hwnd);
            if record.hwnd == 2 {
                Err(io::Error::other("restore failed"))
            } else {
                Ok(())
            }
        })
        .is_err());
    assert_eq!(seen, [1, 2, 3]);
    assert_eq!(service.pending(), 1);
    assert_eq!(dropped.get(), 2);
    assert!(service
        .apply(req(4, Command::Retire { id: 2 }), |_| panic!(
            "recovery is terminal"
        ))
        .is_err());
    service
        .recover(|record, _| {
            assert_eq!(record.hwnd, 2);
            Ok(())
        })
        .unwrap();
    assert_eq!(service.pending(), 0);
    assert_eq!(dropped.get(), 3);
}

#[test]
fn wrong_session_gaps_and_invalid_records_do_not_consume_sequence_or_bind() {
    let mut service = Service::<()>::new([7; 16]).unwrap();
    let mut wrong = register(1, 1, 10);
    wrong.session = [8; 16];
    assert!(service.apply(wrong, |_| panic!("wrong session")).is_err());
    assert!(service
        .apply(register(2, 1, 10), |_| panic!("sequence gap"))
        .is_err());
    assert!(service
        .apply(register(1, 1, 0), |_| panic!("invalid record"))
        .is_err());
    assert_eq!(
        service
            .apply(register(1, 1, 10), |_| Ok(()))
            .unwrap()
            .status,
        Status::Accepted
    );
}

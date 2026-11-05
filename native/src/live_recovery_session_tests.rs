use super::*;
use crate::{
    live_recovery_handshake::{read_ready, write_hello},
    live_recovery_wire::{read_ack, write_request, Command, Request, Status, SCHEMA},
};
use std::{
    io::Write,
    os::windows::io::{AsHandle, FromRawHandle, OwnedHandle},
};

fn peer() -> OwnedHandle {
    unsafe {
        OwnedHandle::from_raw_handle(
            OpenProcess(PROCESS_SYNCHRONIZE, false, std::process::id())
                .unwrap()
                .0,
        )
    }
}
fn request(sequence: u64, command: Command) -> Request {
    Request {
        schema: SCHEMA,
        session: [7; 16],
        sequence,
        command,
    }
}
fn record() -> Record {
    Record {
        hwnd: 1,
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
fn connect(pipe: &mut Pipe) {
    let mut channel = pipe.io(Some(Instant::now() + OPERATION_BUDGET), None);
    write_hello(&mut channel, [7; 16]).unwrap();
    read_ready(&mut channel, [7; 16]).unwrap();
}
fn send(pipe: &mut Pipe, request: Request) -> Status {
    let mut channel = pipe.io(Some(Instant::now() + OPERATION_BUDGET), None);
    write_request(&mut channel, &request).unwrap();
    read_ack(&mut channel, request.session, request.sequence)
        .unwrap()
        .status
}

#[test]
fn actual_pipe_session_replays_registration_retires_and_shuts_down() {
    let (mut parent, child) = Pipe::pair().unwrap();
    let worker = std::thread::spawn(move || {
        let peer = peer();
        let mut exchange = Exchange::accept(child, peer.as_handle(), OPERATION_BUDGET).unwrap();
        let mut binds = 0;
        for _ in 0..4 {
            let done = exchange
                .step(peer.as_handle(), |_| {
                    binds += 1;
                    Ok(())
                })
                .unwrap();
            if done {
                break;
            }
        }
        assert_eq!(binds, 1);
        assert_eq!(exchange.service.pending(), 0);
        assert!(exchange.service.shutdown_accepted());
    });
    connect(&mut parent);
    let register = request(
        1,
        Command::Register {
            id: 1,
            record: record(),
        },
    );
    for command in [
        register.clone(),
        register,
        request(2, Command::Retire { id: 1 }),
        request(3, Command::Shutdown),
    ] {
        assert_eq!(send(&mut parent, command), Status::Accepted);
    }
    worker.join().unwrap();
}

#[test]
fn partial_frame_timeout_keeps_registered_owner_and_permanently_closes_stream() {
    let (mut parent, child) = Pipe::pair().unwrap();
    let worker = std::thread::spawn(move || {
        let peer = peer();
        let mut exchange = Exchange::accept(child, peer.as_handle(), OPERATION_BUDGET).unwrap();
        assert!(!exchange.step(peer.as_handle(), |_| Ok(())).unwrap());
        exchange.budget = Duration::from_millis(30);
        let error = exchange
            .step(peer.as_handle(), |_| panic!("partial frame cannot bind"))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert_eq!(exchange.service.pending(), 1);
        assert!(exchange.step(peer.as_handle(), |_| Ok(())).is_err());
        assert_eq!(exchange.service.pending(), 1);
    });
    connect(&mut parent);
    assert_eq!(
        send(
            &mut parent,
            request(
                1,
                Command::Register {
                    id: 1,
                    record: record()
                }
            )
        ),
        Status::Accepted
    );
    parent
        .io(Some(Instant::now() + OPERATION_BUDGET), None)
        .write_all(&[16])
        .unwrap();
    worker.join().unwrap();
}

#[test]
fn broken_pipe_does_not_discard_acknowledged_recovery_responsibility() {
    let (mut parent, child) = Pipe::pair().unwrap();
    let worker = std::thread::spawn(move || {
        let peer = peer();
        let mut exchange = Exchange::accept(child, peer.as_handle(), OPERATION_BUDGET).unwrap();
        exchange.step(peer.as_handle(), |_| Ok(())).unwrap();
        assert!(exchange.step(peer.as_handle(), |_| Ok(())).is_err());
        assert_eq!(exchange.service.pending(), 1);
        assert!(exchange.failed);
    });
    connect(&mut parent);
    assert_eq!(
        send(
            &mut parent,
            request(
                1,
                Command::Register {
                    id: 1,
                    record: record()
                }
            )
        ),
        Status::Accepted
    );
    drop(parent);
    worker.join().unwrap();
}

#[test]
fn ack_write_failure_keeps_newly_bound_owner_without_reopening_stream() {
    let (mut parent, child) = Pipe::pair().unwrap();
    let (bound_tx, bound_rx) = std::sync::mpsc::sync_channel(1);
    let (closed_tx, closed_rx) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let peer = peer();
        let mut exchange = Exchange::accept(child, peer.as_handle(), OPERATION_BUDGET).unwrap();
        let result = exchange.step(peer.as_handle(), |_| {
            bound_tx.send(()).unwrap();
            closed_rx.recv_timeout(OPERATION_BUDGET).unwrap();
            Ok(())
        });
        assert!(result.is_err());
        assert!(exchange.failed);
        assert_eq!(exchange.service.pending(), 1);
    });
    connect(&mut parent);
    write_request(
        &mut parent.io(Some(Instant::now() + OPERATION_BUDGET), None),
        &request(
            1,
            Command::Register {
                id: 1,
                record: record(),
            },
        ),
    )
    .unwrap();
    bound_rx.recv_timeout(OPERATION_BUDGET).unwrap();
    drop(parent);
    closed_tx.send(()).unwrap();
    worker.join().unwrap();
}

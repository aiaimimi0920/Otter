//! 使用合成订阅和有界后台任务验证会话，不访问系统历史或注册系统事件。
use super::*;
use crate::clipboard_history::Content;
use std::sync::mpsc;
use windows::{core::Error, Win32::Foundation::E_ABORT};

fn snapshot(status: Status) -> Snapshot {
    Snapshot {
        status,
        entries: vec![Entry {
            id: "synthetic".into(),
            content: Content::Text("text".into()),
        }],
    }
}
fn ready() -> Result<Task> {
    Task::spawn(|_, _permit| Ok(snapshot(Status::Ready)))
}
fn wait(session: &mut Session, now: Instant) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while session.task.is_some() {
        session.tick_with(now, || panic!("Unexpected refresh"));
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn first_load_delivers_once_and_idle_ticks_do_not_read() {
    let now = Instant::now();
    let mut session = Session::new(Events::synthetic(), now);
    assert!(session.tick_with(now, ready));
    assert!(session.take_entry(0, "synthetic").is_none());
    wait(&mut session, now);
    assert_eq!(session.state(), State::Ready);
    assert_eq!(session.entries().len(), 1);
    assert!(!session.tick_with(now, || panic!("Idle must not read")));
    assert!(session.take_entry(9, "synthetic").is_none());
    assert!(session.take_entry(0, "previous-painted-item").is_none());
    assert!(session.take_entry(0, "synthetic").is_some());
    assert!(session.take_entry(0, "synthetic").is_none());
}

#[test]
fn event_after_paint_invalidates_selection_before_next_timer() {
    let now = Instant::now();
    let mut session = Session::new(Events::synthetic(), now);
    session.tick_with(now, ready);
    wait(&mut session, now);
    session.events.signal();
    assert!(session.take_entry(0, "synthetic").is_none());
    assert!(session.entries().is_empty());
    assert_eq!(session.state(), State::Loading);
    session.tick_with(now + Duration::from_secs(1), ready);
    wait(&mut session, now + Duration::from_secs(1));
    assert!(session.take_entry(0, "synthetic").is_some());
}

#[test]
fn events_during_read_discard_old_result_and_coalesce_refresh() {
    let now = Instant::now();
    let mut session = Session::new(Events::synthetic(), now);
    let (send, receive) = mpsc::sync_channel(1);
    session.tick_with(now, || {
        Task::spawn(move |_, _permit| {
            receive.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok(snapshot(Status::Ready))
        })
    });
    for _ in 0..100 {
        session.events.signal();
    }
    session.tick_with(now, || panic!("Must not overlap"));
    send.send(()).unwrap();
    wait(&mut session, now); // 未到刷新节流时间。
    assert!(session.entries().is_empty());
    assert!(session.pending);
    session.tick_with(now + Duration::from_secs(1), ready);
    wait(&mut session, now + Duration::from_secs(1));
    assert_eq!(session.state(), State::Ready);
}

#[test]
fn busy_start_retains_refresh_and_rate_limits_retry() {
    let now = Instant::now();
    let mut session = Session::new(Events::synthetic(), now);
    session.tick_with(now, || Err(Error::new(E_ABORT, "Synthetic busy")));
    assert_eq!(session.state(), State::Unavailable);
    assert!(session.pending);
    assert!(!session.tick_with(now, || panic!("Must throttle retry")));
    session.tick_with(now + Duration::from_secs(1), ready);
    wait(&mut session, now + Duration::from_secs(1));
    assert_eq!(session.state(), State::Ready);
}

#[test]
fn terminal_statuses_and_ui_timeout_are_explicit() {
    let now = Instant::now();
    let mut session = Session::new(Events::synthetic(), now);
    for (status, expected) in [
        (Status::Disabled, State::Disabled),
        (Status::Denied, State::Denied),
    ] {
        session.accept(Ok(Snapshot {
            status,
            entries: Vec::new(),
        }));
        assert_eq!(session.state(), expected);
    }
    session.accept(Ok(Snapshot {
        status: Status::Ready,
        entries: Vec::new(),
    }));
    assert_eq!(session.state(), State::Empty);
    let (send, receive) = mpsc::sync_channel(1);
    let (done_send, done_receive) = mpsc::sync_channel(1);
    session.tick_with(now, || {
        Task::spawn(move |cancel, permit| {
            receive.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(cancel.load(std::sync::atomic::Ordering::Acquire));
            drop(permit);
            done_send.send(()).unwrap();
            Ok(snapshot(Status::Ready))
        })
    });
    session.tick_with(now + Duration::from_secs(6), || {
        panic!("No unrequested retry")
    });
    assert_eq!(session.state(), State::Unavailable);
    assert!(session.task.is_none());
    send.send(()).unwrap();
    done_receive.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(session.take_entry(0, "synthetic").is_none());
}

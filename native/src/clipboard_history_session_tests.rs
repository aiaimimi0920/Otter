//! Test sessions with synthetic subscriptions/bounded background tasks, without system-history access or system-event registration.
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
#[test]
fn delete_rechecks_painted_identity_and_refreshes_only_after_confirmed_completion() {
    let now = Instant::now();
    let mut session = Session::test_snapshot(snapshot(Status::Ready));
    assert!(!session.delete_with(0, "old", |_| panic!("must reject stale identity")));
    assert!(session.delete_with(0, "synthetic", |id| {
        assert_eq!(id, "synthetic");
        ready()
    }));
    assert!(session.take_entry(0, "synthetic").is_none());
    wait(&mut session, now);
    assert!(session.pending);
    assert_eq!(session.state(), State::Loading);
    session.tick_with(now + Duration::from_secs(1), ready);
    wait(&mut session, now + Duration::from_secs(1));
    assert_eq!(session.state(), State::Ready);
    session.test_invalidate();
    assert!(!session.delete_with(0, "synthetic", |_| panic!("must reject changed history")));
}
#[test]
fn continuous_deletes_follow_refreshed_ids_until_the_history_is_empty() {
    let entries = |ids: &[&str]| Snapshot {
        status: Status::Ready,
        entries: ids
            .iter()
            .map(|id| Entry {
                id: (*id).into(),
                content: Content::Text((*id).into()),
            })
            .collect(),
    };
    let mut session = Session::test_snapshot(entries(&["first", "second"]));
    let mut now = Instant::now();
    for (id, remaining) in [("first", vec!["second"]), ("second", vec![])] {
        assert!(session.delete_with(0, id, |actual| {
            assert_eq!(actual, id);
            ready()
        }));
        assert!(!session.delete_with(0, id, |_| panic!("duplicate in-flight delete")));
        wait(&mut session, now);
        let refreshed = entries(&remaining);
        now += Duration::from_secs(1);
        session.tick_with(now, || Task::spawn(move |_, _| Ok(refreshed)));
        wait(&mut session, now);
        assert_eq!(session.entries().len(), remaining.len());
        assert!(!session.delete_with(0, id, |_| panic!("old painted ID")));
    }
    assert_eq!(session.state(), State::Empty);
}
#[test]
fn delete_failure_is_visible_without_retry_even_when_history_changed() {
    let now = Instant::now();
    let mut session = Session::test_snapshot(snapshot(Status::Ready));
    assert!(
        session.delete_with(0, "synthetic", |_| Task::spawn(|_, _| Err(Error::new(
            E_ABORT, "refused"
        ))))
    );
    session.test_invalidate();
    wait(&mut session, now);
    assert_eq!(session.state(), State::DeleteFailed);
    assert!(!session.pending);
    session.tick_with(now + Duration::from_secs(1), || {
        panic!("must not retry deletion")
    });
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
    wait(&mut session, now); // The refresh-throttle interval has not elapsed.
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
    let retries = std::cell::Cell::new(0);
    assert!(!session.tick_with(now + Duration::from_millis(250), || {
        retries.set(retries.get() + 1);
        Err(Error::new(E_ABORT, "Still busy"))
    }));
    assert_eq!(retries.get(), 1);
    assert_eq!(session.state(), State::Unavailable);
    assert!(session.pending);
    assert!(!session.tick_with(now + Duration::from_millis(300), || {
        panic!("Must throttle repeated busy result")
    }));
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

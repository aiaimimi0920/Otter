use super::super::Client;
use super::*;
use crate::live_window_target::{snapshot::Snapshot, Target};
use std::time::{Duration, Instant};
use windows::{core::w, Win32::UI::WindowsAndMessaging::*};

struct Window(HWND);
impl Window {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("journal fixture"),
                WS_POPUP,
                20,
                20,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
    fn record(&self) -> Record {
        Snapshot::read(&Target::bind(self.0).unwrap())
            .unwrap()
            .recovery_record()
            .unwrap()
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
fn journal() -> Journal {
    Journal::with_owner(
        Owner::start_with(|| Client::start_command(super::super::tests::fixture("serve"))).unwrap(),
    )
}
fn advance(journal: &mut Journal, done: impl Fn(&Journal) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        journal.poll();
        if done(journal) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Journal timed out: {:?}",
            journal.fault()
        );
        let handle = journal
            .completion_handle()
            .or_else(|| journal.child_handle())
            .expect("Unfinished journal must have a wait owner");
        let result = unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 1000) };
        assert!(result == WAIT_OBJECT_0 || result == WAIT_TIMEOUT);
    }
}

#[test]
fn registration_and_retirement_require_actual_ack_without_blocking_callback() {
    let window = Window::new();
    let mut journal = journal();
    let ticket = journal.register(window.record()).unwrap();
    assert!(!ticket.ready());
    assert!(journal.register(window.record()).is_err());
    advance(&mut journal, |_| ticket.ready());
    assert_eq!(ticket.retire().unwrap_err().code(), RETIREMENT_PENDING);
    assert!(!ticket.ready());
    advance(&mut journal, |_| ticket.retire().is_ok());
    assert!(ticket.retire().is_ok());
    journal.close();
    advance(&mut journal, Journal::finished);
    assert!(journal.fault().is_none());
}

#[test]
fn cancel_inflight_registration_never_becomes_hide_ready_and_is_retired() {
    let window = Window::new();
    let mut journal = journal();
    let ticket = journal.register(window.record()).unwrap();
    advance(&mut journal, |journal| {
        matches!(journal.pending, Some(Pending::Register(_)))
    });
    assert_eq!(ticket.retire().unwrap_err().code(), RETIREMENT_PENDING);
    assert!(!ticket.ready());
    advance(&mut journal, |_| ticket.retire().is_ok());
    assert!(!ticket.ready());
    journal.close();
    advance(&mut journal, Journal::finished);
    assert!(journal.fault().is_none());
}

#[test]
fn bounded_unsent_records_cancel_on_close_without_registration() {
    let window = Window::new();
    let mut journal = journal();
    let mut tickets = Vec::new();
    for index in 0..MAX_RECORDS {
        let mut record = window.record();
        record.hwnd = index as u64 + 1;
        tickets.push(journal.register(record).unwrap());
    }
    assert!(journal.register(window.record()).is_err());
    journal.close();
    advance(&mut journal, Journal::finished);
    assert!(tickets
        .iter()
        .all(|ticket| !ticket.ready() && ticket.retire().is_ok()));
    assert!(journal.fault().is_none());
}

#[test]
fn last_external_ticket_drop_retires_but_an_intermediate_clone_drop_does_not() {
    let window = Window::new();
    let mut journal = journal();
    let ticket = journal.register(window.record()).unwrap();
    let peer = ticket.clone();
    drop(ticket);
    advance(&mut journal, |_| peer.ready());
    drop(peer);
    journal.close();
    advance(&mut journal, Journal::finished);
    assert!(journal.entries.is_empty());
    assert!(journal.fault().is_none());
}

#[test]
fn startup_failure_disables_tickets_and_finishes_without_a_process() {
    let window = Window::new();
    let mut journal = Journal::with_owner(
        Owner::start_with(|| Err(io::Error::other("fixture failure"))).unwrap(),
    );
    let ticket = journal.register(window.record()).unwrap();
    advance(&mut journal, Journal::finished);
    assert!(!ticket.ready());
    assert!(ticket.retire().is_ok());
    assert!(journal.fault().is_some());
}

#[test]
fn missing_client_handle_does_not_retire_an_uncertain_registration() {
    let window = Window::new();
    let mut journal = Journal::with_owner(
        Owner::start_with(|| Err(io::Error::other("fixture lost client"))).unwrap(),
    );
    let ticket = journal.register(window.record()).unwrap();
    // 模拟 worker 丢失 Client 后的未知 ACK；不创建真正泄漏的恢复子进程。
    ticket.0.state.set(State::Registering);
    advance(&mut journal, |journal| journal.fault().is_some());
    assert!(ticket.0.state.get() == State::Unknown);
    assert!(!ticket.ready());
    assert!(!journal.finished());
    assert!(journal.child_handle().is_none());
    assert_eq!(ticket.retire().unwrap_err().code(), RETIREMENT_PENDING);
    journal.close();
    journal.poll();
    assert!(!journal.finished());
    assert!(journal.register(window.record()).is_err());
}

#[test]
fn dropping_journal_invalidates_a_surviving_ticket() {
    let window = Window::new();
    let mut journal = Journal::with_owner(
        Owner::start_with(|| Err(io::Error::other("fixture no child"))).unwrap(),
    );
    let ticket = journal.register(window.record()).unwrap();
    // 只验证本地 owner 丢失的授权撤销，不启动带未退休记录的真实进程。
    ticket.0.state.set(State::Registered);
    assert!(ticket.ready());
    drop(journal);
    assert!(!ticket.ready());
    assert_eq!(ticket.retire().unwrap_err().code(), RETIREMENT_PENDING);
}

#[test]
fn actual_lease_keeps_snapshot_until_retirement_ack_and_allows_next_generation() {
    use crate::live_window_target::visibility::Registry;

    let window = Window::new();
    let before = window.record();
    let target = Target::bind(window.0).unwrap();
    let mut registry = Registry::default();
    let lease = registry.acquire(target.clone()).unwrap();
    let peer = registry.acquire(target.clone()).unwrap();
    let mut journal = journal();
    let ticket = journal.register(before.clone()).unwrap();
    assert!(lease.hide().is_err());
    advance(&mut journal, |_| ticket.ready());
    let retirement = ticket.clone();
    lease
        .arm(Snapshot::read(&target).unwrap(), move || {
            retirement.retire()
        })
        .unwrap();
    assert!(lease.hide().unwrap());
    assert!(!peer.hide().unwrap());
    assert_eq!(lease.restore().unwrap_err().code(), RETIREMENT_PENDING);
    assert_eq!(window.record(), before);
    assert!(peer.has_recovery());
    assert!(peer.hide().is_err());
    assert!(registry.acquire(target.clone()).is_err());
    advance(&mut journal, |_| ticket.retire().is_ok());
    peer.restore().unwrap();
    assert!(!lease.has_recovery());
    let next = journal.register(before.clone()).unwrap();
    advance(&mut journal, |_| next.ready());
    let retirement = next.clone();
    lease
        .arm(Snapshot::read(&target).unwrap(), move || {
            retirement.retire()
        })
        .unwrap();
    assert!(lease.hide().unwrap());
    drop(lease);
    assert!(peer.has_recovery());
    assert!(!next.0.retire.get());
    drop(peer);
    assert_eq!(window.record(), before);
    assert!(registry.acquire(target.clone()).is_err());
    advance(&mut journal, |_| next.retire().is_ok());
    registry.retry_pending().unwrap();
    assert!(registry.acquire(target).is_ok());
    journal.close();
    advance(&mut journal, Journal::finished);
    assert!(journal.fault().is_none());
}

#[test]
fn fault_shutdown_waits_for_every_source_restore_before_stopping_recovery() {
    use crate::live_window_target::visibility::Registry;

    let window = Window::new();
    let other = Window::new();
    let before = window.record();
    let target = Target::bind(window.0).unwrap();
    let mut registry = Registry::default();
    let lease = registry.acquire(target.clone()).unwrap();
    let mut journal = journal();
    let ticket = journal.register(before.clone()).unwrap();
    let second = journal.register(other.record()).unwrap();
    advance(&mut journal, |_| ticket.ready() && second.ready());
    let retirement = ticket.clone();
    lease
        .arm(Snapshot::read(&target).unwrap(), move || {
            retirement.retire()
        })
        .unwrap();
    lease.hide().unwrap();
    let process = journal.child_handle().unwrap().as_raw_handle();
    // 已建立真实子进程/登记；注入协调层故障，验证它不会误杀仍在保护源的进程。
    journal.fail("fixture poisoned channel".into());
    journal.close();
    journal.poll();
    assert!(!journal.stop_requested);
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(process), 0) },
        WAIT_TIMEOUT
    );
    assert!(lease.has_recovery());
    assert_ne!(window.record(), before);
    assert_eq!(lease.restore().unwrap_err().code(), RETIREMENT_PENDING);
    assert_eq!(window.record(), before);
    journal.poll();
    assert!(!journal.stop_requested, "Other source is still protected");
    assert!(ticket.retire().is_err());
    assert!(second.retire().is_err());
    advance(&mut journal, Journal::finished);
    assert!(journal.stop_requested);
    assert!(journal.fault().is_some());
    assert!(ticket.retire().is_ok() && second.retire().is_ok());
    lease.restore().unwrap();
    assert!(!lease.has_recovery());
    assert_eq!(window.record(), before);
}

#[test]
fn destroyed_source_retires_actual_registration_and_allows_clean_shutdown() {
    use crate::live_window_target::visibility::Registry;
    let window = Window::new();
    let target = Target::bind(window.0).unwrap();
    let snapshot = Snapshot::read(&target).unwrap();
    let mut registry = Registry::default();
    let lease = registry.acquire(target).unwrap();
    let mut journal = journal();
    let ticket = journal
        .register(snapshot.recovery_record().unwrap())
        .unwrap();
    advance(&mut journal, |_| ticket.ready());
    let retirement = ticket.clone();
    lease.arm(snapshot, move || retirement.retire()).unwrap();
    lease.hide().unwrap();
    drop(window);
    drop(lease);
    assert!(registry.retry_pending().is_err());
    journal.close();
    advance(&mut journal, Journal::finished);
    registry.retry_pending().unwrap();
    assert!(ticket.retire().is_ok());
    assert!(journal.fault().is_none());
    assert!(
        !journal.stop_requested,
        "Normal retirement must not need termination"
    );
}

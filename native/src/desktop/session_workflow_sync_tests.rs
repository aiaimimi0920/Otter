use super::*;
#[test]
fn admission_is_per_request_and_dropped_reply_denies() {
    for allow in [Some(true), Some(false), None] {
        let (tx, rx) = mpsc::sync_channel(1);
        let (wake, notified) = mpsc::channel();
        let notify: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            wake.send(()).unwrap();
        });
        let worker = std::thread::spawn(move || {
            admit(&tx, "request", "workflow", &AtomicBool::new(false), &notify)
        });
        notified.recv_timeout(Duration::from_secs(2)).unwrap();
        let admission = rx.try_recv().unwrap();
        assert_eq!(admission.request_id(), "request");
        assert_eq!(admission.workflow_id(), "workflow");
        if let Some(allow) = allow {
            admission.resolve(allow);
        } else {
            drop(admission);
        }
        assert_eq!(worker.join().unwrap(), allow == Some(true));
    }
}
#[test]
fn cancellation_and_disconnected_ui_cannot_authorize_send() {
    let (tx, rx) = mpsc::sync_channel(1);
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let (wake, notified) = mpsc::channel();
    let notify: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        wake.send(()).unwrap();
    });
    let worker = std::thread::spawn(move || admit(&tx, "request", "workflow", &signal, &notify));
    notified.recv_timeout(Duration::from_secs(2)).unwrap();
    let admission = rx.try_recv().unwrap();
    cancel.store(true, Ordering::Release);
    admission.resolve(true);
    assert!(!worker.join().unwrap());
    let (tx, rx) = mpsc::sync_channel(1);
    drop(rx);
    let notify: Arc<dyn Fn() + Send + Sync> =
        Arc::new(|| panic!("No notification without mailbox"));
    assert!(!admit(
        &tx,
        "request",
        "workflow",
        &AtomicBool::new(false),
        &notify
    ));
}

use super::*;
use std::{sync::mpsc, time::Duration};
#[test]
fn retired_inflight_notifications_do_not_consume_new_subscription_slots() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let source = Source::<u8>::new();
    let retired_calls = Arc::new(AtomicUsize::new(0));
    let mut consumers = Vec::new();
    for _ in 0..MAX_CONSUMERS {
        let mut consumer = source.subscribe().unwrap();
        let calls = retired_calls.clone();
        consumer
            .set_waker(move || {
                calls.fetch_add(1, Ordering::Relaxed);
                true
            })
            .unwrap();
        consumers.push(consumer);
    }
    let inflight = observers(&source.shared.lock());
    drop(consumers);
    let mut replacements = Vec::new();
    for _ in 0..MAX_CONSUMERS {
        let mut consumer = source.subscribe().unwrap();
        consumer.set_waker(|| true).unwrap();
        replacements.push(consumer);
    }
    for wake in inflight.into_iter().flatten() {
        wake.notify();
    }
    source.publish(1).unwrap();
    assert_eq!(retired_calls.load(Ordering::Relaxed), 0);
}
#[test]
fn notification_is_coalesced_reentrant_and_rearmed_after_read() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let source = Arc::new(Source::new());
    let mut consumer = source.subscribe().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let weak = Arc::downgrade(&source);
    consumer
        .set_waker(move || {
            let source = weak.upgrade().unwrap();
            assert!(source.shared.state.try_lock().is_ok());
            assert!(source.has_consumers());
            count.fetch_add(1, Ordering::Relaxed);
            true
        })
        .unwrap();
    for value in 0..100 {
        source.publish(value).unwrap();
    }
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(consumer.try_next().unwrap().unwrap().frame.resource, 99);
    source.publish(100).unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    assert!(consumer.set_waker(|| panic!("duplicate notifier")).is_err());
    drop(consumer);
    source.close();
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}
#[test]
fn failed_notification_can_retry_and_binding_after_frame_notifies_immediately() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let source = Source::new();
    let mut consumer = source.subscribe().unwrap();
    source.publish(1).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    consumer
        .set_waker(move || count.fetch_add(1, Ordering::Relaxed) != 0)
        .unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    source.publish(2).unwrap();
    assert_eq!(consumer.notification_failures(), 1);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    source.publish(3).unwrap();
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    assert_eq!(consumer.try_next().unwrap().unwrap().frame.resource, 3);
}
#[test]
fn last_consumer_wakes_owner_outside_lock_and_duplicate_owner_is_rejected() {
    let source = Arc::new(Source::<u8>::new());
    let first = source.subscribe().unwrap();
    let second = source.subscribe().unwrap();
    let weak = Arc::downgrade(&source);
    let (send, receive) = mpsc::channel();
    source
        .on_idle(Arc::new(move || {
            if let Some(source) = weak.upgrade() {
                assert!(!source.has_consumers());
            }
            send.send(()).unwrap();
        }))
        .unwrap();
    assert_eq!(
        source.on_idle(Arc::new(|| panic!("duplicate owner"))),
        Err(Error::OwnerConflict)
    );
    drop(first);
    assert!(receive.try_recv().is_err());
    drop(second);
    receive.recv_timeout(Duration::from_secs(2)).unwrap();
    source.close();
    receive.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(source.on_idle(Arc::new(|| {})), Err(Error::Closed));
}
#[test]
fn independent_consumers_share_latest_without_queues_or_destructive_reads() {
    let source = Source::new();
    let mut first = source.subscribe().unwrap();
    let mut second = source.subscribe().unwrap();
    source.publish(vec![1u8; 16]).unwrap();
    let frame = first.try_next().unwrap().unwrap();
    let other = second.try_next().unwrap().unwrap();
    assert!(Arc::ptr_eq(&frame.frame, &other.frame));
    assert_eq!(frame.skipped, 0);
    assert!(first.try_next().unwrap().is_none());
    source.publish(vec![2; 16]).unwrap();
    source.publish(vec![3; 16]).unwrap();
    let latest = second.try_next().unwrap().unwrap();
    assert_eq!(latest.frame.resource[0], 3);
    assert_eq!(latest.skipped, 1);
    assert_eq!(frame.frame.resource[0], 1);
    let mut late = source.subscribe().unwrap();
    assert_eq!(late.try_next().unwrap().unwrap().skipped, 0);
}
#[test]
fn close_wakes_blocked_consumer_and_cannot_publish_or_subscribe_again() {
    let source = Source::<u8>::new();
    let mut consumer = source.subscribe().unwrap();
    let (entered, ready) = mpsc::channel();
    let (done, result) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        entered.send(()).unwrap();
        done.send(consumer.wait_next().err()).unwrap();
    });
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    drop(source);
    assert_eq!(
        result.recv_timeout(Duration::from_secs(2)).unwrap(),
        Some(Error::Closed)
    );
    worker.join().unwrap();
    let closed = Source::new();
    closed.close();
    closed.close();
    assert!(matches!(closed.subscribe(), Err(Error::Closed)));
    assert_eq!(closed.publish(1), Err(Error::Closed));
}
#[test]
fn no_consumers_retains_no_resources_and_limit_is_reusable() {
    let source = Source::new();
    assert_eq!(source.publish(Arc::new(1)), Err(Error::NoConsumers));
    let mut consumers = (0..MAX_CONSUMERS)
        .map(|_| source.subscribe().unwrap())
        .collect::<Vec<_>>();
    assert!(source.has_consumers());
    assert!(matches!(source.subscribe(), Err(Error::ConsumerLimit)));
    consumers.pop();
    consumers.push(source.subscribe().unwrap());
    let resource = Arc::new(9);
    let weak = Arc::downgrade(&resource);
    source.publish(resource).unwrap();
    drop(consumers);
    assert!(!source.has_consumers());
    assert!(weak.upgrade().is_none());
    let mut next = source.subscribe().unwrap();
    assert!(next.try_next().unwrap().is_none());
    source.publish(Arc::new(10)).unwrap();
    assert_eq!(next.try_next().unwrap().unwrap().skipped, 0);
}
#[test]
fn wake_delivers_published_frame_and_sequence_never_wraps() {
    let source = Source::new();
    let mut consumer = source.subscribe().unwrap();
    let worker = std::thread::spawn(move || consumer.wait_next().unwrap().frame.resource);
    source.publish(17).unwrap();
    assert_eq!(worker.join().unwrap(), 17);
    let _consumer = source.subscribe().unwrap();
    source.shared.lock().sequence = u64::MAX;
    assert_eq!(source.publish(18), Err(Error::SequenceExhausted));
}

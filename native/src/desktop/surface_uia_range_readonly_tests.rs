//! MTA subscriptions to read-only properties; UI owners modify edit policy without clients borrowing window state.
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};
use windows::{
    core::{implement, Ref},
    Win32::System::Variant::VARIANT,
};
type Evidence = std::result::Result<(i32, String, bool), windows::core::HRESULT>;
#[implement(IUIAutomationPropertyChangedEventHandler)]
struct Handler {
    sender: SyncSender<Evidence>,
    dropped: Arc<AtomicBool>,
    overflow: Arc<AtomicBool>,
}
impl IUIAutomationPropertyChangedEventHandler_Impl for Handler_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        property: UIA_PROPERTY_ID,
        value: &VARIANT,
    ) -> Result<()> {
        let result = (|| unsafe {
            let sender = sender
                .as_ref()
                .ok_or_else(|| windows::core::Error::from_hresult(E_POINTER))?;
            Ok((
                property.0,
                sender.CachedAutomationId()?.to_string(),
                bool::try_from(value)?,
            ))
        })()
        .map_err(|error: windows::core::Error| error.code());
        if self.sender.try_send(result).is_err() {
            self.overflow.store(true, Ordering::Release);
        }
        Ok(())
    }
}
impl Drop for Handler {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}
struct Subscription {
    client: IUIAutomation2,
    element: IUIAutomationElement,
    handler: IUIAutomationPropertyChangedEventHandler,
}
impl Drop for Subscription {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .client
                .RemovePropertyChangedEventHandler(&self.element, &self.handler);
        }
    }
}
fn expect(receiver: &Receiver<Evidence>, expected: bool, previous: Option<bool>) {
    let deadline = Instant::now() + Duration::from_secs(3);
    for _ in 0..8 {
        let value = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("read-only notification missing")
            .unwrap();
        assert_eq!(value.0, UIA_RangeValueIsReadOnlyPropertyId.0);
        assert_eq!(value.1, "slider");
        if value.2 == expected {
            return;
        }
        // UIA callbacks are asynchronous and may repeat a prior raise; never extend the deadline.
        assert_eq!(Some(value.2), previous, "unexpected read-only state");
    }
    panic!("unbounded read-only duplicates");
}
fn receive(hwnd: usize, commands: SyncSender<(bool, bool)>, acks: Receiver<u32>) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _com = Apartment;
    let (sender, receiver) = sync_channel(8);
    let dropped = Arc::new(AtomicBool::new(false));
    let overflow = Arc::new(AtomicBool::new(false));
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let condition =
            client.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from("slider"))?;
        let element = root.FindFirst(TreeScope_Descendants, &condition)?;
        let cache = client.CreateCacheRequest()?;
        cache.AddProperty(UIA_AutomationIdPropertyId)?;
        let handler: IUIAutomationPropertyChangedEventHandler = Handler {
            sender,
            dropped: dropped.clone(),
            overflow: overflow.clone(),
        }
        .into();
        client.AddPropertyChangedEventHandlerNativeArray(
            &element,
            TreeScope_Element,
            &cache,
            &handler,
            &[UIA_RangeValueIsReadOnlyPropertyId],
        )?;
        let subscription = Subscription {
            client,
            element,
            handler,
        };
        let pattern: IUIAutomationRangeValuePattern = subscription
            .element
            .GetCurrentPatternAs(UIA_RangeValuePatternId)?;
        let policy = |edit, publish| {
            commands.send((edit, publish)).unwrap();
            acks.recv_timeout(Duration::from_secs(3)).unwrap()
        };
        assert!(!pattern.CurrentIsReadOnly()?.as_bool());
        let initial = policy(true, false);
        assert!(!pattern.CurrentIsReadOnly()?.as_bool());
        policy(false, false);
        expect(&receiver, true, None);
        assert!(pattern.CurrentIsReadOnly()?.as_bool());
        assert!(pattern.SetValue(60.0).is_err());
        assert_eq!(pattern.CurrentValue()?, 50.0);
        // This ACK is after the first real callback, hence its raise has already occurred.
        // A quiet callback interval cannot prove no-op behavior; check the UI owner's source count.
        assert_eq!(policy(false, false), initial + 1);
        policy(true, true);
        expect(&receiver, false, Some(true));
        assert!(!pattern.CurrentIsReadOnly()?.as_bool());
        let _client = subscription.client.clone();
        drop(subscription);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "read-only handler retained"
        );
        assert!(!overflow.load(Ordering::Acquire), "read-only evidence lost");
    }
    Ok(())
}
#[test]
fn uia_range_readonly_policy_notifications_and_soft_stop_semantics() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = super::super::range_tests::mount();
    let (commands, requests) = sync_channel(1);
    let (ack, acks) = sync_channel(1);
    let emitted = super::super::abi::readonly_notifications();
    query_with(
        window.hwnd.0 as usize,
        move |hwnd| receive(hwnd, commands, acks),
        || {
            if let Ok((edit, publish)) = requests.try_recv() {
                window.set_input_policy(edit, publish).unwrap();
                ack.send(super::super::abi::readonly_notifications())
                    .unwrap();
            }
        },
    );
    assert_eq!(super::super::abi::readonly_notifications(), emitted + 2);
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}

#[test]
fn readonly_callback_sequence_accepts_only_bounded_matching_history() {
    let (sender, receiver) = sync_channel(8);
    for value in [true, true, false] {
        sender
            .send(Ok((
                UIA_RangeValueIsReadOnlyPropertyId.0,
                "slider".into(),
                value,
            )))
            .unwrap();
    }
    expect(&receiver, true, None);
    expect(&receiver, false, Some(true));
    sender
        .send(Ok((
            UIA_RangeValueIsReadOnlyPropertyId.0,
            "foreign".into(),
            false,
        )))
        .unwrap();
    assert!(std::panic::catch_unwind(|| expect(&receiver, false, Some(true))).is_err());
    for _ in 0..8 {
        sender
            .send(Ok((
                UIA_RangeValueIsReadOnlyPropertyId.0,
                "slider".into(),
                true,
            )))
            .unwrap();
    }
    assert!(std::panic::catch_unwind(|| expect(&receiver, false, Some(true))).is_err());
    sender.send(Err(E_FAIL)).unwrap();
    assert!(std::panic::catch_unwind(|| expect(&receiver, false, Some(true))).is_err());
}

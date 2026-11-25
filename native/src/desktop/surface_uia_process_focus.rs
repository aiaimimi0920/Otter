//! System-focus subscriptions filter self-started process PID first; no recording other applications' node identities or stealing foreground.
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};
use windows::core::{implement, Ref};
#[implement(IUIAutomationFocusChangedEventHandler)]
struct Handler {
    pid: u32,
    sender: SyncSender<String>,
    dropped: Arc<AtomicBool>,
}
impl IUIAutomationFocusChangedEventHandler_Impl for Handler_Impl {
    fn HandleFocusChangedEvent(&self, sender: Ref<'_, IUIAutomationElement>) -> Result<()> {
        if let Some(element) = sender.as_ref() {
            unsafe {
                // Focus subscriptions are system-global; check only cached PID, never querying other applications' live properties.
                if element.CachedProcessId().ok().map(|pid| pid as u32) != Some(self.pid) {
                    return Ok(());
                }
                if let Ok(id) = element.CachedAutomationId() {
                    let id = id.to_string();
                    if id == "switch" || id == "button" {
                        let _ = self.sender.try_send(id);
                    }
                }
            }
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
    handler: Option<IUIAutomationFocusChangedEventHandler>,
}
impl Subscription {
    fn remove(&mut self) -> Result<()> {
        if let Some(handler) = &self.handler {
            unsafe {
                self.client.RemoveFocusChangedEventHandler(handler)?;
            }
            self.handler = None;
        }
        Ok(())
    }
}
impl Drop for Subscription {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}
fn expect(receiver: &Receiver<String>, expected: &str) {
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("cross-process focus event timed out"),
        expected
    );
    for _ in 0..8 {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(id) => assert_eq!(id, expected),
            Err(_) => break,
        }
    }
}
pub(super) fn verify(
    client: &IUIAutomation2,
    switch: &IUIAutomationElement,
    button: &IUIAutomationElement,
    pid: u32,
) {
    let (sender, receiver) = sync_channel(8);
    let dropped = Arc::new(AtomicBool::new(false));
    unsafe {
        let cache = client.CreateCacheRequest().unwrap();
        cache.AddProperty(UIA_ProcessIdPropertyId).unwrap();
        cache.AddProperty(UIA_AutomationIdPropertyId).unwrap();
        let handler: IUIAutomationFocusChangedEventHandler = Handler {
            pid,
            sender,
            dropped: dropped.clone(),
        }
        .into();
        client
            .AddFocusChangedEventHandler(&cache, &handler)
            .unwrap();
        let mut subscription = Subscription {
            client: client.clone(),
            handler: Some(handler),
        };
        switch.SetFocus().unwrap();
        expect(&receiver, "switch");
        assert!(switch.CurrentHasKeyboardFocus().unwrap().as_bool());
        assert!(!button.CurrentHasKeyboardFocus().unwrap().as_bool());
        button.SetFocus().unwrap();
        expect(&receiver, "button");
        assert!(button.CurrentHasKeyboardFocus().unwrap().as_bool());
        assert!(!switch.CurrentHasKeyboardFocus().unwrap().as_bool());
        subscription.remove().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "cross-process focus handler retained"
        );
        switch.SetFocus().unwrap();
        assert!(switch.CurrentHasKeyboardFocus().unwrap().as_bool());
        assert!(receiver.recv_timeout(Duration::from_millis(250)).is_err());
        // Restore focus to the button the caller will operate next, without activating desktop foreground.
        button.SetFocus().unwrap();
    }
}

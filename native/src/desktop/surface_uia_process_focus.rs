//! 系统焦点订阅先按自启进程 PID 过滤；不记录其他应用的节点身份或抢占前台。
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
                // 焦点订阅是系统全局接口，只检查 cache PID，不查询其他应用实时属性。
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
        // 恢复调用者即将操作的按钮焦点，不激活桌面前台。
        button.SetFocus().unwrap();
    }
}

//! 跨进程订阅 owner：回调只读缓存，证据有界传递，注销先于 COM apartment 退出。
use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};
use windows::core::{implement, Ref};
type Evidence = std::result::Result<(i32, String, i32), windows::core::HRESULT>;
#[implement(IUIAutomationEventHandler, IUIAutomationPropertyChangedEventHandler)]
struct Handler {
    sender: SyncSender<Evidence>,
    dropped: Arc<AtomicUsize>,
}
impl Handler {
    fn record(&self, sender: Ref<'_, IUIAutomationElement>, id: i32, value: Result<i32>) {
        let evidence = (|| unsafe {
            let element = sender
                .as_ref()
                .ok_or_else(|| windows::core::Error::from_hresult(E_POINTER))?;
            Ok((id, element.CachedAutomationId()?.to_string(), value?))
        })()
        .map_err(|error: windows::core::Error| error.code());
        let _ = self.sender.try_send(evidence);
    }
}
impl IUIAutomationEventHandler_Impl for Handler_Impl {
    fn HandleAutomationEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        event: UIA_EVENT_ID,
    ) -> Result<()> {
        self.record(sender, event.0, Ok(0));
        Ok(())
    }
}
impl IUIAutomationPropertyChangedEventHandler_Impl for Handler_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        property: UIA_PROPERTY_ID,
        value: &VARIANT,
    ) -> Result<()> {
        self.record(sender, property.0, i32::try_from(value));
        Ok(())
    }
}
impl Drop for Handler {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::Release);
    }
}
pub(super) struct Subscription {
    client: IUIAutomation2,
    switch: IUIAutomationElement,
    button: IUIAutomationElement,
    property: Option<IUIAutomationPropertyChangedEventHandler>,
    invoked: Option<IUIAutomationEventHandler>,
    receiver: Receiver<Evidence>,
    dropped: Arc<AtomicUsize>,
}
impl Subscription {
    pub fn new(
        client: &IUIAutomation2,
        switch: &IUIAutomationElement,
        button: &IUIAutomationElement,
    ) -> Result<Self> {
        let (sender, receiver) = sync_channel(8);
        let dropped = Arc::new(AtomicUsize::new(0));
        let mut owner = Self {
            client: client.clone(),
            switch: switch.clone(),
            button: button.clone(),
            property: None,
            invoked: None,
            receiver,
            dropped: dropped.clone(),
        };
        unsafe {
            let cache = client.CreateCacheRequest()?;
            cache.AddProperty(UIA_AutomationIdPropertyId)?;
            let property: IUIAutomationPropertyChangedEventHandler = Handler {
                sender: sender.clone(),
                dropped: dropped.clone(),
            }
            .into();
            client.AddPropertyChangedEventHandlerNativeArray(
                switch,
                TreeScope_Element,
                &cache,
                &property,
                &[UIA_ToggleToggleStatePropertyId],
            )?;
            owner.property = Some(property);
            let invoked: IUIAutomationEventHandler = Handler { sender, dropped }.into();
            client.AddAutomationEventHandler(
                UIA_Invoke_InvokedEventId,
                button,
                TreeScope_Element,
                &cache,
                &invoked,
            )?;
            owner.invoked = Some(invoked);
        }
        Ok(owner)
    }
    pub fn expect(&self, id: i32, node: &str, value: i32) {
        let expected = (id, node.to_owned(), value);
        assert_eq!(
            self.receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("cross-process UIA event timed out")
                .unwrap(),
            expected
        );
        // 系统可能重复交付同一 raise；核对每条身份/载荷，不把回调数当动作数。
        for _ in 0..8 {
            match self.receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(item) => assert_eq!(item.unwrap(), expected),
                Err(_) => break,
            }
        }
    }
    pub fn expect_quiet(&self) {
        assert!(
            matches!(
                self.receiver.recv_timeout(Duration::from_millis(250)),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ),
            "old subscription received a new-instance event or lost its channel"
        );
    }
    fn remove(&mut self) -> Result<()> {
        unsafe {
            if let Some(handler) = &self.property {
                self.client
                    .RemovePropertyChangedEventHandler(&self.switch, handler)?;
                self.property = None;
            }
            if let Some(handler) = &self.invoked {
                self.client.RemoveAutomationEventHandler(
                    UIA_Invoke_InvokedEventId,
                    &self.button,
                    handler,
                )?;
                self.invoked = None;
            }
        }
        Ok(())
    }
    pub fn finish(mut self) {
        self.remove().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while self.dropped.load(Ordering::Acquire) != 2 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            self.dropped.load(Ordering::Acquire),
            2,
            "cross-process handlers retained after unsubscribe"
        );
    }
}
impl Drop for Subscription {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

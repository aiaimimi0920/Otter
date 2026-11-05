//! 跨进程稳定根结构通知：订阅完成后才请求子进程 UI owner 替换场景。
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, SyncSender},
    Arc,
};
use windows::core::{implement, Ref};
type Evidence = std::result::Result<(i32, String), windows::core::HRESULT>;
#[implement(IUIAutomationStructureChangedEventHandler)]
struct Handler {
    sender: SyncSender<Evidence>,
    dropped: Arc<AtomicBool>,
}
impl IUIAutomationStructureChangedEventHandler_Impl for Handler_Impl {
    fn HandleStructureChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        change: StructureChangeType,
        _runtime: *const SAFEARRAY,
    ) -> Result<()> {
        let result = (|| unsafe {
            let element = sender
                .as_ref()
                .ok_or_else(|| windows::core::Error::from_hresult(E_POINTER))?;
            Ok((change.0, element.CachedName()?.to_string()))
        })()
        .map_err(|error: windows::core::Error| error.code());
        let _ = self.sender.try_send(result);
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
    root: IUIAutomationElement,
    handler: Option<IUIAutomationStructureChangedEventHandler>,
}
impl Subscription {
    fn remove(&mut self) -> Result<()> {
        if let Some(handler) = &self.handler {
            unsafe {
                self.client
                    .RemoveStructureChangedEventHandler(&self.root, handler)?;
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
pub(super) fn verify(
    client: &IUIAutomation2,
    root: &IUIAutomationElement,
    fixture: &mut fixture::Fixture,
) {
    let (sender, receiver) = sync_channel(8);
    let dropped = Arc::new(AtomicBool::new(false));
    unsafe {
        let cache = client.CreateCacheRequest().unwrap();
        cache.AddProperty(UIA_NamePropertyId).unwrap();
        let handler: IUIAutomationStructureChangedEventHandler = Handler {
            sender,
            dropped: dropped.clone(),
        }
        .into();
        client
            .AddStructureChangedEventHandler(root, TreeScope_Element, &cache, &handler)
            .unwrap();
        let mut subscription = Subscription {
            client: client.clone(),
            root: root.clone(),
            handler: Some(handler),
        };
        fixture.replace();
        let expected = (
            StructureChangeType_ChildrenInvalidated.0,
            "Otter Surface".to_owned(),
        );
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("cross-process structure event timed out")
                .unwrap(),
            expected
        );
        let replacement = find(client, root, "replacement").unwrap();
        assert_eq!(replacement.CurrentName().unwrap().to_string(), "New tree");
        let value: IUIAutomationValuePattern =
            replacement.GetCurrentPatternAs(UIA_ValuePatternId).unwrap();
        assert_eq!(value.CurrentValue().unwrap().to_string(), "updated");
        for id in ["input", "switch", "button"] {
            let condition = client
                .CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(id))
                .unwrap();
            assert_eq!(
                root.FindAll(TreeScope_Descendants, &condition)
                    .unwrap()
                    .Length()
                    .unwrap(),
                0
            );
        }
        // 同一次通知可能重复交付，所有重复项都应指向稳定根并具有同一类型。
        for _ in 0..8 {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(item) => assert_eq!(item.unwrap(), expected),
                Err(_) => break,
            }
        }
        subscription.remove().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "cross-process structure handler retained"
        );
    }
}

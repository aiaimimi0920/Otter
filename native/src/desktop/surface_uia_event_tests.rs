//! 真正 MTA 订阅 Invoked；回调只读取 cache，并通过有界 channel 交还证据。
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, SyncSender},
    Arc,
};
use windows::core::{implement, Ref};

type Evidence = std::result::Result<(i32, String, String), windows::core::HRESULT>;
#[implement(IUIAutomationEventHandler)]
struct Handler {
    sender: SyncSender<Evidence>,
    dropped: Arc<AtomicBool>,
}
impl IUIAutomationEventHandler_Impl for Handler_Impl {
    fn HandleAutomationEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        eventid: UIA_EVENT_ID,
    ) -> Result<()> {
        let evidence = (|| unsafe {
            let sender = sender
                .as_ref()
                .ok_or_else(|| windows::core::Error::from_hresult(E_POINTER))?;
            Ok((
                eventid.0,
                sender.CachedAutomationId()?.to_string(),
                sender.CachedName()?.to_string(),
            ))
        })()
        .map_err(|error: windows::core::Error| error.code());
        // COM 回调不能阻塞 UIA 线程，也不保存 sender 或访问 UI 线程 State。
        let _ = self.sender.try_send(evidence);
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
    handler: IUIAutomationEventHandler,
    active: bool,
}
impl Subscription {
    fn remove(&mut self) -> Result<()> {
        if self.active {
            unsafe {
                self.client.RemoveAutomationEventHandler(
                    UIA_Invoke_InvokedEventId,
                    &self.element,
                    &self.handler,
                )?;
            }
            self.active = false;
        }
        Ok(())
    }
}
impl Drop for Subscription {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}
fn receive(hwnd: usize) -> Result<(i32, String, String)> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _apartment = Apartment;
    let (send, recv) = sync_channel(4);
    let dropped = Arc::new(AtomicBool::new(false));
    let evidence = unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        client.SetAutoSetFocus(false)?;
        let window = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let condition = client.CreatePropertyCondition(
            UIA_AutomationIdPropertyId,
            &windows::Win32::System::Variant::VARIANT::from("button"),
        )?;
        let element = window.FindFirst(TreeScope_Descendants, &condition)?;
        let cache = client.CreateCacheRequest()?;
        cache.AddProperty(UIA_AutomationIdPropertyId)?;
        cache.AddProperty(UIA_NamePropertyId)?;
        let handler: IUIAutomationEventHandler = Handler {
            sender: send,
            dropped: dropped.clone(),
        }
        .into();
        client.AddAutomationEventHandler(
            UIA_Invoke_InvokedEventId,
            &element,
            TreeScope_Element,
            &cache,
            &handler,
        )?;
        let mut subscription = Subscription {
            client,
            element,
            handler,
            active: true,
        };
        let invoke: IUIAutomationInvokePattern = subscription
            .element
            .GetCurrentPatternAs(UIA_InvokePatternId)?;
        let emitted = super::super::abi::invoke_notifications();
        invoke.Invoke()?;
        let evidence = recv
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| windows::core::Error::new(E_FAIL, "UIA Invoked event timed out"))?
            .map_err(windows::core::Error::from_hresult)?;
        assert_eq!(super::super::abi::invoke_notifications(), emitted + 1);
        assert_eq!(
            invoke.Invoke().unwrap_err().code().0,
            UIA_E_INVALIDOPERATION as i32
        );
        assert_eq!(super::super::abi::invoke_notifications(), emitted + 1);
        // 系统可能重复交付同一次 raise；逐个核验身份，不把回调次数当动作次数。
        for _ in 0..4 {
            match recv.recv_timeout(Duration::from_millis(100)) {
                Ok(value) => assert_eq!(value.unwrap(), evidence),
                Err(_) => break,
            }
        }
        subscription.remove()?;
        let element = subscription.element.clone();
        let _client = subscription.client.clone();
        drop(subscription);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "UIA handler retained after unsubscribe"
        );
        // 明确切焦撤销首个未消费候选，再触发同一节点；不执行任何远端动作。
        element.SetFocus()?;
        invoke.Invoke()?;
        let extra = recv.recv_timeout(Duration::from_millis(250));
        assert!(
            extra.is_err(),
            "event delivered after unsubscribe: {extra:?}"
        );
        evidence
    };
    Ok(evidence)
}

#[test]
fn uia_invoke_system_event_delivers_cached_identity_and_releases_subscription() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(
        br#"{"id":"button","type":"button","props":{"label":"Run"},"events":{"click":"run"}}"#,
    )
    .unwrap();
    let window = super::super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
    }
    assert_eq!(
        query(window.hwnd.0 as usize, receive),
        (UIA_Invoke_InvokedEventId.0, "button".into(), "Run".into())
    );
    assert_eq!(window.take_click().unwrap().node_id, "button");
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}

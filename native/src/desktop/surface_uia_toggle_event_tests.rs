//! 真实 MTA ToggleState 订阅：缓存身份、二态载荷、拒绝不发送及 COM 清理。
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};
use windows::{
    core::{implement, Ref},
    Win32::{
        System::Variant::VARIANT,
        UI::Input::KeyboardAndMouse::{EnableWindow, VK_SPACE},
    },
};
type Evidence = std::result::Result<(i32, String, i32), windows::core::HRESULT>;

#[implement(IUIAutomationPropertyChangedEventHandler)]
struct Handler {
    sender: SyncSender<Evidence>,
    dropped: Arc<AtomicBool>,
}
impl IUIAutomationPropertyChangedEventHandler_Impl for Handler_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        property: UIA_PROPERTY_ID,
        value: &VARIANT,
    ) -> Result<()> {
        let evidence = (|| unsafe {
            let sender = sender
                .as_ref()
                .ok_or_else(|| windows::core::Error::from_hresult(E_POINTER))?;
            Ok((
                property.0,
                sender.CachedAutomationId()?.to_string(),
                i32::try_from(value)?,
            ))
        })()
        .map_err(|error: windows::core::Error| error.code());
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
    handler: IUIAutomationPropertyChangedEventHandler,
    active: bool,
}
impl Subscription {
    fn remove(&mut self) -> Result<()> {
        if self.active {
            unsafe {
                self.client
                    .RemovePropertyChangedEventHandler(&self.element, &self.handler)?;
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
fn expect(receiver: &Receiver<Evidence>, expected: ToggleState) -> Result<()> {
    let item = receiver
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| windows::core::Error::new(E_FAIL, "UIA ToggleState event timed out"))?
        .map_err(windows::core::Error::from_hresult)?;
    assert_eq!(
        item,
        (
            UIA_ToggleToggleStatePropertyId.0,
            "switch".into(),
            expected.0
        )
    );
    // 默认 UIA 可能重复交付，重复项也必须有同一状态与身份；循环有界。
    for _ in 0..4 {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(value) => assert_eq!(value.unwrap(), item),
            Err(_) => break,
        }
    }
    Ok(())
}
fn receive(hwnd: usize) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _apartment = Apartment;
    let (send, recv) = sync_channel(4);
    let dropped = Arc::new(AtomicBool::new(false));
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        client.SetAutoSetFocus(false)?;
        let hwnd = HWND(hwnd as *mut _);
        let window = client.ElementFromHandle(hwnd)?;
        let condition =
            client.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from("switch"))?;
        let element = window.FindFirst(TreeScope_Descendants, &condition)?;
        let cache = client.CreateCacheRequest()?;
        cache.AddProperty(UIA_AutomationIdPropertyId)?;
        let handler: IUIAutomationPropertyChangedEventHandler = Handler {
            sender: send,
            dropped: dropped.clone(),
        }
        .into();
        client.AddPropertyChangedEventHandlerNativeArray(
            &element,
            TreeScope_Element,
            &cache,
            &handler,
            &[UIA_ToggleToggleStatePropertyId],
        )?;
        let mut subscription = Subscription {
            client,
            element,
            handler,
            active: true,
        };
        let pattern: IUIAutomationTogglePattern = subscription
            .element
            .GetCurrentPatternAs(UIA_TogglePatternId)?;
        let emitted = super::super::abi::toggle_notifications();
        assert_eq!(pattern.CurrentToggleState()?, ToggleState_Off);
        pattern.Toggle()?;
        expect(&recv, ToggleState_On)?;
        assert_eq!(pattern.CurrentToggleState()?, ToggleState_On);
        pattern.Toggle()?;
        expect(&recv, ToggleState_Off)?;
        assert_eq!(super::super::abi::toggle_notifications(), emitted + 2);
        // 仅向自有窗口分派输入消息，验证物理输入处理路径共用通知出口。
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(LPARAM(10 | (9 << 16))));
        SendMessageW(hwnd, WM_LBUTTONUP, None, Some(LPARAM(10 | (9 << 16))));
        expect(&recv, ToggleState_On)?;
        assert_eq!(pattern.CurrentToggleState()?, ToggleState_On);
        subscription.element.SetFocus()?;
        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_SPACE.0 as usize)), None);
        SendMessageW(hwnd, WM_KEYUP, Some(WPARAM(VK_SPACE.0 as usize)), None);
        expect(&recv, ToggleState_Off)?;
        assert_eq!(pattern.CurrentToggleState()?, ToggleState_Off);
        assert_eq!(super::super::abi::toggle_notifications(), emitted + 4);
        // 仅禁用本测试拥有的 HWND；不操作其他程序或注入桌面输入。
        let _ = EnableWindow(hwnd, false);
        let rejected = pattern.Toggle();
        let _ = EnableWindow(hwnd, true);
        assert_eq!(
            rejected.unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        assert_eq!(pattern.CurrentToggleState()?, ToggleState_Off);
        assert_eq!(super::super::abi::toggle_notifications(), emitted + 4);
        subscription.remove()?;
        let _client = subscription.client.clone();
        drop(subscription);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "UIA property handler retained after unsubscribe"
        );
        pattern.Toggle()?;
        assert_eq!(pattern.CurrentToggleState()?, ToggleState_On);
        assert!(recv.recv_timeout(Duration::from_millis(250)).is_err());
    }
    Ok(())
}
#[test]
fn uia_toggle_system_property_event_delivers_state_and_releases_subscription() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(
        br#"{"id":"switch","type":"switch","props":{"value":false},"events":{"change":"change"}}"#,
    )
    .unwrap();
    let window = super::super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, false).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
    }
    query(window.hwnd.0 as usize, receive);
    assert!(window.has_pending_edit());
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}

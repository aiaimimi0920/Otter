//! 系统 UIA 订阅真实数值通知；不在 UI owner 内等待回调。
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};
use windows::{
    core::{implement, Ref},
    Win32::{System::Variant::VARIANT, UI::Input::KeyboardAndMouse::VK_RIGHT},
};
type Evidence = std::result::Result<(i32, String, f64), windows::core::HRESULT>;
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
                f64::try_from(value)?,
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
/// 跨进程退出测试保留实际系统订阅，而不是只保留 RangeValue 接口。
pub(in crate::desktop::surface_window::uia) struct Observer {
    subscription: Subscription,
    receiver: Receiver<Evidence>,
    dropped: Arc<AtomicBool>,
}
impl Observer {
    pub(in crate::desktop::surface_window::uia) unsafe fn new(
        client: &IUIAutomation2,
        element: &IUIAutomationElement,
    ) -> Result<Self> {
        let (sender, receiver) = sync_channel(16);
        let dropped = Arc::new(AtomicBool::new(false));
        let handler: IUIAutomationPropertyChangedEventHandler = Handler {
            sender,
            dropped: dropped.clone(),
        }
        .into();
        let cache = client.CreateCacheRequest()?;
        cache.AddProperty(UIA_AutomationIdPropertyId)?;
        client.AddPropertyChangedEventHandlerNativeArray(
            element,
            TreeScope_Element,
            &cache,
            &handler,
            &[UIA_RangeValueValuePropertyId],
        )?;
        Ok(Self {
            subscription: Subscription {
                client: client.clone(),
                element: element.clone(),
                handler,
                active: true,
            },
            receiver,
            dropped,
        })
    }
    pub(in crate::desktop::surface_window::uia) fn expect_sequence(&self, expected: &[f64]) {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut index = 0;
        while index < expected.len() {
            let event = self
                .receiver
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("retained RangeValue subscription missed event")
                .unwrap();
            assert_eq!(event.0, UIA_RangeValueValuePropertyId.0);
            assert_eq!(event.1, "slider");
            if index > 0 && event.2 == expected[index - 1] {
                continue;
            }
            assert_eq!(event.2, expected[index]);
            index += 1;
        }
        // 默认 UIA 允许重复最后值，排空有界；之后新实例不得向此订阅发送。
        for _ in 0..16 {
            match self.receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(event) => assert_eq!(
                    event.unwrap(),
                    (
                        UIA_RangeValueValuePropertyId.0,
                        "slider".into(),
                        *expected.last().unwrap()
                    )
                ),
                Err(_) => return,
            }
        }
        panic!("unbounded RangeValue duplicate notifications");
    }
    pub(in crate::desktop::surface_window::uia) fn expect_quiet(&self) {
        assert!(
            self.receiver
                .recv_timeout(Duration::from_millis(250))
                .is_err(),
            "old process subscription received a new instance event"
        );
    }
    pub(in crate::desktop::surface_window::uia) fn finish(mut self) {
        self.subscription.remove().unwrap();
        let dropped = self.dropped.clone();
        let _client = self.subscription.client.clone();
        drop(self);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "retained RangeValue observer was not released"
        );
    }
}
fn expect(receiver: &Receiver<Evidence>, expected: f64) -> Result<()> {
    let item = receiver
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| {
            windows::core::Error::new(
                E_FAIL,
                format!("UIA RangeValue event timed out: expected {expected}"),
            )
        })?
        .map_err(windows::core::Error::from_hresult)?;
    assert_eq!(
        item,
        (UIA_RangeValueValuePropertyId.0, "slider".into(), expected)
    );
    // UIA 可重复交付；有界排空只接受相同身份及状态。
    for _ in 0..4 {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(value) => assert_eq!(value.unwrap(), item),
            Err(_) => break,
        }
    }
    Ok(())
}
pub(in crate::desktop::surface_window::uia) fn receive(hwnd: usize) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _apartment = Apartment;
    let (send, recv) = sync_channel(8);
    let dropped = Arc::new(AtomicBool::new(false));
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        client.SetAutoSetFocus(false)?;
        let hwnd = HWND(hwnd as *mut _);
        let window = client.ElementFromHandle(hwnd)?;
        let condition =
            client.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from("slider"))?;
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
            &[UIA_RangeValueValuePropertyId],
        )?;
        let mut subscription = Subscription {
            client,
            element,
            handler,
            active: true,
        };
        let pattern: IUIAutomationRangeValuePattern = subscription
            .element
            .GetCurrentPatternAs(UIA_RangeValuePatternId)?;
        pattern.SetValue(60.0)?;
        expect(&recv, 60.0)?;
        pattern.SetValue(60.0)?;
        assert!(pattern.SetValue(101.0).is_err());
        assert!(recv.recv_timeout(Duration::from_millis(200)).is_err());
        // 真实子 HWND 键盘路径必须共用通知出口，而非只覆盖 SetValue。
        subscription.element.SetFocus()?;
        assert!(
            subscription.element.CurrentHasKeyboardFocus()?.as_bool(),
            "Slider UIA SetFocus did not retain keyboard focus"
        );
        let child = FindWindowExW(Some(hwnd), None, windows::core::w!("STATIC"), None)?;
        SendMessageW(child, WM_KEYDOWN, Some(WPARAM(VK_RIGHT.0 as usize)), None);
        assert_eq!(
            pattern.CurrentValue()?,
            61.0,
            "Slider keyboard input was not accepted"
        );
        expect(&recv, 61.0)?;
        assert_eq!(pattern.CurrentValue()?, 61.0);
        subscription.remove()?;
        let _client = subscription.client.clone();
        drop(subscription);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "RangeValue subscription retained"
        );
        pattern.SetValue(70.0)?;
        assert!(recv.recv_timeout(Duration::from_millis(200)).is_err());
    }
    Ok(())
}
#[test]
fn uia_range_system_value_notifications_and_subscription_cleanup() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = super::super::range_tests::mount();
    query(window.hwnd.0 as usize, receive);
    assert!(window.take_error().is_none());
}

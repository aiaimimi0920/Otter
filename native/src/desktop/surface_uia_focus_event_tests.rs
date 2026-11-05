//! 全局焦点订阅只保留本测试唯一节点；MTA 注册/注销，缓存读取与有界回调。
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};
use windows::core::{implement, Ref};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_TAB;
const A: &str = "otter-focus-event-a";
const B: &str = "otter-focus-event-b";
#[implement(IUIAutomationFocusChangedEventHandler)]
struct Handler {
    sender: SyncSender<String>,
    dropped: Arc<AtomicBool>,
}
impl IUIAutomationFocusChangedEventHandler_Impl for Handler_Impl {
    fn HandleFocusChangedEvent(&self, sender: Ref<'_, IUIAutomationElement>) -> Result<()> {
        if let Some(sender) = sender.as_ref() {
            if let Ok(id) = unsafe { sender.CachedAutomationId() } {
                let id = id.to_string();
                if id == A || id == B {
                    let _ = self.sender.try_send(id);
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
    handler: IUIAutomationFocusChangedEventHandler,
    active: bool,
}
impl Subscription {
    fn remove(&mut self) -> Result<()> {
        if self.active {
            unsafe {
                self.client.RemoveFocusChangedEventHandler(&self.handler)?;
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
fn expect(receiver: &Receiver<String>, expected: &str) {
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("UIA focus event timed out"),
        expected
    );
    // 系统可能重复交付；下一次焦点动作前有界排空本次同身份通知。
    for _ in 0..8 {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(id) => assert_eq!(id, expected),
            Err(_) => break,
        }
    }
}
fn receive(hwnd: usize) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _apartment = Apartment;
    let (sender, receiver) = sync_channel(16);
    let dropped = Arc::new(AtomicBool::new(false));
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let hwnd = HWND(hwnd as *mut _);
        let window = client.ElementFromHandle(hwnd)?;
        let condition = client.CreatePropertyCondition(
            UIA_AutomationIdPropertyId,
            &windows::Win32::System::Variant::VARIANT::from(A),
        )?;
        let a = window.FindFirst(TreeScope_Descendants, &condition)?;
        let cache = client.CreateCacheRequest()?;
        cache.AddProperty(UIA_AutomationIdPropertyId)?;
        let handler: IUIAutomationFocusChangedEventHandler = Handler {
            sender,
            dropped: dropped.clone(),
        }
        .into();
        client.AddFocusChangedEventHandler(&cache, &handler)?;
        let mut subscription = Subscription {
            client,
            handler,
            active: true,
        };
        a.SetFocus()?;
        expect(&receiver, A);
        assert!(a.CurrentHasKeyboardFocus()?.as_bool());
        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_TAB.0 as usize)), None);
        expect(&receiver, B);
        // SW_SHOWNOACTIVATE 不抢用户前台；全局 GetFocusedElement 可属于另一程序。
        // 验证本 HWND 的节点实时属性，而不是把线程焦点误称桌面全局焦点。
        let condition = subscription.client.CreatePropertyCondition(
            UIA_AutomationIdPropertyId,
            &windows::Win32::System::Variant::VARIANT::from(B),
        )?;
        let b = window.FindFirst(TreeScope_Descendants, &condition)?;
        assert!(b.CurrentHasKeyboardFocus()?.as_bool());
        assert!(!a.CurrentHasKeyboardFocus()?.as_bool());
        SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(LPARAM(10 | (9 << 16))));
        SendMessageW(hwnd, WM_LBUTTONUP, None, Some(LPARAM(10 | (9 << 16))));
        expect(&receiver, A);
        assert!(a.CurrentHasKeyboardFocus()?.as_bool());
        subscription.remove()?;
        let _client = subscription.client.clone();
        drop(subscription);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "UIA focus handler retained after unsubscribe"
        );
        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_TAB.0 as usize)), None);
        assert!(receiver.recv_timeout(Duration::from_millis(250)).is_err());
    }
    Ok(())
}
#[test]
fn uia_focus_system_event_tracks_set_focus_and_tab_and_releases_subscription() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"otter-focus-event-a","type":"button","props":{"label":"A"}},{"id":"otter-focus-event-b","type":"button","props":{"label":"B"}}]}"#).unwrap();
    let window = super::super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
    window.set_input_policy(true, false).unwrap();
    window.show().unwrap();
    query(window.hwnd.0 as usize, receive);
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}

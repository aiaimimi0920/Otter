//! 三种选择路径共享真实 Invoked 事件；菜单关闭后仍能读取事件缓存并释放订阅。
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, SyncSender},
    Arc,
};
use windows::core::{implement, Ref, HRESULT};
pub(super) const NAME: &str = "otter-menu-invoked-fixture";
#[implement(IUIAutomationEventHandler)]
struct Handler {
    sender: SyncSender<std::result::Result<String, HRESULT>>,
    process: i32,
    dropped: Arc<AtomicBool>,
}
impl IUIAutomationEventHandler_Impl for Handler_Impl {
    fn HandleAutomationEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        event: UIA_EVENT_ID,
    ) -> Result<()> {
        if event == UIA_Invoke_InvokedEventId {
            let result = unsafe {
                sender
                    .as_ref()
                    .ok_or_else(|| Error::from_hresult(E_POINTER))
                    .and_then(|sender| {
                        if sender.CachedProcessId()? != self.process {
                            return Ok(None);
                        }
                        sender.CachedName().map(|name| Some(name.to_string()))
                    })
            };
            match result {
                Ok(Some(name)) => {
                    let _ = self.sender.try_send(Ok(name));
                }
                Err(error) => {
                    let _ = self.sender.try_send(Err(error.code()));
                }
                Ok(None) => {}
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
    root: IUIAutomationElement,
    handler: IUIAutomationEventHandler,
}
impl Drop for Subscription {
    fn drop(&mut self) {
        unsafe {
            let _ = self.client.RemoveAutomationEventHandler(
                UIA_Invoke_InvokedEventId,
                &self.root,
                &self.handler,
            );
        }
    }
}
pub(super) fn receive(hwnd: usize, mode: u8) -> Result<()> {
    receive_scoped(hwnd, mode, Scope::Menu)
}
pub(super) enum Scope {
    Menu,
    Source,
    Desktop,
}
pub(super) fn receive_scoped(hwnd: usize, mode: u8, scope: Scope) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _com = Apartment;
    let dropped = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = sync_channel(8);
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        // 菜单根会随选择销毁；精确源订阅或稳定祖先订阅避免依赖瞬态子树锚点。
        // 桌面范围先检查缓存 PID，绝不把其他进程的名称写入测试通道。
        let event_root = match scope {
            Scope::Desktop => client.GetRootElement()?,
            Scope::Source => client.RawViewWalker()?.GetFirstChildElement(&root)?,
            Scope::Menu => root.clone(),
        };
        let cache = client.CreateCacheRequest()?;
        cache.AddProperty(UIA_NamePropertyId)?;
        cache.AddProperty(UIA_ProcessIdPropertyId)?;
        let handler: IUIAutomationEventHandler = Handler {
            sender,
            process: root.CurrentProcessId()?,
            dropped: dropped.clone(),
        }
        .into();
        let subscription = Subscription {
            client,
            root: event_root,
            handler,
        };
        subscription.client.AddAutomationEventHandler(
            UIA_Invoke_InvokedEventId,
            &subscription.root,
            if matches!(scope, Scope::Source) {
                TreeScope_Element
            } else {
                TreeScope_Subtree
            },
            &cache,
            &subscription.handler,
        )?;
        let hwnd = HWND(hwnd as *mut _);
        match mode {
            0 => {
                let row = subscription
                    .client
                    .RawViewWalker()?
                    .GetFirstChildElement(&root)?;
                let invoke: IUIAutomationInvokePattern =
                    row.GetCurrentPatternAs(UIA_InvokePatternId)?;
                invoke.Invoke()?;
            }
            1 => {
                PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x28), LPARAM(0))?;
                PostMessageW(Some(hwnd), WM_KEYDOWN, WPARAM(0x0d), LPARAM(0))?;
            }
            _ => {
                PostMessageW(Some(hwnd), WM_LBUTTONDOWN, WPARAM(0), LPARAM(5 | (5 << 16)))?;
                PostMessageW(Some(hwnd), WM_LBUTTONUP, WPARAM(0), LPARAM(5 | (5 << 16)))?;
            }
        }
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("menu Invoked event timed out")
                .expect("menu Invoked event cache failed"),
            NAME
        );
        let extra = receiver.recv_timeout(Duration::from_millis(150));
        assert!(
            extra.is_err(),
            "duplicate menu invocation notification: {extra:?}"
        );
        drop(subscription);
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        dropped.load(Ordering::Acquire),
        "menu Invoked handler retained"
    );
    Ok(())
}
fn automation(hwnd: usize) -> Result<()> {
    receive(hwnd, 0)
}
fn keyboard(hwnd: usize) -> Result<()> {
    receive(hwnd, 1)
}
fn pointer(hwnd: usize) -> Result<()> {
    receive(hwnd, 2)
}
#[test]
fn system_invoked_event_covers_automation_keyboard_and_pointer_without_replaying_commands() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    for receive in [automation, keyboard, pointer] {
        let mut command = item(42);
        command.label = NAME.into();
        let mut menu = Cascade::open(owner.0, vec![command], POINT { x: 30, y: 30 }).unwrap();
        let hwnd = menu.pages[0].panel.handle().unwrap().0 as usize;
        query(hwnd, receive, || menu.poll().unwrap());
        menu.poll().unwrap();
        assert!(menu.closed());
        assert_eq!(menu.take_command(), Some(42));
        assert!(menu.take_command().is_none());
    }
}

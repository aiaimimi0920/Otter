//! MTA 订阅真实焦点、结构和属性事件；回调仅提取缓存并写入有界通道。
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};
use windows::{
    core::{implement, Interface, Ref},
    Win32::System::Variant::VARIANT,
};
const A: &str = "otter-menu-event-a";
const B: &str = "otter-menu-event-b";
#[derive(Debug, PartialEq)]
enum Event {
    Focus(String),
    Structure,
    Property(i32, f64),
}
#[implement(
    IUIAutomationFocusChangedEventHandler,
    IUIAutomationStructureChangedEventHandler,
    IUIAutomationPropertyChangedEventHandler
)]
struct Handler {
    sender: SyncSender<Event>,
    dropped: Arc<AtomicBool>,
}
impl IUIAutomationFocusChangedEventHandler_Impl for Handler_Impl {
    fn HandleFocusChangedEvent(&self, sender: Ref<'_, IUIAutomationElement>) -> Result<()> {
        if let Some(sender) = sender.as_ref() {
            if let Ok(name) = unsafe { sender.CachedName() } {
                let name = name.to_string();
                if name == A || name == B {
                    let _ = self.sender.try_send(Event::Focus(name));
                }
            }
        }
        Ok(())
    }
}
impl IUIAutomationStructureChangedEventHandler_Impl for Handler_Impl {
    fn HandleStructureChangedEvent(
        &self,
        _: Ref<'_, IUIAutomationElement>,
        change: StructureChangeType,
        _: *const SAFEARRAY,
    ) -> Result<()> {
        if change == StructureChangeType_ChildrenInvalidated {
            let _ = self.sender.try_send(Event::Structure);
        }
        Ok(())
    }
}
impl IUIAutomationPropertyChangedEventHandler_Impl for Handler_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        _: Ref<'_, IUIAutomationElement>,
        property: UIA_PROPERTY_ID,
        value: &VARIANT,
    ) -> Result<()> {
        if let Ok(value) = f64::try_from(value) {
            let _ = self.sender.try_send(Event::Property(property.0, value));
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
    focus: IUIAutomationFocusChangedEventHandler,
    structure: IUIAutomationStructureChangedEventHandler,
    property: IUIAutomationPropertyChangedEventHandler,
}
impl Drop for Subscription {
    fn drop(&mut self) {
        unsafe {
            let _ = self.client.RemoveFocusChangedEventHandler(&self.focus);
            let _ = self
                .client
                .RemoveStructureChangedEventHandler(&self.root, &self.structure);
            let _ = self
                .client
                .RemovePropertyChangedEventHandler(&self.root, &self.property);
        }
    }
}
fn expect(receiver: &Receiver<Event>, expected: Event) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let event = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("menu UIA notification timed out");
        if event == expected {
            return;
        }
    }
}
fn subscribe_and_operate(hwnd: usize) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _com = Apartment;
    let dropped = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = sync_channel(32);
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let cache = client.CreateCacheRequest()?;
        cache.AddProperty(UIA_NamePropertyId)?;
        let focus: IUIAutomationFocusChangedEventHandler = Handler {
            sender,
            dropped: dropped.clone(),
        }
        .into();
        let structure = focus.cast::<IUIAutomationStructureChangedEventHandler>()?;
        let property = focus.cast::<IUIAutomationPropertyChangedEventHandler>()?;
        // 先建立清理 owner，任何注册中途失败仍会尝试撤销已经成功的订阅。
        let subscription = Subscription {
            client,
            root,
            focus,
            structure,
            property,
        };
        subscription
            .client
            .AddFocusChangedEventHandler(&cache, &subscription.focus)?;
        subscription.client.AddStructureChangedEventHandler(
            &subscription.root,
            TreeScope_Element,
            &cache,
            &subscription.structure,
        )?;
        subscription
            .client
            .AddPropertyChangedEventHandlerNativeArray(
                &subscription.root,
                TreeScope_Subtree,
                &cache,
                &subscription.property,
                &[
                    UIA_ExpandCollapseExpandCollapseStatePropertyId,
                    UIA_ScrollVerticalScrollPercentPropertyId,
                ],
            )?;
        let walker = subscription.client.RawViewWalker()?;
        let a = walker.GetFirstChildElement(&subscription.root)?;
        let b = walker.GetNextSiblingElement(&a)?;
        a.SetFocus()?;
        expect(&receiver, Event::Focus(A.into()));
        b.SetFocus()?;
        expect(&receiver, Event::Focus(B.into()));
        let expand: IUIAutomationExpandCollapsePattern =
            a.GetCurrentPatternAs(UIA_ExpandCollapsePatternId)?;
        expand.Expand()?;
        expect(
            &receiver,
            Event::Property(
                UIA_ExpandCollapseExpandCollapseStatePropertyId.0,
                ExpandCollapseState_Expanded.0 as f64,
            ),
        );
        expand.Collapse()?;
        expect(
            &receiver,
            Event::Property(
                UIA_ExpandCollapseExpandCollapseStatePropertyId.0,
                ExpandCollapseState_Collapsed.0 as f64,
            ),
        );
        let scroll: IUIAutomationScrollPattern =
            subscription.root.GetCurrentPatternAs(UIA_ScrollPatternId)?;
        scroll.SetScrollPercent(UIA_ScrollPatternNoScroll, 100.0)?;
        let deadline = Instant::now() + Duration::from_secs(3);
        let (mut structure_seen, mut percent_seen) = (false, false);
        while !structure_seen || !percent_seen {
            match receiver
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("menu scroll notifications timed out")
            {
                Event::Structure => structure_seen = true,
                Event::Property(id, 100.0) if id == UIA_ScrollVerticalScrollPercentPropertyId.0 => {
                    percent_seen = true
                }
                _ => {}
            }
        }
        drop(subscription);
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        dropped.load(Ordering::Acquire),
        "menu UIA subscription retained handler"
    );
    Ok(())
}
#[test]
fn system_uia_events_report_committed_focus_expansion_and_scroll_then_release_handlers() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let owner = Owner::new();
    let mut items: Vec<_> = (0..40).map(item).collect();
    items[0].label = A.into();
    items[1].label = B.into();
    items[0].children = Some(vec![item(42)]);
    let mut menu = Cascade::open(owner.0, items, POINT { x: 30, y: 30 }).unwrap();
    let hwnd = menu.pages[0].panel.handle().unwrap().0 as usize;
    query(hwnd, subscribe_and_operate, || menu.poll().unwrap());
    assert_eq!(menu.pages[0].first, 40 - menu.pages[0].visible);
    assert!(menu.take_command().is_none());
}

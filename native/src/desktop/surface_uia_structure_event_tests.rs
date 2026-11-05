//! MTA 订阅稳定根的结构通知；UI owner 替换场景后客户端重读新子树。
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{channel, sync_channel, Sender, SyncSender},
    Arc,
};
use windows::core::{implement, Ref};
#[implement(IUIAutomationStructureChangedEventHandler)]
struct Handler {
    sender: SyncSender<(i32, String)>,
    dropped: Arc<AtomicBool>,
}
impl IUIAutomationStructureChangedEventHandler_Impl for Handler_Impl {
    fn HandleStructureChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
        change: StructureChangeType,
        _runtime: *const SAFEARRAY,
    ) -> Result<()> {
        if let Some(sender) = sender.as_ref() {
            if let Ok(name) = unsafe { sender.CachedName() } {
                let _ = self.sender.try_send((change.0, name.to_string()));
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
    element: IUIAutomationElement,
    handler: IUIAutomationStructureChangedEventHandler,
    active: bool,
}
impl Subscription {
    fn remove(&mut self) -> Result<()> {
        if self.active {
            unsafe {
                self.client
                    .RemoveStructureChangedEventHandler(&self.element, &self.handler)?;
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
enum Progress {
    Ready,
    Done(std::result::Result<(), windows::core::HRESULT>),
}
fn receive(hwnd: usize, progress: &Sender<Progress>, late_mount: bool) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _apartment = Apartment;
    let (sender, receiver) = sync_channel(8);
    let dropped = Arc::new(AtomicBool::new(false));
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let element = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let cache = client.CreateCacheRequest()?;
        cache.AddProperty(UIA_NamePropertyId)?;
        let handler: IUIAutomationStructureChangedEventHandler = Handler {
            sender,
            dropped: dropped.clone(),
        }
        .into();
        client.AddStructureChangedEventHandler(&element, TreeScope_Element, &cache, &handler)?;
        let mut subscription = Subscription {
            client,
            element,
            handler,
            active: true,
        };
        progress.send(Progress::Ready).unwrap();
        let expected = (
            StructureChangeType_ChildrenInvalidated.0,
            "Otter Surface".into(),
        );
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("UIA structure event timed out"),
            expected
        );
        let condition = subscription.client.CreatePropertyCondition(
            UIA_AutomationIdPropertyId,
            &windows::Win32::System::Variant::VARIANT::from("new"),
        )?;
        let node = subscription
            .element
            .FindFirst(TreeScope_Descendants, &condition)?;
        assert_eq!(node.CurrentName()?.to_string(), "New tree");
        if late_mount {
            let value: IUIAutomationValuePattern = node.GetCurrentPatternAs(UIA_ValuePatternId)?;
            assert_eq!(value.CurrentValue()?.to_string(), "late");
            assert_eq!(
                subscription
                    .element
                    .FindAll(TreeScope_Descendants, &condition)?
                    .Length()?,
                1
            );
        }
        // Windows 可重复交付同一次 raise；每个回调都必须属于正确根和类型。
        for _ in 0..8 {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => assert_eq!(event, expected),
                Err(_) => break,
            }
        }
        subscription.remove()?;
        let _client = subscription.client.clone();
        drop(subscription);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !dropped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            dropped.load(Ordering::Acquire),
            "UIA structure handler retained after unsubscribe"
        );
    }
    Ok(())
}
#[test]
fn uia_structure_system_event_invalidates_old_tree_and_client_reads_replacement() {
    run(false);
}
#[test]
fn uia_structure_system_event_reports_late_edit_mount_without_scene_replacement() {
    run(true);
}
fn binding() -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn run(late_mount: bool) {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(
        if late_mount { br#"{"id":"new","type":"input","props":{"value":"late"},"accessibility":{"label":"New tree"}}"# }
        else { br#"{"id":"old","type":"text","props":{"text":"Old tree"}}"# },
    )
    .unwrap();
    let window = super::super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
    let hwnd = window.hwnd.0 as usize;
    let (sender, receiver) = channel();
    let worker = std::thread::spawn(move || {
        let result = receive(hwnd, &sender, late_mount).map_err(|error| error.code());
        let _ = sender.send(Progress::Done(result));
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut replaced = false;
    loop {
        match receiver.try_recv() {
            Ok(Progress::Ready) => {
                assert!(!replaced);
                if late_mount {
                    window.bind_edits(&binding()).unwrap();
                } else {
                    window
                        .replace(
                            crate::surface_scene::Scene::parse(
                                br#"{"id":"new","type":"text","props":{"text":"New tree"}}"#,
                            )
                            .unwrap(),
                        )
                        .unwrap();
                }
                replaced = true;
            }
            Ok(Progress::Done(result)) => {
                result.unwrap();
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                panic!("UIA structure client disconnected")
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        assert!(Instant::now() < deadline, "UIA structure client timed out");
        unsafe {
            let mut message = MSG::default();
            for _ in 0..64 {
                if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    worker.join().unwrap();
    assert!(replaced);
    assert!(window.take_error().is_none());
}

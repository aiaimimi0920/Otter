//! 真正系统 EDIT 与声明式容器合并：唯一节点、逻辑父级及原生编辑模式。
use super::*;
use windows::Win32::System::Variant::VARIANT;
fn receive(hwnd: usize) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _apartment = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        let root = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let walker = client.ControlViewWalker()?;
        for id in ["input", "textarea", "number"] {
            let condition =
                client.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(id))?;
            let nodes = root.FindAll(TreeScope_Descendants, &condition)?;
            assert_eq!(nodes.Length()?, 1, "duplicate or absent EDIT {id}");
            let edit = nodes.GetElement(0)?;
            assert_eq!(
                edit.CurrentControlType()?,
                if id == "textarea" {
                    UIA_DocumentControlTypeId
                } else {
                    UIA_EditControlTypeId
                },
                "EDIT {id}: hwnd={:?}, class={}, value={}, text={}",
                edit.CurrentNativeWindowHandle()?,
                edit.CurrentClassName()?,
                edit.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                    .is_ok(),
                edit.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
                    .is_ok()
            );
            assert_eq!(
                walker
                    .GetParentElement(&edit)?
                    .CurrentAutomationId()?
                    .to_string(),
                "group"
            );
            let value: IUIAutomationValuePattern = edit.GetCurrentPatternAs(UIA_ValuePatternId)?;
            assert!(!value.CurrentIsReadOnly()?.as_bool());
            if id == "input" {
                edit.SetFocus()?;
                assert!(edit.CurrentHasKeyboardFocus()?.as_bool());
                assert_eq!(value.CurrentValue()?.to_string(), "hello");
                let text: IUIAutomationTextPattern = edit.GetCurrentPatternAs(UIA_TextPatternId)?;
                assert_eq!(text.DocumentRange()?.GetText(-1)?.to_string(), "hello");
                value.SetValue(&windows::core::BSTR::from("changed"))?;
                assert_eq!(value.CurrentValue()?.to_string(), "changed");
            }
        }
        let edit_condition =
            client.CreatePropertyCondition(UIA_ClassNamePropertyId, &VARIANT::from("Edit"))?;
        assert_eq!(
            root.FindAll(TreeScope_Descendants, &edit_condition)?
                .Length()?,
            3,
            "native EDIT duplicated outside the declarative group"
        );
    }
    Ok(())
}
#[test]
fn uia_edit_system_relocation_preserves_native_patterns_and_unique_parentage() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"group","type":"column","children":[{"id":"input","type":"input","props":{"value":"hello"},"events":{"change":"change"}},{"id":"textarea","type":"textarea","props":{"value":"lines"}},{"id":"number","type":"number","props":{"value":2}}]}]}"#).unwrap();
    let window = super::super::super::SurfaceWindow::new(scene, 400, 400).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, false).unwrap();
    window.show().unwrap();
    let mut previous = None;
    let mut traces = Vec::new();
    for _ in 0..3 {
        let edit = unsafe {
            FindWindowExW(Some(window.hwnd), previous, windows::core::w!("EDIT"), None).unwrap()
        };
        traces.push(InputTrace::new(edit));
        previous = Some(edit);
    }
    // 系统 UIA 全局焦点需要真正前台；复用自有授权夹具，不把线程 GetFocus 当全局焦点。
    let _foreground = unsafe { crate::surface_javascript::fixture::focus_foreground(window.hwnd) };
    query(window.hwnd.0 as usize, receive);
    assert!(window.take_error().is_none());
}

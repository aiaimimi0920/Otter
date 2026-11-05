//! MTA 客户端通过系统 TogglePattern 读取并切换真实 Switch。
use super::*;
fn change(hwnd: usize) -> Result<(i32, i32, i32)> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
    }
    let _apartment = Apartment;
    unsafe {
        let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
        client.SetConnectionTimeout(3000)?;
        client.SetTransactionTimeout(3000)?;
        client.SetAutoSetFocus(false)?;
        let window = client.ElementFromHandle(HWND(hwnd as *mut _))?;
        let condition = client.CreatePropertyCondition(
            UIA_AutomationIdPropertyId,
            &windows::Win32::System::Variant::VARIANT::from("switch"),
        )?;
        let element = window.FindFirst(TreeScope_Descendants, &condition)?;
        let pattern: IUIAutomationTogglePattern =
            element.GetCurrentPatternAs(UIA_TogglePatternId)?;
        let first = pattern.CurrentToggleState()?.0;
        pattern.Toggle()?;
        let second = pattern.CurrentToggleState()?.0;
        pattern.Toggle()?;
        Ok((first, second, pattern.CurrentToggleState()?.0))
    }
}
#[test]
fn uia_toggle_system_client_reads_and_changes_live_value() {
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
    assert_eq!(
        query(window.hwnd.0 as usize, change),
        (ToggleState_Off.0, ToggleState_On.0, ToggleState_Off.0)
    );
    assert!(window.has_pending_edit());
    assert!(window.take_click().is_none());
    assert!(window.state.keyboard.borrow().focused().is_none());
    assert!(window.take_error().is_none());
}

//! 真正跨进程 UIA：父 MTA 客户端读取独立 STA 子进程，不访问其内部 State。
use super::*;
use std::time::{Duration, Instant};
use windows::Win32::{System::Variant::VARIANT, UI::WindowsAndMessaging::*};
#[path = "surface_uia_process_events.rs"]
mod events;
#[path = "surface_uia_process_fixture.rs"]
pub(super) mod fixture;
#[path = "surface_uia_process_focus.rs"]
mod focus;
#[path = "surface_uia_process_range.rs"]
mod range;
#[path = "surface_uia_process_structure.rs"]
mod structure;
struct Apartment;
#[test]
#[ignore = "explicit cross-process native Select check"]
fn uia_select_external_process_selection() {
    let mut fixture = fixture::Fixture::start_select();
    super::client_tests::native_select::receive(fixture.hwnd.0 as usize).unwrap();
    fixture.accepted();
    fixture.close();
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
unsafe fn find(
    client: &IUIAutomation2,
    root: &IUIAutomationElement,
    id: &str,
) -> Result<IUIAutomationElement> {
    let condition =
        client.CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(id))?;
    let nodes = root.FindAll(TreeScope_Descendants, &condition)?;
    assert_eq!(
        nodes.Length()?,
        1,
        "missing/duplicate cross-process node {id}"
    );
    nodes.GetElement(0)
}
#[test]
#[ignore = "explicit cross-process desktop check; creates only its own bounded fixture process"]
fn uia_external_process_tree_edit_toggle_invoke_and_exit_revoke() {
    run(false, false);
}
#[test]
#[ignore = "explicit cross-process desktop check; terminates only its own fixture process"]
fn uia_external_process_forced_exit_and_restart_revoke() {
    run(true, false);
}
#[test]
#[ignore = "explicit cross-process desktop check; retains subscriptions across owned child exit"]
fn uia_external_process_live_subscription_exit_and_restart_isolation() {
    run(true, true);
}
fn run(force_exit: bool, retain_subscription: bool) {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let mut fixture = fixture::Fixture::start();
    unsafe {
        let client: IUIAutomation2 =
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).unwrap();
        client.SetConnectionTimeout(3000).unwrap();
        client.SetTransactionTimeout(3000).unwrap();
        client.SetAutoSetFocus(false).unwrap();
        let root = client.ElementFromHandle(fixture.hwnd).unwrap();
        assert_eq!(root.CurrentProcessId().unwrap() as u32, fixture.pid());
        assert_ne!(fixture.pid(), std::process::id());
        assert_eq!(
            root.CurrentFrameworkId().unwrap().to_string(),
            "Otter.Native"
        );
        let input = find(&client, &root, "input").unwrap();
        assert_eq!(
            client
                .ControlViewWalker()
                .unwrap()
                .GetParentElement(&input)
                .unwrap()
                .CurrentAutomationId()
                .unwrap()
                .to_string(),
            "group"
        );
        let value: IUIAutomationValuePattern =
            input.GetCurrentPatternAs(UIA_ValuePatternId).unwrap();
        assert_eq!(value.CurrentValue().unwrap().to_string(), "original");
        value
            .SetValue(&windows::core::BSTR::from("external"))
            .unwrap();
        assert_eq!(value.CurrentValue().unwrap().to_string(), "external");
        let switch = find(&client, &root, "switch").unwrap();
        let button = find(&client, &root, "button").unwrap();
        let subscription = events::Subscription::new(&client, &switch, &button).unwrap();
        let toggle: IUIAutomationTogglePattern =
            switch.GetCurrentPatternAs(UIA_TogglePatternId).unwrap();
        assert_eq!(toggle.CurrentToggleState().unwrap(), ToggleState_Off);
        toggle.Toggle().unwrap();
        subscription.expect(
            UIA_ToggleToggleStatePropertyId.0,
            "switch",
            ToggleState_On.0,
        );
        assert_eq!(toggle.CurrentToggleState().unwrap(), ToggleState_On);
        focus::verify(&client, &switch, &button, fixture.pid());
        assert!(button.CurrentHasKeyboardFocus().unwrap().as_bool());
        let invoke: IUIAutomationInvokePattern =
            button.GetCurrentPatternAs(UIA_InvokePatternId).unwrap();
        invoke.Invoke().unwrap();
        subscription.expect(UIA_Invoke_InvokedEventId.0, "button", 0);
        fixture.accepted();
        let old_subscription = if retain_subscription {
            Some(subscription)
        } else {
            subscription.finish();
            None
        };
        if force_exit {
            fixture.terminate();
        } else {
            structure::verify(&client, &root, &mut fixture);
            assert!(toggle.Toggle().is_err());
            assert!(invoke.Invoke().is_err());
            fixture.close();
        }
        assert!(
            toggle.CurrentToggleState().is_err(),
            "closed process retained a usable Toggle provider"
        );
        assert!(
            invoke.Invoke().is_err(),
            "closed process retained an actionable Invoke provider"
        );
        assert!(
            value.CurrentValue().is_err(),
            "closed process retained a live EDIT provider"
        );
        if force_exit {
            // 保留旧引用并重启相同场景，证明新实例不使旧 provider 恢复可用。
            let mut replacement = fixture::Fixture::start();
            let fresh_root = client.ElementFromHandle(replacement.hwnd).unwrap();
            assert_eq!(
                fresh_root.CurrentProcessId().unwrap() as u32,
                replacement.pid()
            );
            let fresh_input = find(&client, &fresh_root, "input").unwrap();
            let fresh_value: IUIAutomationValuePattern =
                fresh_input.GetCurrentPatternAs(UIA_ValuePatternId).unwrap();
            assert_eq!(fresh_value.CurrentValue().unwrap().to_string(), "original");
            fresh_value
                .SetValue(&windows::core::BSTR::from("external"))
                .unwrap();
            let fresh_switch = find(&client, &fresh_root, "switch").unwrap();
            let fresh_button = find(&client, &fresh_root, "button").unwrap();
            let fresh_subscription =
                events::Subscription::new(&client, &fresh_switch, &fresh_button).unwrap();
            let fresh_toggle: IUIAutomationTogglePattern = fresh_switch
                .GetCurrentPatternAs(UIA_TogglePatternId)
                .unwrap();
            assert_eq!(fresh_toggle.CurrentToggleState().unwrap(), ToggleState_Off);
            fresh_toggle.Toggle().unwrap();
            fresh_subscription.expect(
                UIA_ToggleToggleStatePropertyId.0,
                "switch",
                ToggleState_On.0,
            );
            let fresh_invoke: IUIAutomationInvokePattern = fresh_button
                .GetCurrentPatternAs(UIA_InvokePatternId)
                .unwrap();
            fresh_invoke.Invoke().unwrap();
            fresh_subscription.expect(UIA_Invoke_InvokedEventId.0, "button", 0);
            replacement.accepted();
            assert!(toggle.CurrentToggleState().is_err());
            assert!(toggle.Toggle().is_err());
            assert!(invoke.Invoke().is_err());
            assert!(value.CurrentValue().is_err());
            assert!(value.SetValue(&windows::core::BSTR::from("stale")).is_err());
            assert_eq!(fresh_toggle.CurrentToggleState().unwrap(), ToggleState_On);
            assert_eq!(fresh_value.CurrentValue().unwrap().to_string(), "external");
            if let Some(subscription) = old_subscription {
                // 同名节点的新实例通知不能流入仍持有的旧进程订阅。
                subscription.expect_quiet();
                subscription.finish();
            }
            fresh_subscription.finish();
            replacement.close();
        }
    }
}

//! Slider 跨进程数值、通知、键盘事件和旧引用撤销；只操作自启 Fixture。
use super::*;
unsafe fn connect(fixture: &fixture::Fixture) -> Result<IUIAutomationRangeValuePattern> {
    let client: IUIAutomation2 = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
    client.SetConnectionTimeout(3000)?;
    client.SetTransactionTimeout(3000)?;
    let root = client.ElementFromHandle(fixture.hwnd)?;
    assert_eq!(root.CurrentProcessId()? as u32, fixture.pid());
    assert_ne!(fixture.pid(), std::process::id());
    let element = find(&client, &root, "slider")?;
    assert_eq!(element.CurrentControlType()?, UIA_SliderControlTypeId);
    element.GetCurrentPatternAs(UIA_RangeValuePatternId)
}
fn exercise(fixture: &fixture::Fixture) {
    super::super::client_tests::range_events::receive(fixture.hwnd.0 as usize).unwrap();
    // 此信号仅在子进程实际校验并结算全部 input/change 后发出。
    fixture.accepted();
}
unsafe fn observe(
    fixture: &fixture::Fixture,
) -> super::super::client_tests::range_events::Observer {
    let client: IUIAutomation2 =
        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).unwrap();
    client.SetConnectionTimeout(3000).unwrap();
    client.SetTransactionTimeout(3000).unwrap();
    let root = client.ElementFromHandle(fixture.hwnd).unwrap();
    let element = find(&client, &root, "slider").unwrap();
    super::super::client_tests::range_events::Observer::new(&client, &element).unwrap()
}
fn run(force: bool, retain_subscription: bool) {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
    }
    let _com = Apartment;
    let mut fixture = fixture::Fixture::start_range();
    unsafe {
        let old = connect(&fixture).unwrap();
        assert_eq!(old.CurrentValue().unwrap(), 50.0);
        assert_eq!(old.CurrentMinimum().unwrap(), 0.0);
        assert_eq!(old.CurrentMaximum().unwrap(), 100.0);
        let observer = retain_subscription.then(|| observe(&fixture));
        exercise(&fixture);
        if let Some(observer) = &observer {
            observer.expect_sequence(&[60.0, 61.0, 70.0]);
        }
        assert_eq!(old.CurrentValue().unwrap(), 70.0);
        if force {
            fixture.terminate();
        } else {
            fixture.close();
        }
        assert!(
            old.CurrentValue().is_err(),
            "dead Slider retained readable provider"
        );
        assert!(
            old.SetValue(80.0).is_err(),
            "dead Slider retained writable provider"
        );
        let mut replacement = fixture::Fixture::start_range();
        let fresh = connect(&replacement).unwrap();
        assert_eq!(fresh.CurrentValue().unwrap(), 50.0);
        assert!(old.SetValue(90.0).is_err());
        assert_eq!(
            fresh.CurrentValue().unwrap(),
            50.0,
            "stale provider mutated new process"
        );
        exercise(&replacement);
        if let Some(observer) = observer {
            observer.expect_quiet();
            observer.finish();
        }
        assert_eq!(fresh.CurrentValue().unwrap(), 70.0);
        replacement.close();
        assert!(fresh.CurrentValue().is_err());
    }
}
#[test]
#[ignore = "explicit cross-process Slider read/write/notifications/exit check"]
fn uia_range_external_process_notifications_edges_and_graceful_exit() {
    run(false, false);
}
#[test]
#[ignore = "explicit owned Slider child termination and restart isolation check"]
fn uia_range_external_process_forced_exit_and_restart() {
    run(true, false);
}
#[test]
#[ignore = "explicit retained Slider subscription across graceful exit and restart"]
fn uia_range_external_process_live_subscription_graceful_exit() {
    run(false, true);
}
#[test]
#[ignore = "explicit retained Slider subscription across owned child termination"]
fn uia_range_external_process_live_subscription_forced_exit() {
    run(true, true);
}

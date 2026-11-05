//! 系统 MTA RangeValue 客户端验证 HWND override 合并后的树和实时读写。
use super::*;
fn change(hwnd: usize) -> Result<(i32, i32, f64, f64)> {
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
            &windows::Win32::System::Variant::VARIANT::from("slider"),
        )?;
        let all = window.FindAll(TreeScope_Descendants, &condition)?;
        let element = all.GetElement(0)?;
        let pattern: IUIAutomationRangeValuePattern =
            element.GetCurrentPatternAs(UIA_RangeValuePatternId)?;
        let first = pattern.CurrentValue()?;
        pattern.SetValue(72.0)?;
        Ok((
            all.Length()?,
            element.CurrentControlType()?.0,
            first,
            pattern.CurrentValue()?,
        ))
    }
}
#[test]
fn uia_range_system_client_single_slider_reads_and_sets() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = super::super::range_tests::mount();
    assert_eq!(
        query(window.hwnd.0 as usize, change),
        (1, UIA_SliderControlTypeId.0, 50.0, 72.0)
    );
    assert!(window.has_pending_edit());
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}

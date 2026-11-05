//! 精确UIA属性/Pattern读取；不执行Invoke/写值，不截断后冒充exact，不读取密码。
use super::*;
use ring::digest::{digest, SHA256};
use serde_json::{json, Value};
fn text(value: BSTR, limit: usize) -> Result<Option<String>> {
    if value.len() > limit * 2 {
        return Err(Error::from(E_BOUNDS));
    }
    let value = value.to_string();
    if value.chars().count() > limit {
        return Err(Error::from(E_BOUNDS));
    }
    Ok((!value.is_empty()).then_some(value))
}
pub(super) unsafe fn label(e: &IUIAutomationElement) -> Result<String> {
    let id = text(e.CurrentAutomationId()?, 400)?;
    Ok(format!(
        "{}:{}",
        e.CurrentControlType()?.0,
        id.as_deref().unwrap_or("")
    ))
}
unsafe fn available(e: &IUIAutomationElement, id: UIA_PROPERTY_ID) -> Result<bool> {
    bool::try_from(&e.GetCurrentPropertyValue(id)?)
}
pub(super) unsafe fn read(
    e: &IUIAutomationElement,
    ancestors: Vec<String>,
    root: RECT,
    is_root: bool,
) -> Result<Option<Sample>> {
    let kind = e.CurrentControlType()?;
    let name = match kind {
        v if v == UIA_ProgressBarControlTypeId => "ProgressBar",
        v if v == UIA_ButtonControlTypeId => "Button",
        v if v == UIA_TextControlTypeId => "Text",
        v if v == UIA_CheckBoxControlTypeId => "CheckBox",
        v if v == UIA_SliderControlTypeId => "Slider",
        v if v == UIA_EditControlTypeId => "Edit",
        v if v == UIA_WindowControlTypeId => "Window",
        _ if is_root => "Window",
        _ => return Ok(None),
    };
    let locator = Locator {
        automation_id: text(e.CurrentAutomationId()?, 512)?,
        name: text(e.CurrentName()?, 512)?,
        control_type: name.into(),
        ancestor_path: ancestors,
        runtime_id: None,
    };
    if !is_root && locator.automation_id.is_none() && locator.name.is_none() {
        return Ok(None);
    }
    let stable = json!({"automationId":locator.automation_id,"name":if locator.automation_id.is_none(){locator.name.clone()}else{None},"controlType":locator.control_type,"ancestorPath":locator.ancestor_path});
    let hash = digest(
        &SHA256,
        &serde_json::to_vec(&stable).map_err(|_| Error::from(E_FAIL))?,
    );
    let id = if is_root {
        "uia:source".into()
    } else {
        format!(
            "uia:{}",
            hash.as_ref()[..12]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
    };
    let value = values(e, &locator, root).map_err(|_| "uia_property_unreadable".to_owned());
    Ok(Some(Sample { id, locator, value }))
}
unsafe fn values(e: &IUIAutomationElement, locator: &Locator, root: RECT) -> Result<Value> {
    let rect = e.CurrentBoundingRectangle()?;
    let width = f64::from(root.right - root.left);
    let height = f64::from(root.bottom - root.top);
    if width <= 0. || height <= 0. {
        return Err(Error::from(E_INVALIDARG));
    }
    let mut value = json!({"enabled":e.CurrentIsEnabled()?.as_bool(),"offscreen":e.CurrentIsOffscreen()?.as_bool(),
        "focused":e.CurrentHasKeyboardFocus()?.as_bool(),"anchor":{"coordinateSpace":"source_window",
        "screenBounds":{"x":rect.left,"y":rect.top,"width":rect.right-rect.left,"height":rect.bottom-rect.top},
        "normalizedBounds":{"x":((f64::from(rect.left)-f64::from(root.left))/width).clamp(0.,1.),
        "y":((f64::from(rect.top)-f64::from(root.top))/height).clamp(0.,1.),
        "width":(f64::from(rect.right-rect.left)/width).clamp(0.,1.),"height":(f64::from(rect.bottom-rect.top)/height).clamp(0.,1.)}}});
    let mut patterns = Vec::new();
    if available(e, UIA_IsInvokePatternAvailablePropertyId)? {
        patterns.push("invoke");
    }
    if available(e, UIA_IsRangeValuePatternAvailablePropertyId)? {
        let p: IUIAutomationRangeValuePattern = e.GetCurrentPatternAs(UIA_RangeValuePatternId)?;
        let (current, minimum, maximum) =
            (p.CurrentValue()?, p.CurrentMinimum()?, p.CurrentMaximum()?);
        if [current, minimum, maximum].iter().any(|n| !n.is_finite()) {
            return Err(Error::from(E_INVALIDARG));
        }
        value["rangeValue"] = json!({"value":current,"minimum":minimum,"maximum":maximum,"readOnly":p.CurrentIsReadOnly()?.as_bool()});
        patterns.push("range_value");
    }
    if available(e, UIA_IsValuePatternAvailablePropertyId)? {
        let p: IUIAutomationValuePattern = e.GetCurrentPatternAs(UIA_ValuePatternId)?;
        value["value"] = json!({"text":text(p.CurrentValue()?,4096)?.unwrap_or_default(),"readOnly":p.CurrentIsReadOnly()?.as_bool()});
        patterns.push("value");
    }
    if available(e, UIA_IsTogglePatternAvailablePropertyId)? {
        let p: IUIAutomationTogglePattern = e.GetCurrentPatternAs(UIA_TogglePatternId)?;
        value["toggleState"] = json!(match p.CurrentToggleState()? {
            v if v == ToggleState_Off => "off",
            v if v == ToggleState_On => "on",
            v if v == ToggleState_Indeterminate => "indeterminate",
            _ => return Err(Error::from(E_INVALIDARG)),
        });
        patterns.push("toggle");
    }
    if available(e, UIA_IsScrollPatternAvailablePropertyId)? {
        let p: IUIAutomationScrollPattern = e.GetCurrentPatternAs(UIA_ScrollPatternId)?;
        let (horizontal, vertical) = (
            p.CurrentHorizontalScrollPercent()?,
            p.CurrentVerticalScrollPercent()?,
        );
        if !horizontal.is_finite() || !vertical.is_finite() {
            return Err(Error::from(E_INVALIDARG));
        }
        value["scroll"] = json!({"horizontalPercent":horizontal,"verticalPercent":vertical});
        patterns.push("scroll");
    }
    if locator.control_type == "Text" {
        value["text"] = json!(locator.name);
    }
    if locator.control_type == "Window" {
        value["title"] = json!(locator.name);
    }
    value["patterns"] = json!(patterns);
    Ok(value)
}

//! Read-only marker-state presentation; refresh revokes old nodes without inventing selection groups.
use super::*;

#[test]
fn status_reflects_known_marks_without_inventing_patterns_or_actions() {
    let fixture = Fixture::new();
    let root = create(fixture.hwnd, fixture.owner.view(), 28).unwrap();
    let fragment: IRawElementProviderFragment = root.cast().unwrap();
    unsafe {
        assert_eq!(
            root.GetPropertyValue(UIA_ItemStatusPropertyId)
                .unwrap()
                .to_string(),
            ""
        );
        let mut previous: Option<IRawElementProviderSimple> = None;
        for (checked, radio, separator, enabled, expected) in [
            (false, false, false, true, ""),
            (
                true,
                false,
                false,
                true,
                crate::ui_text::text("native-menu-checked"),
            ),
            (
                false,
                true,
                false,
                true,
                crate::ui_text::text("native-menu-unselected"),
            ),
            (
                true,
                true,
                false,
                true,
                crate::ui_text::text("native-menu-selected"),
            ),
            (
                true,
                true,
                false,
                false,
                crate::ui_text::text("native-menu-selected"),
            ),
            (true, true, true, true, ""),
        ] {
            let mut item = row(0);
            item.checked = checked;
            item.radio = radio;
            item.separator = separator;
            item.enabled = enabled;
            fixture.owner.replace(&[item]);
            if let Some(old) = previous.take() {
                assert_eq!(
                    old.GetPropertyValue(UIA_ItemStatusPropertyId)
                        .unwrap_err()
                        .code()
                        .0,
                    UIA_E_ELEMENTNOTAVAILABLE as i32
                );
            }
            let node: IRawElementProviderSimple = fragment
                .Navigate(NavigateDirection_FirstChild)
                .unwrap()
                .cast()
                .unwrap();
            assert_eq!(
                node.GetPropertyValue(UIA_ItemStatusPropertyId)
                    .unwrap()
                    .to_string(),
                expected
            );
            for id in [UIA_TogglePatternId, UIA_SelectionItemPatternId] {
                let mut unsupported = ptr::dangling_mut();
                assert_eq!(
                    (node.vtable().GetPatternProvider)(node.as_raw(), id, &mut unsupported),
                    S_OK
                );
                assert!(unsupported.is_null());
            }
            assert!(fixture.owner.take().is_none());
            previous = Some(node);
        }
    }
}

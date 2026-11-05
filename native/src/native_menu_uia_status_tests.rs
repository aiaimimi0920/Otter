//! 标记状态只读呈现；刷新撤销旧节点，不推断不存在的选择分组。
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
            (true, false, false, true, "已勾选"),
            (false, true, false, true, "未选中"),
            (true, true, false, true, "已选中"),
            (true, true, false, false, "已选中"),
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

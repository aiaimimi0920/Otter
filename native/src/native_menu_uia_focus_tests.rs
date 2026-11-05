//! FragmentRoot 汇总实际子页焦点，祖先高亮不能成为重复焦点。
use super::*;
#[test]
fn fragment_root_returns_active_descendant_then_restored_parent_without_new_identity() {
    let parent = Fixture::new();
    let child = Fixture::new();
    let mut parent_row = row(0);
    parent_row.focused = true;
    parent_row.submenu = true;
    let mut child_row = row(0);
    child_row.label = "child".into();
    child_row.focused = true;
    parent.owner.replace(&[parent_row]);
    child.owner.replace(&[child_row]);
    child.owner.set_location(child.hwnd.0 as usize, 28);
    parent.owner.link_child(&child.owner, 0);
    let parent_node = parent.owner.nodes().remove(0);
    let id = parent_node.runtime_id();
    parent.owner.set_active(false);
    let root = create(parent.hwnd, parent.owner.view(), 28).unwrap();
    let fragment: IRawElementProviderFragmentRoot = root.cast().unwrap();
    unsafe {
        assert!(!parent_node.read().unwrap().focused);
        let focused: IRawElementProviderSimple = fragment.GetFocus().unwrap().cast().unwrap();
        assert_eq!(
            focused
                .GetPropertyValue(UIA_NamePropertyId)
                .unwrap()
                .to_string(),
            "child"
        );
        child.owner.set_active(false);
        parent.owner.set_active(true);
        let restored: IRawElementProviderSimple = fragment.GetFocus().unwrap().cast().unwrap();
        assert_eq!(
            restored
                .GetPropertyValue(UIA_NamePropertyId)
                .unwrap()
                .to_string(),
            "命令 0"
        );
        assert_eq!(parent_node.runtime_id(), id);
        assert!(parent_node.read().unwrap().focused);
        child.owner.revoke();
        assert!(focused.GetPropertyValue(UIA_NamePropertyId).is_err());
        assert!(fragment.GetFocus().is_ok());
    }
}

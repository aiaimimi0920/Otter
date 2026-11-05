//! 从已提交快照生成有界通知；发出事件时不持模型锁或 RefCell 借用。
use super::*;
use crate::native_menu::accessibility::Scroll;
pub(crate) struct Snapshot {
    ids: Vec<[i32; 5]>,
    focused: Option<[i32; 5]>,
    expanded: Vec<bool>,
    scroll: Scroll,
}
pub(crate) enum Change {
    Structure,
    Focus(Node),
    Expanded(Node, bool, bool),
    Percent(f64, f64),
    Size(f64, f64),
}
fn percent(scroll: Scroll) -> f64 {
    if scroll.total <= scroll.visible {
        UIA_ScrollPatternNoScroll
    } else {
        100.0 * scroll.first as f64 / (scroll.total - scroll.visible) as f64
    }
}
fn size(scroll: Scroll) -> f64 {
    if scroll.total == 0 {
        100.0
    } else {
        (100.0 * scroll.visible as f64 / scroll.total as f64).min(100.0)
    }
}
pub(crate) fn changes(view: &View, previous: &mut Option<Snapshot>) -> Vec<Change> {
    let nodes = view.nodes();
    let mut current = Snapshot {
        ids: Vec::new(),
        focused: None,
        expanded: Vec::new(),
        scroll: view.scroll().unwrap_or_default(),
    };
    let mut focused = None;
    for node in &nodes {
        if let (Some(id), Some(entry)) = (node.runtime_id(), node.read()) {
            current.ids.push(id);
            current.expanded.push(entry.expanded);
            if entry.focused {
                current.focused = Some(id);
                focused = Some(node.clone());
            }
        }
    }
    let mut output = Vec::new();
    if let Some(old) = previous.as_ref() {
        if old.ids != current.ids {
            output.push(Change::Structure);
        } else {
            for (index, node) in nodes.iter().enumerate() {
                if old.expanded[index] != current.expanded[index] {
                    output.push(Change::Expanded(
                        node.clone(),
                        old.expanded[index],
                        current.expanded[index],
                    ));
                }
            }
        }
        if percent(old.scroll) != percent(current.scroll) {
            output.push(Change::Percent(
                percent(old.scroll),
                percent(current.scroll),
            ));
        }
        if size(old.scroll) != size(current.scroll) {
            output.push(Change::Size(size(old.scroll), size(current.scroll)));
        }
    }
    if current.focused != previous.as_ref().and_then(|old| old.focused) {
        if let Some(node) = focused {
            output.push(Change::Focus(node));
        }
    }
    *previous = Some(current);
    output
}
pub(crate) fn notify(provider: &IRawElementProviderSimple, changes: Vec<Change>) {
    unsafe {
        for change in changes {
            let object = object(provider.as_raw());
            if available(object).is_err() {
                break;
            }
            // 客户端通知可能重入；每项通知前复核撤销状态，失败不重放业务操作。
            match change {
                Change::Structure => {
                    let _ = UiaRaiseStructureChangedEvent(
                        provider,
                        StructureChangeType_ChildrenInvalidated,
                        ptr::null_mut(),
                        0,
                    );
                }
                Change::Focus(node) if node.read().is_some_and(|entry| entry.focused) => {
                    let _ = UiaRaiseAutomationEvent(
                        &child(object, node),
                        UIA_AutomationFocusChangedEventId,
                    );
                }
                Change::Expanded(node, before, after) if node.read().is_some() => {
                    let row = child(object, node);
                    let state = |expanded| {
                        if expanded {
                            ExpandCollapseState_Expanded.0
                        } else {
                            ExpandCollapseState_Collapsed.0
                        }
                    };
                    let _ = UiaRaiseAutomationPropertyChangedEvent(
                        &row,
                        UIA_ExpandCollapseExpandCollapseStatePropertyId,
                        &VARIANT::from(state(before)),
                        &VARIANT::from(state(after)),
                    );
                    if available(super::object(row.as_raw())).is_ok() {
                        let _ = UiaRaiseStructureChangedEvent(
                            &row,
                            StructureChangeType_ChildrenInvalidated,
                            ptr::null_mut(),
                            0,
                        );
                    }
                }
                Change::Percent(before, after) => {
                    let _ = UiaRaiseAutomationPropertyChangedEvent(
                        provider,
                        UIA_ScrollVerticalScrollPercentPropertyId,
                        &VARIANT::from(before),
                        &VARIANT::from(after),
                    );
                }
                Change::Size(before, after) => {
                    let _ = UiaRaiseAutomationPropertyChangedEvent(
                        provider,
                        UIA_ScrollVerticalViewSizePropertyId,
                        &VARIANT::from(before),
                        &VARIANT::from(after),
                    );
                }
                _ => {}
            }
        }
    }
}

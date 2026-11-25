//! Generate bounded notices from committed snapshots; hold neither model locks nor RefCell borrows while emitting events.
use super::scroll::{percent_value, size_value};
use super::*;
use crate::native_menu::accessibility::NotificationFrame;
pub(crate) type Snapshot = NotificationFrame;
pub(crate) enum Change {
    Structure,
    Focus(Node),
    Expanded(Node, bool, bool),
    Percent(f64, f64),
    Size(f64, f64),
}
pub(crate) fn changes(
    view: &View,
    previous: &mut Option<Snapshot>,
    listening: bool,
) -> Vec<Change> {
    let Some(current) = view.notification_frame() else {
        *previous = None;
        return Vec::new();
    };
    // Advance baselines even without listeners so the first scroll/expand after late subscription has correct old values.
    if !listening {
        *previous = Some(current);
        return Vec::new();
    }
    let node = |slot| view.snapshot_node(current.epoch.1, slot);
    // Row-slot identity/order remain unchanged within one generation; construct Nodes only for actually emitted notices.
    let focus = |frame: &NotificationFrame| frame.focused.map(|slot| (frame.epoch, slot));
    let mut output = Vec::new();
    let old = previous.as_ref();
    if let Some(old) = old {
        if old.epoch != current.epoch {
            if !old.expanded.is_empty() || !current.expanded.is_empty() {
                output.push(Change::Structure);
            }
        } else {
            for (slot, (&before, &after)) in old.expanded.iter().zip(&current.expanded).enumerate()
            {
                if before != after {
                    output.push(Change::Expanded(node(slot), before, after));
                }
            }
        }
        let (before, after) = (percent_value(old.scroll), percent_value(current.scroll));
        if before != after {
            output.push(Change::Percent(before, after));
        }
        let (before, after) = (size_value(old.scroll), size_value(current.scroll));
        if before != after {
            output.push(Change::Size(before, after));
        }
    }
    if focus(&current) != old.and_then(focus) {
        if let Some(slot) = current.focused {
            output.push(Change::Focus(node(slot)));
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
            // Client notices may reenter; recheck revocation before each notice, without replaying business operations on failure.
            match change {
                Change::Structure => {
                    let _ = UiaRaiseStructureChangedEvent(
                        provider,
                        StructureChangeType_ChildrenInvalidated,
                        ptr::null_mut(),
                        0,
                    );
                }
                Change::Focus(node)
                    if node.with_entry(|entry, active| active && entry.focused) == Some(true) =>
                {
                    let _ = UiaRaiseAutomationEvent(
                        &child(object, node),
                        UIA_AutomationFocusChangedEventId,
                    );
                }
                Change::Expanded(node, before, after) if node.with_entry(|_, _| ()).is_some() => {
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

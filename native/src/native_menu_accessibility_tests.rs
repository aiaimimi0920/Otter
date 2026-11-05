//! 快照与操作队列的撤销、限流及属性合同，不访问系统可访问性客户端。
use super::*;
fn row() -> Row {
    Row {
        key: 0,
        thumbnail: None,
        label: "&打开 && 编辑\tCtrl+O".into(),
        enabled: true,
        checked: false,
        submenu: false,
        separator: false,
        focused: false,
        radio: false,
        default: false,
    }
}
#[test]
fn names_shortcuts_and_flags_are_separate_from_command_identity() {
    let owner = Owner::new(&[row()]);
    let nodes = owner.nodes();
    let entry = nodes[0].read().unwrap();
    assert_eq!(entry.name, "打开 & 编辑");
    assert_eq!(entry.shortcut, "Ctrl+O");
    assert!(entry.enabled);
    assert!(!entry.submenu);
    nodes[0].enqueue(Action::Invoke).unwrap();
    assert_eq!(owner.take(), Some((0, Action::Invoke)));
    assert!(owner.take().is_none());
}
#[test]
fn same_named_replacement_revokes_old_node_and_queued_action() {
    let owner = Owner::new(&[row()]);
    let node = owner.nodes().remove(0);
    node.enqueue(Action::Invoke).unwrap();
    let mut replacement = row();
    replacement.key = 1;
    owner.replace(&[replacement]);
    assert!(node.read().is_none());
    assert_eq!(node.enqueue(Action::Invoke), Err(Rejected::Unavailable));
    assert!(owner.take().is_none());
    let current = owner.nodes().remove(0);
    current.enqueue(Action::Invoke).unwrap();
    drop(owner);
    assert!(current.read().is_none());
    assert_eq!(current.enqueue(Action::Invoke), Err(Rejected::Unavailable));
}
#[test]
fn disabled_separator_and_wrong_patterns_never_enter_queue() {
    let mut submenu = row();
    submenu.submenu = true;
    let mut disabled = row();
    disabled.enabled = false;
    let mut separator = row();
    separator.separator = true;
    let owner = Owner::new(&[row(), submenu, disabled, separator]);
    let nodes = owner.nodes();
    assert_eq!(nodes[0].enqueue(Action::Expand), Err(Rejected::Unsupported));
    assert_eq!(nodes[1].enqueue(Action::Invoke), Err(Rejected::Unsupported));
    assert_eq!(nodes[2].enqueue(Action::Focus), Err(Rejected::Disabled));
    assert_eq!(nodes[3].enqueue(Action::Invoke), Err(Rejected::Disabled));
    assert!(owner.take().is_none());
    nodes[1].enqueue(Action::Expand).unwrap();
    assert_eq!(owner.take(), Some((1, Action::Expand)));
}
#[test]
fn worker_requests_are_bounded_and_revoke_is_immediate() {
    let owner = Owner::new(&[row()]);
    let node = owner.nodes().remove(0);
    let worker = node.clone();
    std::thread::spawn(move || {
        for _ in 0..32 {
            worker.enqueue(Action::Focus).unwrap();
        }
        assert_eq!(worker.enqueue(Action::Focus), Err(Rejected::Busy));
    })
    .join()
    .unwrap();
    assert_eq!(owner.take(), Some((0, Action::Focus)));
    node.enqueue(Action::Focus).unwrap();
    owner.revoke();
    assert!(owner.take().is_none());
    assert!(owner.nodes().is_empty());
    assert_eq!(node.enqueue(Action::Focus), Err(Rejected::Unavailable));
}

#[test]
fn focus_only_refresh_preserves_node_and_pending_invoke() {
    let owner = Owner::new(&[row()]);
    let node = owner.nodes().remove(0);
    node.enqueue(Action::Focus).unwrap();
    node.enqueue(Action::Invoke).unwrap();
    assert_eq!(owner.take(), Some((0, Action::Focus)));
    let mut focused = row();
    focused.focused = true;
    owner.replace(&[focused]);
    assert!(node.read().unwrap().focused);
    assert_eq!(owner.take(), Some((0, Action::Invoke)));
}

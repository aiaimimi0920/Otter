use super::*;

#[test]
fn accessibility_images_use_alt_not_generic_label_or_disabled() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"named","type":"image","props":{"alt":"Map","disabled":true},"accessibility":{"label":"Wrong"}},{"id":"decorative","type":"image","accessibility":{"label":"Not an image alt"}},{"id":"number","type":"image","props":{"alt":42}}]}"#).unwrap();
    let owner = Owner::new(&scene);
    let named = owner.node("named").unwrap().read().unwrap();
    assert_eq!(named.label.as_deref(), Some("Map"));
    assert!(!named.disabled);
    assert_eq!(
        owner.node("decorative").unwrap().read().unwrap().label,
        None
    );
    assert_eq!(owner.node("number").unwrap().read().unwrap().label, None);
}

#[test]
fn accessibility_images_are_leaves_and_pending_images_have_no_subtree() {
    let scene = || {
        Scene::parse(br#"{"id":"image","type":"image","props":{"alt":"Chart"},"children":[{"id":"ignored","type":"button","props":{"label":"Not rendered"}}]}"#).unwrap()
    };
    let owner = Owner::new(&scene());
    assert!(owner.node("ignored").is_none());
    assert!(owner
        .root()
        .unwrap()
        .relative(Direction::FirstChild)
        .is_none());
    assert!(Owner::new(&scene().with_image_availability(|_| false))
        .root()
        .is_none());
}

#[test]
fn accessibility_navigation_preserves_preorder_parent_and_sibling_boundaries() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"a","type":"view","children":[{"id":"nested","type":"text","props":{"text":"Read me"}}]},{"id":"b","type":"button","props":{"label":"Run"}}]}"#).unwrap();
    let owner = Owner::new(&scene);
    let root = owner.root().unwrap();
    let first = root.relative(Direction::FirstChild).unwrap();
    assert_eq!(first.read().unwrap().id, "a");
    let last = root.relative(Direction::LastChild).unwrap();
    assert_eq!(last.read().unwrap().label.as_deref(), Some("Run"));
    assert_eq!(
        first.relative(Direction::NextSibling).unwrap().runtime_id(),
        last.runtime_id()
    );
    assert_eq!(
        last.relative(Direction::PreviousSibling)
            .unwrap()
            .runtime_id(),
        first.runtime_id()
    );
    assert_eq!(
        first.relative(Direction::Parent).unwrap().runtime_id(),
        root.runtime_id()
    );
    let nested = first.relative(Direction::FirstChild).unwrap();
    assert_eq!(nested.read().unwrap().label.as_deref(), Some("Read me"));
    assert!(nested.relative(Direction::NextSibling).is_none());
    assert!(root.relative(Direction::Parent).is_none());
}

#[test]
fn accessibility_runtime_identity_is_stable_unique_and_revoked() {
    let first = Owner::new(&scene("first"));
    let second = Owner::new(&scene("second"));
    let old = first.root().unwrap();
    assert_ne!(old.runtime_id(), second.root().unwrap().runtime_id());
    assert_eq!(old.runtime_id(), first.root().unwrap().runtime_id());
    first.revoke();
    assert!(old.runtime_id().is_none());
    assert!(old.relative(Direction::FirstChild).is_none());
}

fn scene(label: &str) -> Scene {
    Scene::parse(
        format!(r#"{{"id":"root","type":"text","accessibility":{{"label":"{label}"}}}}"#)
            .as_bytes(),
    )
    .unwrap()
}

#[test]
fn accessibility_snapshot_filters_hidden_and_icon_children() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"hidden","type":"view","props":{"visible":false},"children":[{"id":"secret","type":"text"}]},{"id":"icon","type":"icon","children":[{"id":"ignored","type":"text"}]},{"id":"switch","type":"switch","accessibility":{"label":"Power"}}]}"#).unwrap();
    let owner = Owner::new(&scene);
    assert_eq!(owner.0.entries.len(), 3);
    for id in ["hidden", "secret", "ignored"] {
        assert!(owner.node(id).is_none());
    }
    let entry = owner.node("switch").unwrap().read().unwrap();
    assert_eq!(entry.label.as_deref(), Some("Power"));
    assert_eq!(entry.parent, Some(0));
    assert_eq!(entry.kind, Kind::Switch);
}

#[test]
fn accessibility_same_id_replacement_never_revives_old_handle() {
    let owner = Owner::new(&scene("old"));
    let old = owner.node("root").unwrap();
    owner.revoke();
    let replacement = Owner::new(&scene("new"));
    assert!(old.read().is_none());
    assert!(owner.node("root").is_none());
    assert_eq!(
        replacement
            .node("root")
            .unwrap()
            .read()
            .unwrap()
            .label
            .as_deref(),
        Some("new")
    );
}

#[test]
fn accessibility_drop_releases_snapshot_despite_retained_handles() {
    let owner = Owner::new(&scene("label"));
    let handle = owner.node("root").unwrap();
    let copy = handle.clone();
    drop(owner);
    assert!(handle.read().is_none());
    assert!(copy.read().is_none());
    assert!(handle.snapshot.upgrade().is_none());
}

#[test]
fn accessibility_cross_thread_read_after_revoke_is_unavailable() {
    let owner = Owner::new(&scene("label"));
    let handle = owner.node("root").unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        rx.recv().unwrap();
        assert!(handle.read().is_none());
    });
    owner.revoke();
    tx.send(()).unwrap();
    reader.join().unwrap();
}

#[test]
fn accessibility_wrong_label_type_and_hidden_root_are_not_exposed() {
    let scene = Scene::parse(
        br#"{"id":"root","type":"input","props":{"value":"private"},"accessibility":{"label":42}}"#,
    )
    .unwrap();
    let owner = Owner::new(&scene);
    assert_eq!(owner.node("root").unwrap().read().unwrap().label, None);
    let hidden = Scene::parse(br#"{"id":"root","type":"text","props":{"visible":false}}"#).unwrap();
    assert!(Owner::new(&hidden).node("root").is_none());
}

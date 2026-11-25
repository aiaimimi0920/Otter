//! Disabling notifications must neither block image display nor retrigger execution through relays or imported cycles.
use super::super::tests::{id, link, sticker};
use super::*;

#[test]
fn source_and_intermediate_policy_stop_notifications_not_image_resolution() {
    let ids = [id("source"), id("relay"), id("target")];
    let mut nodes = ids.iter().map(sticker).collect::<Vec<_>>();
    nodes[0].source = Some("formal-source");
    nodes[2].kind = Kind::Art;
    let links = [
        link("a", &ids[0], &ids[1], "image"),
        link("b", &ids[1], &ids[2], "input"),
    ];
    let graph = Graph::new(&nodes, &links).unwrap();
    assert_eq!(graph.notification_targets(&ids[0]).unwrap(), vec![&ids[2]]);
    for blocked in [&ids[0], &ids[1]] {
        assert!(graph
            .notification_targets_with(&ids[0], |unit| unit != blocked)
            .unwrap()
            .is_empty());
        assert_eq!(graph.image(&ids[1]), Some("formal-source"));
        assert_eq!(
            graph.connected_image(&ids[2], "input"),
            Some("formal-source")
        );
    }
    // The target's own notify affects only its successors and must not prevent it from receiving this upstream notification.
    assert_eq!(
        graph
            .notification_targets_with(&ids[0], |unit| unit != &ids[2])
            .unwrap(),
        vec![&ids[2]]
    );
}
#[test]
fn alternate_enabled_path_delivers_once_despite_disabled_relay_and_cycle() {
    let ids = [id("source"), id("blocked"), id("enabled"), id("target")];
    let mut nodes = ids.iter().map(sticker).collect::<Vec<_>>();
    nodes[3].kind = Kind::Art;
    let links = [
        link("a", &ids[0], &ids[1], "image"),
        link("b", &ids[1], &ids[3], "a"),
        link("c", &ids[0], &ids[2], "image"),
        link("d", &ids[2], &ids[3], "b"),
        link("cycle", &ids[2], &ids[0], "image"),
    ];
    let graph = Graph::new(&nodes, &links).unwrap();
    assert_eq!(
        graph
            .notification_targets_with(&ids[0], |unit| unit != &ids[1])
            .unwrap(),
        vec![&ids[3]]
    );
    assert!(graph
        .notification_targets_with(&ids[0], |unit| unit != &ids[0])
        .unwrap()
        .is_empty());
}

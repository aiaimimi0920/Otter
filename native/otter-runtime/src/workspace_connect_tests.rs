//! 交互连线与底层导入合同分开验证；所有失败比较完整边集合，不能只比较数量。
use super::*;
fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
fn edge(name: &str, from: &str, to: &str, port: &str) -> Link {
    Link {
        id: id(name),
        from_unit: id(from),
        from_port: "output_image".into(),
        to_unit: id(to),
        to_port: port.into(),
    }
}
fn workspace() -> Workspace {
    let mut workspace = Workspace::default();
    workspace
        .add_units([id("a"), id("b"), id("c"), id("d")])
        .unwrap();
    workspace
}
#[test]
fn single_input_replacement_preserves_other_ports_and_appends() {
    let mut graph = workspace();
    let first = edge("old", "a", "c", "input_image");
    graph.add_link(first.clone()).unwrap();
    graph.add_link(edge("other", "a", "c", "mask")).unwrap();
    graph
        .add_link(edge("unrelated", "a", "d", "input_image"))
        .unwrap();
    let replacement = edge("new", "b", "c", "input_image");
    assert_eq!(graph.connect(replacement.clone()), Ok(Some(first)));
    assert_eq!(
        graph.links(),
        &[
            edge("other", "a", "c", "mask"),
            edge("unrelated", "a", "d", "input_image"),
            replacement
        ]
    );
}
#[test]
fn invalid_drop_never_disconnects_existing_input() {
    let mut graph = workspace();
    graph
        .add_link(edge("old", "a", "b", "input_image"))
        .unwrap();
    graph
        .add_link(edge("next", "b", "c", "input_image"))
        .unwrap();
    let before = graph.links().to_vec();
    for (candidate, error) in [
        (edge("self", "b", "b", "input_image"), Error::SelfConnection),
        (edge("cycle", "c", "b", "input_image"), Error::Cycle),
        (edge("old", "d", "b", "input_image"), Error::DuplicateId),
        (
            edge("missing", "missing", "b", "input_image"),
            Error::MissingUnit,
        ),
        (edge("invalid", "d", "b", ""), Error::InvalidPort),
    ] {
        assert_eq!(graph.connect(candidate), Err(error));
        assert_eq!(graph.links(), before);
    }
}
#[test]
fn imported_cycles_terminate_and_do_not_forbid_unrelated_edges() {
    let mut graph = workspace();
    graph.add_link(edge("ab", "a", "b", "input_image")).unwrap();
    graph.add_link(edge("ba", "b", "a", "input_image")).unwrap();
    assert_eq!(graph.connect(edge("ca", "c", "a", "mask")), Ok(None));
    let before = graph.links().to_vec();
    assert_eq!(
        graph.connect(edge("bc", "b", "c", "input_image")),
        Err(Error::Cycle)
    );
    assert_eq!(graph.links(), before);
    assert_eq!(graph.connect(edge("cd", "c", "d", "input_image")), Ok(None));
}
#[test]
fn full_budget_allows_replacement_but_rejects_new_input_atomically() {
    let mut graph = workspace();
    for index in 0..MAX_LINKS {
        graph
            .add_link(edge(
                &format!("edge-{index}"),
                "a",
                "b",
                &format!("port-{index}"),
            ))
            .unwrap();
    }
    assert!(graph
        .connect(edge("replacement", "c", "b", "port-0"))
        .unwrap()
        .is_some());
    assert_eq!(graph.links().len(), MAX_LINKS);
    let before = graph.links().to_vec();
    assert_eq!(
        graph.connect(edge("overflow", "c", "b", "new-port")),
        Err(Error::Capacity)
    );
    assert_eq!(graph.links(), before);
}
#[test]
fn full_unit_chain_checks_cycle_with_bounded_queue() {
    let mut graph = Workspace::default();
    graph
        .add_units((0..MAX_UNITS).map(|index| id(&index.to_string())))
        .unwrap();
    for index in 1..MAX_UNITS {
        graph
            .connect(edge(
                &format!("edge-{index}"),
                &(index - 1).to_string(),
                &index.to_string(),
                "input_image",
            ))
            .unwrap();
    }
    let before = graph.links().to_vec();
    assert_eq!(
        graph.connect(edge(
            "cycle",
            &(MAX_UNITS - 1).to_string(),
            "0",
            "input_image"
        )),
        Err(Error::Cycle)
    );
    assert_eq!(graph.links(), before);
}

//! 跨节点视觉范围按场景先序，保留独立端点并拒绝聚合文字复制。
use super::*;
pub(super) fn cross_owner(reverse: bool) -> Presentation {
    cross_owner_changed(reverse, "")
}
#[test]
fn projected_selection_matches_independent_legacy_ranges_for_all_endpoints() {
    for reverse in [false, true] {
        for change in ["", "selectable", "empty"] {
            let mut p = cross_owner_changed(reverse, change);
            let start = point(&p, "n0", false);
            p.begin_text_selection(start).unwrap();
            let mut endpoints: Vec<_> = p
                .text
                .items()
                .iter()
                .flat_map(|item| {
                    (0..=item.layout().text_len())
                        .map(move |offset| (item.node_id().to_owned(), offset))
                })
                .collect();
            endpoints.push(("missing".into(), 0));
            for (a, anchor) in &endpoints {
                for (b, focus) in &endpoints {
                    let s = p.selection.as_mut().unwrap();
                    s.node = a.clone();
                    s.focus_node = b.clone();
                    s.anchor = *anchor;
                    s.focus = *focus;
                    let projected = s.project(&p.text);
                    for (index, item) in p.text.items().iter().enumerate() {
                        assert_eq!(
                            projected.and_then(|span| span.item_range(index, item)),
                            s.item_range(item, &p.text),
                            "{change}: {a}:{anchor}->{b}:{focus}, index={index}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn cross_node_vertical_keyboard_preserves_global_column_across_local_lines() {
    for mode in ["column", "row", "reverse"] {
        let children: Vec<_> = (0..3).map(|i| serde_json::json!({
            "id":format!("n{i}"),"type":"text","props":{"text":"AAAA\nAAAA","selectable":true},"style":{"whiteSpace":"pre"},
            "layout":{"position":"absolute","left":match mode {"column"=>0,"row"=>i*100,_=>(2-i)*100},
                "top":if mode=="column" {i*60} else {0},"width":80,"height":55}
        })).collect();
        let scene = serde_json::json!({"id":"root","type":"stack","layout":{"position":"relative","width":300,"height":200},"children":children});
        let mut p = Presentation::new(
            Scene::parse(&serde_json::to_vec(&scene).unwrap()).unwrap(),
            Style {
                size: 12.,
                ..style()
            },
            context(),
            [300., 200.],
        )
        .unwrap();
        let a = point(&p, "n0", false);
        let item = &p.text.items()[2];
        let rect = item.layout().cluster_rect(2).unwrap();
        let c = [
            item.origin()[0] + rect.left + 0.1,
            item.origin()[1] + (rect.top + rect.bottom) / 2.,
        ];
        p.begin_text_selection(a).unwrap();
        p.finish_text_selection(c).unwrap();
        let offset = match mode {
            "column" => 2,
            "row" => 4,
            _ => 0,
        };
        for (down, node, focus) in [
            (false, "n1", 5 + offset),
            (false, "n1", offset),
            (false, "n0", 5 + offset),
            (true, "n1", offset),
        ] {
            assert!(p.vertical_text_selection("n0", down).unwrap());
            let s = p.selection.as_ref().unwrap();
            assert_eq!(
                (&*s.focus_node, s.focus),
                (node, focus),
                "{mode}, down={down}"
            );
        }
    }
}
#[test]
fn cross_node_keyboard_steps_focus_owner_and_preserves_plain_arrow_selection() {
    for reverse in [false, true] {
        let mut p = cross_owner(reverse);
        let a = point(&p, "n0", false);
        let c = point(&p, "n2", false);
        p.begin_text_selection(a).unwrap();
        p.finish_text_selection(c).unwrap();
        assert!(!p.step_text_selection("n0", true, false).unwrap());
        assert!(!p.step_text_selection("n2", true, true).unwrap());
        assert!(p.vertical_text_selection("n0", true).unwrap());
        assert!(p.step_text_selection("n0", false, true).unwrap());
        let s = p.selection.as_ref().unwrap();
        assert_eq!(s.focus_node, "n1");
        assert_eq!((s.anchor, s.focus), (0, 5));
        p.step_text_selection("n0", false, true).unwrap();
        assert_eq!(p.selection.as_ref().unwrap().focus, 4);
        p.line_edge_text_selection("n0", false).unwrap();
        assert_eq!(p.selection.as_ref().unwrap().focus, 0);
        p.line_edge_text_selection("n0", true).unwrap();
        assert_eq!(p.selection.as_ref().unwrap().focus, 5);
        p.step_text_selection("n0", true, true).unwrap();
        let s = p.selection.as_ref().unwrap();
        assert_eq!(s.focus_node, "n2");
        assert_eq!(s.focus, 0);
        assert!(p.selected_text().unwrap().is_none());
    }
}
#[test]
fn keyboard_selection_crosses_from_same_node_and_returns_without_index_aliasing() {
    let mut p = cross_owner(false);
    let b = point(&p, "n1", true);
    p.begin_text_selection(b).unwrap();
    p.finish_text_selection(b).unwrap();
    p.step_text_selection("n1", true, true).unwrap();
    assert_eq!(p.selection.as_ref().unwrap().focus_node, "n2");
    p.step_text_selection("n1", true, true).unwrap();
    assert_eq!(p.selection.as_ref().unwrap().focus, 1);
    p.step_text_selection("n1", false, true).unwrap();
    p.step_text_selection("n1", false, true).unwrap();
    let s = p.selection.as_ref().unwrap();
    assert_eq!(s.focus_node, "n1");
    assert_eq!((s.anchor, s.focus), (5, 5));
    p.step_text_selection("n1", false, true).unwrap();
    assert_eq!(p.selected_text().unwrap().as_deref(), Some("o"));
    p.line_edge_text_selection("n1", true).unwrap();
    for _ in 0..20 {
        p.step_text_selection("n1", true, true).unwrap();
    }
    let s = p.selection.as_ref().unwrap();
    assert_eq!(s.focus_node, "n2");
    assert_eq!(s.focus, 7);
}
#[test]
fn keyboard_selection_skips_nonselectable_owner_and_stays_inside_surface() {
    let mut p = cross_owner_changed(false, "selectable");
    let a = point(&p, "n0", true);
    p.begin_text_selection(a).unwrap();
    p.finish_text_selection(a).unwrap();
    p.step_text_selection("n0", true, true).unwrap();
    assert_eq!(p.selection.as_ref().unwrap().focus_node, "n2");
    assert_eq!(p.selection.as_ref().unwrap().focus, 0);
    p.step_text_selection("n0", false, true).unwrap();
    assert_eq!(p.selection.as_ref().unwrap().focus_node, "n0");
    p.line_edge_text_selection("n0", false).unwrap();
    p.step_text_selection("n0", false, true).unwrap();
    assert_eq!(p.selection.as_ref().unwrap().focus, 0);
}
#[test]
fn cross_node_word_drag_switches_boundary_owner_and_returns_to_initial_word() {
    for reverse in [false, true] {
        let mut p = cross_owner(reverse);
        let middle = point(&p, "n1", false);
        let first = point(&p, "n0", false);
        let last = point(&p, "n2", false);
        p.begin_word_selection(middle).unwrap();
        assert_eq!(p.selected_text().unwrap().as_deref(), Some("Bravo"));
        p.update_text_selection(last).unwrap();
        let s = p.selection.as_ref().unwrap();
        assert_eq!((s.anchor, s.focus), (0, 7));
        assert_eq!(s.focus_node, "n2");
        assert!(p.selected_text().unwrap().is_none());
        p.update_text_selection(first).unwrap();
        let s = p.selection.as_ref().unwrap();
        assert_eq!((s.anchor, s.focus), (5, 0));
        assert_eq!(s.focus_node, "n0");
        p.finish_text_selection(middle).unwrap();
        assert_eq!(p.selected_text().unwrap().as_deref(), Some("Bravo"));
        assert!(p.selection.as_ref().unwrap().word.is_none());
    }
}
#[test]
fn cross_node_word_drag_keeps_utf16_ranges_owned_by_each_node() {
    let mut p = cross_owner_changed(false, "unicode");
    let middle = point(&p, "n1", false);
    p.begin_word_selection(middle).unwrap();
    assert_eq!(p.selected_text().unwrap().as_deref(), Some("Bravo "));
    let last = point(&p, "n2", true);
    p.update_text_selection(last).unwrap();
    let s = p.selection.as_ref().unwrap();
    assert_eq!((s.anchor, s.focus), (0, 4));
    assert_eq!(s.item_range(&p.text.items()[2], &p.text), Some(0..4));
    let first = point(&p, "n0", true);
    p.update_text_selection(first).unwrap();
    let s = p.selection.as_ref().unwrap();
    assert_eq!((s.anchor, s.focus), (5, 2));
    assert_eq!(s.item_range(&p.text.items()[0], &p.text), Some(2..4));
    p.finish_text_selection(middle).unwrap();
    assert_eq!(p.selected_text().unwrap().as_deref(), Some("Bravo"));
}
#[test]
fn cross_node_word_drag_matches_browser_core_ranges_after_initial_space() {
    let children: Vec<_> = ["Alpha small end", "Bravo middle end", "Charlie final end"]
        .iter()
        .enumerate()
        .map(|(i, text)| {
            serde_json::json!({
                "id":format!("n{i}"),"type":"text","props":{"text":text,"selectable":true},
                "layout":{"width":240,"height":32}
            })
        })
        .collect();
    let scene = serde_json::json!({"id":"root","type":"column","children":children});
    let mut p = Presentation::new(
        Scene::parse(&serde_json::to_vec(&scene).unwrap()).unwrap(),
        style(),
        context(),
        [300., 150.],
    )
    .unwrap();
    let at = |owner: &Presentation, node: usize, offset| {
        let item = &owner.text.items()[node];
        let rect = item.layout().cluster_rect(offset).unwrap();
        [
            item.origin()[0] + rect.left + (rect.right - rect.left) * 0.25,
            item.origin()[1] + (rect.top + rect.bottom) / 2.,
        ]
    };
    let start = at(&p, 1, 8);
    p.begin_word_selection(start).unwrap();
    assert_eq!(p.selected_text().unwrap().as_deref(), Some("middle "));
    // 未移动抬起不裁掉初始空白。
    p.finish_text_selection(start).unwrap();
    assert_eq!(p.selected_text().unwrap().as_deref(), Some("middle "));
    for delta in [0.1, 1., 2., 3., 4., 6.] {
        p.begin_word_selection(start).unwrap();
        p.finish_text_selection([start[0] + delta, start[1]])
            .unwrap();
        assert_eq!(p.selected_text().unwrap().as_deref(), Some("middle"));
    }
    p.begin_word_selection(start).unwrap();
    for (node, offset, anchor, focus) in [
        (2, 9, 6, 13),
        (0, 8, 12, 6),
        (1, 0, 12, 0),
        (1, 14, 6, 16),
        (1, 8, 6, 12),
    ] {
        let target = at(&p, node, offset);
        p.update_text_selection(target).unwrap();
        let s = p.selection.as_ref().unwrap();
        assert_eq!(
            (s.anchor, s.focus),
            (anchor, focus),
            "node={node}, offset={offset}"
        );
    }
    p.finish_text_selection(start).unwrap();
    assert_eq!(p.selected_text().unwrap().as_deref(), Some("middle"));
}
fn cross_owner_changed(reverse: bool, change: &str) -> Presentation {
    let mut children: Vec<_> = ["Alpha", "Bravo", "Charlie"].iter().enumerate().map(|(i,text)| {
        serde_json::json!({"id":format!("n{i}"),"type":"text","props":{"text":text,"selectable":true},
            "layout":{"position":"absolute","left":if reverse {(2-i)*60} else {i*60},"top":0,"width":60,"height":30}})
    }).collect();
    match change {
        "empty" => children[1]["props"]["text"] = "".into(),
        "unicode" => {
            for (node, text) in children.iter_mut().zip(["你好世界", "Bravo x", "a 😀"]) {
                node["props"]["text"] = text.into();
            }
        }
        "swap" => children.swap(0, 1),
        "remove" => {
            children.remove(1);
        }
        "insert" => {
            let mut node = children[1].clone();
            node["id"] = "added".into();
            children.insert(1, node);
        }
        "text" => children[1]["props"]["text"] = "Changed".into(),
        "selectable" => children[1]["props"]["selectable"] = false.into(),
        _ => {}
    }
    let scene = serde_json::json!({"id":"root","type":"stack","layout":{"position":"relative","width":200,"height":100},"children":children});
    Presentation::new(
        Scene::parse(&serde_json::to_vec(&scene).unwrap()).unwrap(),
        Style {
            size: 12.,
            ..style()
        },
        context(),
        [200., 100.],
    )
    .unwrap()
}
#[test]
fn cross_node_reconcile_rejects_changed_order_membership_text_and_permission() {
    for change in ["swap", "remove", "insert", "text", "selectable"] {
        let mut p = cross_owner(false);
        let a = point(&p, "n0", false);
        let c = point(&p, "n2", true);
        p.begin_text_selection(a).unwrap();
        p.finish_text_selection(c).unwrap();
        assert!(p.selection.as_ref().unwrap().cross_node());
        let next = cross_owner_changed(false, change);
        p.reconcile_text_selection(&next.text);
        assert!(p.selection.is_none(), "{change}");
    }
}
pub(super) fn point(owner: &Presentation, id: &str, end: bool) -> [f32; 2] {
    let item = owner
        .text
        .items()
        .iter()
        .find(|item| item.node_id() == id)
        .unwrap();
    let rect = item
        .layout()
        .cluster_rect(if end { item.layout().text_len() - 1 } else { 0 })
        .unwrap();
    [
        item.origin()[0]
            + if end {
                rect.right - 0.1
            } else {
                rect.left + 0.1
            },
        item.origin()[1] + (rect.top + rect.bottom) / 2.,
    ]
}
#[test]
fn cross_node_ranges_follow_scene_not_visual_order_and_do_not_copy() {
    for reverse in [false, true] {
        let mut p = cross_owner(reverse);
        let a = point(&p, "n0", false);
        let c = point(&p, "n2", true);
        assert!(p.begin_text_selection(a).unwrap());
        assert!(p.finish_text_selection(c).unwrap());
        let selection = p.selection.as_ref().unwrap();
        assert_eq!(selection.node, "n0");
        assert_eq!(selection.focus_node, "n2");
        for item in p.text.items() {
            assert_eq!(
                selection.item_range(item, &p.text),
                Some(0..item.layout().text_len())
            );
        }
        assert_eq!(p.selected_text().unwrap(), None);
        assert!(p.text_selection_keyboard_ready("n0", true));
        p.reflow(style(), context(), [200., 100.]).unwrap();
        assert!(p.selection.as_ref().unwrap().cross_node());
    }
}
#[test]
fn cross_node_reverse_endpoints_preserve_partial_edges_and_can_return_home() {
    let mut p = cross_owner(false);
    let c = point(&p, "n2", false);
    let a = point(&p, "n0", true);
    p.begin_text_selection(c).unwrap();
    p.update_text_selection(a).unwrap();
    let s = p.selection.as_ref().unwrap();
    assert_eq!(s.item_range(&p.text.items()[0], &p.text), Some(5..5));
    assert_eq!(s.item_range(&p.text.items()[1], &p.text), Some(0..5));
    assert_eq!(s.item_range(&p.text.items()[2], &p.text), Some(0..0));
    let end = point(&p, "n2", true);
    p.finish_text_selection(end).unwrap();
    assert_eq!(p.selected_text().unwrap().as_deref(), Some("Charlie"));
}

//! 跨字段依赖、无handler草稿表单、权威同步与取消；预期值为独立字符串/布尔常量。
use super::*;
#[test]
fn local_draft_is_not_reintroduced_into_a_form_after_its_field_is_removed() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (_, w) = setup_form(false);
    let h = edit(&w, [4, 4]);
    unsafe {
        replace(h, "must not return");
    }
    let mut v = form(false);
    v["children"].as_array_mut().unwrap().remove(0);
    let (next, _next_window) = setup(v);
    let p = crate::tile_surface_value::tests::presentation(&next);
    let mut commands =
        crate::tile_surface_click::prepare(&next.frame, &next.art, &p, [4, 24], Default::default())
            .unwrap();
    assert_eq!(p.hit_test([4., 24.]), Some("send"));
    assert!(!commands.is_empty());
    text::decorate(&w.state, &mut commands).unwrap();
    let Kind::Event(event) = &commands[0].kind else {
        panic!()
    };
    assert_eq!(event["payload"]["surfaceValues"], json!({"second":"two"}));
}
#[test]
fn new_authority_before_paint_never_settles_against_the_old_edit_source() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scene("input"));
    let h = edit(&w, [4, 4]);
    unsafe {
        replace(h, "local");
    }
    assert!(text::commit_for_click(&w.state));
    {
        let mut s = f.shared.lock();
        while let Some(c) = s.actions.take(&f.state, Instant::now()) {
            s.actions
                .attempted(&c, &f.ack(&c, "running"), Instant::now());
        }
    }
    let mut updated = scene("input");
    updated["props"]["value"] = json!("new authority");
    let next = fixture(updated);
    f.shared
        .lock()
        .actions
        .observe(next.state.clone(), "origin", "device", Instant::now());
    f.shared
        .lock()
        .unavailable(f.job.generation, &f.job.key, "preparing text result");
    w.tick().unwrap();
    let o = w.state.text.borrow();
    let slot = o.as_ref().unwrap().fields.values().next().unwrap();
    assert_eq!(slot.session.value(), "local");
    assert!(slot.session.pending() > 0);
}
fn form(handlers: bool) -> Value {
    let mut v = json!({"id":"root","type":"column","layout":{"width":64,"height":64},"children":[
        {"id":"first","type":"input","layout":{"width":64,"height":20},"props":{"value":"one"}},
        {"id":"second","type":"textarea","layout":{"width":64,"height":20},"props":{"value":"two"}},
        {"id":"send","type":"button","layout":{"width":64,"height":24},"props":{"label":"Send","includeSurfaceValues":true},"events":{"click":"submit"}}]});
    // 显式控制盒最小高度，不让默认30px挤压其它字段。
    v["children"][1]["layout"]["minHeight"] = json!(20);
    v["children"][2]["layout"]["minHeight"] = json!(24);
    if handlers {
        for i in 0..2 {
            v["children"][i]["events"] = json!({"input":"typing","change":"save"});
        }
    }
    v
}
fn setup_form(handlers: bool) -> (Fixture, Window) {
    let (f, w) = setup(form(handlers));
    let p = crate::tile_surface_value::tests::presentation(&f);
    for (id, y, height, pixel) in [
        ("first", 0., 20., [4, 4]),
        ("second", 20., 20., [4, 24]),
        ("send", 40., 24., [4, 50]),
    ] {
        let g = p.geometry().nodes().iter().find(|g| g.id() == id).unwrap();
        assert_eq!(g.origin(), [0., y], "{id}");
        assert_eq!(g.size(), [64., height], "{id}");
        assert_eq!(p.hit_test(pixel.map(|v| v as f32 + 0.5)), Some(id));
        if id != "send" {
            assert_eq!(text::field(&w.state, pixel).unwrap().node, id);
        }
    }
    (f, w)
}
#[test]
fn handlerless_fields_keep_local_drafts_and_submit_current_values_without_fake_change() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup_form(false);
    let a = edit(&w, [4, 4]);
    unsafe {
        replace(a, "本地🙂");
    }
    let b = edit(&w, [4, 24]);
    unsafe {
        replace(b, "第一行\r\n第二行");
    }
    assert!(events(&f).is_empty());
    assert!(text::commit_for_click(&w.state));
    assert!(w.test_art_click([4, 50]));
    let actual = events(&f);
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0]["event"], "click");
    assert_eq!(
        actual[0]["payload"]["surfaceValues"],
        json!({"first":"本地🙂","second":"第一行\n第二行"})
    );
    w.tick().unwrap();
    assert_eq!(w.state.text_preview.borrow().len(), 2);
    assert!(std::str::from_utf8(&f.art.snapshot.scene)
        .unwrap()
        .contains("one"));
}
#[test]
fn two_text_fields_and_later_button_share_ordered_prefix_and_failed_group_cancels_suffix() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup_form(true);
    let a = edit(&w, [4, 4]);
    unsafe {
        replace(a, "first edited");
    }
    let b = edit(&w, [4, 24]);
    unsafe {
        replace(b, "second edited");
    }
    assert!(text::commit_for_click(&w.state));
    assert!(w.test_art_click([4, 50]));
    let mut s = f.shared.lock();
    assert_eq!(s.actions.pending.len(), 5);
    let edits = s
        .actions
        .pending
        .iter()
        .map(|c| c.edit.clone().unwrap())
        .collect::<Vec<_>>();
    assert!(edits.iter().all(|e| e.same_group(&edits[0])));
    assert!(!Arc::ptr_eq(&edits[0], &edits[2]));
    let first = s.actions.take(&f.state, Instant::now()).unwrap();
    s.actions
        .attempted(&first, &f.ack(&first, "failed"), Instant::now());
    assert!(s.actions.pending.is_empty());
    assert!(!edits[0].active());
}
#[test]
fn authoritative_writeback_does_not_replace_a_newer_unsent_draft_or_emit_a_change() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(scene("input"));
    let h = edit(&w, [4, 4]);
    unsafe {
        replace(h, "first");
    }
    let first = f
        .shared
        .lock()
        .actions
        .take(&f.state, Instant::now())
        .unwrap();
    f.shared
        .lock()
        .actions
        .attempted(&first, &f.ack(&first, "running"), Instant::now());
    unsafe {
        replace(h, "newer");
    }
    let mut updated = scene("input");
    updated["props"]["value"] = json!("server result");
    let next = fixture(updated);
    {
        let mut s = f.shared.lock();
        s.actions
            .observe(next.state.clone(), "origin", "device", Instant::now());
        let n = Frame {
            size: f.frame.size,
            key: f.frame.key.clone(),
            layout: f.frame.layout.clone(),
            images: f.frame.images.clone(),
            arts: next.frame.arts.clone(),
            live_until: None,
        };
        s.frame = Some(Arc::new(n));
        s.presented_frame = None;
    }
    w.tick().unwrap();
    w.tick().unwrap();
    assert_eq!(
        w.state
            .text
            .borrow()
            .as_ref()
            .unwrap()
            .fields
            .values()
            .next()
            .unwrap()
            .session
            .value(),
        "newer"
    );
    assert!(events(&f).iter().all(|e| e["event"] != "change"));
}

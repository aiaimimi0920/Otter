//! 独立常量核空隙前后56px偏移；减权/身份变化不复活旧偏移，不保旧Art或图片。
use super::*;
use serde_json::json;
fn fixture() -> crate::tile_surface_actions::tests::Fixture {
    crate::tile_surface_value::tests::fixture(
        json!({"id":"root","type":"scroll","layout":{"width":64,"height":64},"children":[
        {"id":"gap","type":"view","layout":{"height":100,"minHeight":100}},
        {"id":"last","type":"button","layout":{"height":20,"minHeight":20}}]}),
    )
}
fn prepare(cache: &mut BTreeMap<String, Prepared>, frame: Option<&Frame>) {
    super::super::super::prepare(cache, frame).unwrap();
}
fn top(cache: &BTreeMap<String, Prepared>) -> f32 {
    cache["art"]
        .presentation
        .geometry()
        .nodes()
        .iter()
        .find(|g| g.id() == "last")
        .unwrap()
        .origin()[1]
}
fn scrolled(
    f: &crate::tile_surface_actions::tests::Fixture,
) -> (Memory, BTreeMap<String, Prepared>) {
    let mut cache = BTreeMap::new();
    prepare(&mut cache, Some(&f.frame));
    let scroll = cache["art"]
        .presentation
        .planned_wheel([8., 8.], 1, 1000., false)
        .unwrap();
    install(&mut cache, &f.frame, "art", &scroll).unwrap();
    assert_eq!(top(&cache), 44.);
    let mut memory = Memory::default();
    memory.synchronize(Some(&f.frame), &cache, &f.shared.lock());
    (memory, cache)
}
#[test]
fn preparation_gap_keeps_only_scroll_and_restores_after_dropping_the_presentation() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let f = fixture();
    let (mut memory, mut cache) = scrolled(&f);
    f.shared
        .lock()
        .unavailable(f.job.generation, &f.frame.key, "prepare");
    prepare(&mut cache, None);
    assert!(cache.is_empty());
    memory.synchronize(None, &cache, &f.shared.lock());
    assert_eq!(memory.entries.len(), 1);
    prepare(&mut cache, Some(&f.frame));
    assert_eq!(top(&cache), 100.);
    memory.restore(Some(&f.frame), &mut cache).unwrap();
    assert_eq!(top(&cache), 44.);
}
#[test]
fn lifecycle_boundaries_clear_scroll_memory_instead_of_reviving_it_after_a_gap() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for case in 0..9 {
        let f = fixture();
        let (mut memory, mut cache) = scrolled(&f);
        prepare(&mut cache, None);
        {
            let mut s = f.shared.lock();
            match case {
                0 => s.stopped = true,
                1 => s.expired = true,
                2 => s.black = true,
                3 => s.frozen = true,
                4 => s.until = Some(std::time::Instant::now()),
                5 => s.desired = None,
                6 => s.desired.as_mut().unwrap().lease = "new".into(),
                7 => s.actions.states.clear(),
                _ => s.until = None,
            }
            memory.synchronize(None, &cache, &s);
        }
        assert!(memory.entries.is_empty(), "case {case}");
        prepare(&mut cache, Some(&f.frame));
        memory.restore(Some(&f.frame), &mut cache).unwrap();
        assert_eq!(top(&cache), 100., "case {case}");
    }
}
#[test]
fn replacement_output_view_generation_or_source_size_never_inherits_saved_scroll() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for case in 0..5 {
        let f = fixture();
        let (memory, mut cache) = scrolled(&f);
        prepare(&mut cache, None);
        let mut view = f.art.view.clone();
        if case == 2 {
            view.attachment_id = "other".into();
        }
        let art = Arc::new(Art {
            view,
            generation: if case == 3 { 2 } else { 1 },
            snapshot: f.art.snapshot.clone(),
            size: if case == 4 { [65, 64] } else { f.art.size },
            images: Default::default(),
        });
        let mut key = f.frame.key.clone();
        if case == 0 {
            key.lease = "new".into();
        }
        let frame = Frame {
            key,
            layout: f.frame.layout.clone(),
            size: if case == 1 {
                crate::tile_wire::Size {
                    width: 3,
                    height: 2,
                }
            } else {
                f.frame.size
            },
            images: Default::default(),
            arts: [("art".into(), art)].into(),
            live_until: None,
        };
        prepare(&mut cache, Some(&frame));
        memory.restore(Some(&frame), &mut cache).unwrap();
        assert_eq!(top(&cache), 100., "case {case}");
    }
}

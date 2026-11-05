//! 独立Wall场景/ACK oracle与队列生命周期；夹具不走生产编码计算预期状态。
use super::*;
use crate::{
    tile_assets::Job,
    tile_shared::{Frame, Shared},
    tile_surface_owner::Art,
};
use serde_json::{json, Value};
pub(crate) struct Fixture {
    pub shared: Arc<Shared>,
    pub job: Job,
    pub state: Arc<State>,
    pub art: Arc<Art>,
    pub frame: Arc<Frame>,
}
pub(crate) fn value(key: &crate::tile_shared::Key, instance: &str) -> Value {
    json!({"protocolVersion":"loom.wall.v1","view":{"binding":{"endpointId":key.endpoint,"leaseId":key.lease,"revision":key.revision},"instanceId":instance,"attachmentId":"view"},
        "width":64,"height":64,"generation":1,"sequence":0,"preview":null,"result":null,"confirmations":[],"pending":[],"failure":null,
        "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":instance,"attachmentId":"view","artId":"fixture","artVersion":"1","revision":1,"runtime":"declarative","resources":[],"resourceLeases":[],
            "scene":{"id":"button","type":"button","layout":{"width":64,"height":64},"props":{"label":"Run","eventPayload":{"known":7}},"events":{"click":"run"}}}})
}
pub(crate) fn ack_value(event: &str, status: &str) -> Value {
    json!({"protocolVersion":"loom.surface.v1","instanceId":"art","eventId":event,"requestId":"request","accepted":true,"status":status})
}
pub(crate) fn confirmation(event: &str, endpoint: &str) -> Value {
    json!({"protocolVersion":"loom.surface.v1","confirmationId":"confirm","instanceId":"art","attachmentId":"view","deviceId":"device","hookNodeId":format!("wall:{endpoint}"),"eventId":event,"requestId":"request","actionId":"run","risk":"high","expiresAtMs":crate::tile_runtime::now_ms()+60000,"payload":{"known":7}})
}
impl Fixture {
    pub(crate) fn new(instance: &str) -> Self {
        Self::new_generation(instance, 1)
    }
    fn new_generation(instance: &str, generation: u64) -> Self {
        let (shared, mut frame) = crate::tile_tests::shared_frame();
        frame.layout.placements.push(crate::tile_wire::Placement {
            placement_id: "interactive".into(),
            source: crate::tile_wire::Source {
                kind: "surface".into(),
                id: instance.into(),
            },
            rect: crate::tile_wire::Rect {
                x: 0.,
                y: 0.,
                width: 2.,
                height: 2.,
            },
            source_crop: crate::tile_wire::Rect {
                x: 0.,
                y: 0.,
                width: 1.,
                height: 1.,
            },
            z_index: 0,
            interactive: true,
        });
        let mut initial = value(&frame.key, instance);
        initial["generation"] = generation.into();
        let state = Arc::new(
            State::parse(
                &serde_json::to_vec(&initial).unwrap(),
                &(&frame.key).into(),
                instance,
                None,
            )
            .unwrap(),
        );
        let art = Arc::new(Art {
            view: state.view.clone(),
            generation,
            snapshot: state.snapshot.clone(),
            size: state.size,
            images: Default::default(),
        });
        frame.arts.insert(instance.into(), art.clone());
        let job = Job {
            generation: 1,
            key: frame.key.clone(),
            layout: frame.layout.clone(),
            size: frame.size,
        };
        shared.publish(1, frame);
        let frame = shared.lock().frame.clone().unwrap();
        shared.confirm(&frame, Instant::now());
        // 本夹具只模拟已成功的布局ACK；延迟/失败ACK由独立TLS与生命周期测试覆盖。
        {
            let mut s = shared.lock();
            crate::tile_shared::admission::Heartbeat::capture(
                &mut s,
                Some(&job.key),
                Some(job.key.revision),
            )
            .acknowledge(&mut s);
        }
        shared
            .lock()
            .actions
            .observe(state.clone(), "https://localhost", "device", Instant::now());
        Self {
            shared,
            job,
            state,
            art,
            frame,
        }
    }
    pub(crate) fn commands(&self) -> Vec<Command> {
        let p = crate::surface_presentation::Presentation::new(
            crate::surface_scene::Scene::parse(&self.art.snapshot.scene).unwrap(),
            crate::surface_viewport::style(),
            crate::surface_viewport::context([64., 64.]),
            [64., 64.],
        )
        .unwrap();
        crate::tile_surface_click::prepare(&self.frame, &self.art, &p, [0, 0], Default::default())
            .unwrap()
    }
    pub(crate) fn ack(&self, command: &Command, status: &str) -> Ack {
        let Kind::Event(v) = &command.kind else {
            panic!("event");
        };
        Ack::parse_identity(
            &serde_json::to_vec(&ack_value(v["eventId"].as_str().unwrap(), status)).unwrap(),
            "art",
            v["eventId"].as_str().unwrap(),
        )
        .unwrap()
    }
}
#[test]
fn initial_generation_zero_encodes_without_inventing_a_generation_and_successor_retires_it() {
    let f = Fixture::new_generation("art", 0);
    let mut c = f.commands().pop().unwrap();
    let Kind::Event(event) = &c.kind else {
        panic!()
    };
    assert_eq!(event["generation"], 0);
    assert!(crate::tile_surface_action_dispatch::validate(
        &mut c,
        &f.state,
        Some(&f.art),
        &f.job,
        &f.shared
    ));
    let next = Arc::new(
        State::parse(
            &serde_json::to_vec(&value(&f.job.key, "art")).unwrap(),
            &f.state.view.binding,
            "art",
            Some(&f.state),
        )
        .unwrap(),
    );
    f.shared
        .lock()
        .actions
        .attempted(&c, &f.ack(&c, "awaiting_confirmation"), Instant::now());
    assert_eq!(f.shared.lock().actions.accepted.len(), 1);
    f.shared
        .lock()
        .actions
        .observe(next.clone(), "origin", "device", Instant::now());
    assert!(f.shared.lock().actions.accepted.is_empty());
    assert!(!crate::tile_surface_action_dispatch::validate(
        &mut c,
        &next,
        Some(&f.art),
        &f.job,
        &f.shared
    ));
}
#[test]
fn clicks_encode_real_wall_identity_and_fresh_revalidation_preserves_the_event_id() {
    let f = Fixture::new("art");
    let mut commands = f.commands();
    assert_eq!(commands.len(), 1);
    let c = &mut commands[0];
    let Kind::Event(v) = &c.kind else { panic!() };
    assert_eq!(v["attachmentId"], "view");
    assert_eq!(v["payload"], json!({"known":7}));
    assert_eq!(v["generation"], 1);
    assert_eq!(v["event"], "click");
    let id = v["eventId"].clone();
    assert!(crate::tile_surface_action_dispatch::validate(
        c,
        &f.state,
        Some(&f.art),
        &f.job,
        &f.shared
    ));
    let Kind::Event(v) = &c.kind else { panic!() };
    assert_eq!(v["eventId"], id);
    f.shared.lock().frozen = true;
    assert!(!crate::tile_surface_action_dispatch::validate(
        c,
        &f.state,
        Some(&f.art),
        &f.job,
        &f.shared
    ));
}
#[test]
fn queue_is_bounded_by_count_bytes_and_expiry_including_held_cancels() {
    let f = Fixture::new("art");
    let now = Instant::now();
    let mut q = Queue::default();
    q.observe(f.state.clone(), "origin", "device", now);
    for _ in 0..32 {
        assert!(q.offer(f.commands(), now));
    }
    assert!(!q.offer(f.commands(), now));
    assert!(q.take(&f.state, now + Duration::from_secs(2)).is_none());
    assert_eq!(q.notice, "Art操作已过期");
    let mut c = f.commands().pop().unwrap();
    let mut snapshot = crate::tile_surface_wire::Snapshot {
        revision: 1,
        digest: [0; 32],
        scene: vec![b'x'; 8 * 1024 * 1024],
        nodes: 1,
        images: Default::default(),
        dimensions: Default::default(),
    };
    snapshot.images.insert("node".into(), "resource".into());
    c.anchor.snapshot = Arc::new(snapshot);
    assert!(!q.offer(vec![c], now));
    assert_eq!(q.notice, "Art操作内存预算不足");
    let mut c = f.commands().pop().unwrap();
    c.kind = Kind::Cancel {
        request: "request".into(),
        event: "event".into(),
    };
    assert!(q.offer(vec![c], now));
    assert!(q.cancel_for(&f.state.view, now));
    assert!(!q.cancel_for(&f.state.view, now + Duration::from_secs(2)));
    assert!(q
        .take_cancel(&f.state, now + Duration::from_secs(2))
        .is_none());
}
#[test]
fn pending_waits_per_instance_new_failure_clears_only_its_successors_and_unknown_never_replays() {
    let f = Fixture::new("art");
    let other = Fixture::new("other");
    let mut q = Queue::default();
    let now = Instant::now();
    q.observe(f.state.clone(), "origin", "device", now);
    q.observe(other.state.clone(), "origin", "device", now);
    assert!(q.offer(f.commands(), now));
    assert!(q.offer(other.commands(), now));
    let mut v = value(&f.job.key, "art");
    v["pending"] = json!([{"ack":ack_value("event","running"),"actionId":"run","cancelable":true}]);
    let waiting = State::parse(
        &serde_json::to_vec(&v).unwrap(),
        &f.state.view.binding,
        "art",
        Some(&f.state),
    )
    .unwrap();
    assert!(q.take(&waiting, now).is_none());
    assert!(q.take(&other.state, now).is_some());
    v["failure"] = json!({"requestId":"request","code":"wall_surface_action_failed"});
    let failed = Arc::new(
        State::parse(
            &serde_json::to_vec(&v).unwrap(),
            &f.state.view.binding,
            "art",
            Some(&f.state),
        )
        .unwrap(),
    );
    q.observe(failed.clone(), "origin", "device", now);
    assert!(q.take(&f.state, now).is_none());
    assert!(q.offer(f.commands(), now));
    q.observe(failed, "origin", "device", now);
    assert!(q.take(&f.state, now).is_some());
    assert!(q.offer(f.commands(), now));
    q.failed("art");
    assert!(q.take(&f.state, now).is_none());
    assert_eq!(q.notice, "Art操作未确认；未重放");
}
#[test]
fn confirmation_needs_our_ack_complete_prompt_device_and_current_view() {
    let f = Fixture::new("art");
    let c = f.commands().pop().unwrap();
    let ack = f.ack(&c, "awaiting_confirmation");
    let Kind::Event(v) = &c.kind else { panic!() };
    let cv = confirmation(v["eventId"].as_str().unwrap(), &f.job.key.endpoint);
    let mut v = value(&f.job.key, "art");
    v["confirmations"] = json!([cv]);
    let state = Arc::new(
        State::parse(
            &serde_json::to_vec(&v).unwrap(),
            &f.state.view.binding,
            "art",
            Some(&f.state),
        )
        .unwrap(),
    );
    let mut q = Queue::default();
    let now = Instant::now();
    q.observe(state.clone(), "origin", "device", now);
    assert!(q
        .owns_confirmation(&state.activity.confirmations[0], &state, 0)
        .is_none());
    q.attempted(&c, &ack, now);
    assert!(q
        .owns_confirmation(&state.activity.confirmations[0], &state, 0)
        .is_some());
    let mut forged = state.activity.confirmations[0].clone();
    forged.device_id = "other".into();
    assert!(q.owns_confirmation(&forged, &state, 0).is_none());
    q.suspend();
    q.observe(state.clone(), "origin", "device", now);
    assert!(q
        .owns_confirmation(&state.activity.confirmations[0], &state, 0)
        .is_some());
    assert!(!q.offer(
        vec![Command {
            edit: None,
            anchor: Anchor {
                scroll: Default::default(),
                generation: 2,
                ..c.anchor.clone()
            },
            kind: Kind::Cancel {
                request: "r".into(),
                event: "e".into()
            },
            until: now + Duration::from_secs(1)
        }],
        now
    ));
    q.observe(
        state.clone(),
        "origin",
        "device",
        now + Duration::from_secs(121),
    );
    assert!(q.accepted.is_empty());
    q.attempted(&c, &ack, now);
    q.clear();
    assert!(q.accepted.is_empty());
}
#[test]
fn refreshed_control_change_or_old_generation_cannot_retarget_and_held_cancel_needs_exact_pending()
{
    let f = Fixture::new("art");
    let mut c = f.commands().pop().unwrap();
    let mut v = value(&f.job.key, "art");
    v["snapshot"]["revision"] = 2.into();
    v["snapshot"]["scene"]["props"]["eventPayload"] = json!({"changed":true});
    let state = State::parse(
        &serde_json::to_vec(&v).unwrap(),
        &f.state.view.binding,
        "art",
        Some(&f.state),
    )
    .unwrap();
    let art = Arc::new(Art {
        view: state.view.clone(),
        generation: state.generation,
        snapshot: state.snapshot.clone(),
        size: state.size,
        images: Default::default(),
    });
    assert!(!crate::tile_surface_action_dispatch::validate(
        &mut c,
        &state,
        Some(&art),
        &f.job,
        &f.shared
    ));
    let mut v = value(&f.job.key, "art");
    v["pending"] = json!([{"ack":ack_value("event","running"),"actionId":"run","cancelable":true}]);
    let state = State::parse(
        &serde_json::to_vec(&v).unwrap(),
        &f.state.view.binding,
        "art",
        Some(&f.state),
    )
    .unwrap();
    c.kind = Kind::Cancel {
        request: "request".into(),
        event: "event".into(),
    };
    f.shared.lock().frozen = true;
    assert!(crate::tile_surface_action_dispatch::validate(
        &mut c, &state, None, &f.job, &f.shared
    ));
    c.kind = Kind::Cancel {
        request: "successor".into(),
        event: "event".into(),
    };
    assert!(!crate::tile_surface_action_dispatch::validate(
        &mut c, &state, None, &f.job, &f.shared
    ));
    c.anchor.generation = 2;
    assert!(!crate::tile_surface_action_dispatch::validate(
        &mut c, &state, None, &f.job, &f.shared
    ));
}

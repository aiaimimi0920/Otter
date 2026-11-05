//! 配方缓存身份、LRU 与跨线程字节租约；不连接服务或加载 ANGLE。
use super::super::{capability_selection::Selection, shader_prepare::tests as staging};
use super::*;
use crate::{desktop::loom_execution_job::Execution, editor::Editor, shader_angle::Backend};
use serde_json::json;
use std::{sync::atomic::AtomicBool, time::Duration};
#[path = "shader_recipe_platform_tests.rs"]
mod platform;
fn fixture_state() -> State {
    let state = State::new(None, false).unwrap();
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let mut raw = crate::loom_handshake::tests::fixture();
    raw["capabilities"]["artDefinitions"][0]["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"gain","default":0.25}));
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&raw).unwrap())
        .unwrap()
        .unwrap();
    let parameters =
        crate::loom_parameter_state::Parameters::new(&handshake.capability(0).unwrap()).unwrap();
    *state.selected_art.borrow_mut() = Some(Selection {
        identity: staging::id("selection"),
        execution: Default::default(),
        parameters,
        catalog: Rc::new(Loaded {
            identity,
            handshake,
        }),
    });
    state
}
fn payload() -> Arc<Payload> {
    Arc::new(Payload::from_batch(staging::batch(staging::payload())).unwrap())
}
fn execution(state: &State) -> Execution {
    let (_, identity, _) = crate::loom_bridge_client::tests::fixture();
    Execution {
        shader: Some(staging::runtime(Backend::D3d11, false)),
        identity,
        unit: state.unit_id.clone(),
        request: crate::graph_requests::Request {
            id: staging::id("request"),
            generation: 1,
        },
        message: String::new(),
        inputs: super::super::loom_input_job::PreparedBatch::empty_for_test(
            state.unit_id.clone(),
            &Editor::default(),
        ),
        timeout: Duration::from_secs(1),
    }
}
#[test]
fn shader_recipe_cache_lru_hit_protects_recent_entry_and_disabled_or_closed_releases_all() {
    let mut cache = Cache::default();
    let states: Vec<_> = (0..17)
        .map(|index| {
            let mut state = fixture_state();
            state.unit_id = staging::id(&format!("unit-{index}"));
            state
        })
        .collect();
    for state in &states[..16] {
        cache.insert(
            Key::capture(state, Context::Independent).unwrap(),
            payload(),
        );
    }
    assert_eq!(cache.entries.len(), 16);
    let mut job = execution(&states[0]);
    cache.bind(&states[0], &mut job);
    assert!(job.shader.as_ref().unwrap().is_cached());
    drop(job);
    cache.insert(
        Key::capture(&states[16], Context::Independent).unwrap(),
        payload(),
    );
    assert_eq!(cache.entries.len(), 16);
    for (index, expected) in [(0, true), (1, false), (16, true)] {
        let mut job = execution(&states[index]);
        cache.bind(&states[index], &mut job);
        assert_eq!(job.shader.as_ref().unwrap().is_cached(), expected);
    }
    cache.retain(&[], true);
    assert!(cache.entries.is_empty());
    assert_eq!(cache.budget.used(), 0);
    cache.insert(
        Key::capture(&states[0], Context::Independent).unwrap(),
        payload(),
    );
    cache.retain(&[], false);
    assert_eq!(cache.budget.used(), 0);
}
#[test]
fn shader_recipe_cached_preparation_rebinds_current_input_and_restores_response_defaults_and_lut() {
    let _serial = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let state = fixture_state();
    let inputs = staging::prepared(&Editor::default(), 1, 1, &[9, 17, 33, 255]);
    let lut = [253, 137, 79, 1];
    let mut raw = staging::payload();
    raw["textures"]["lut"] = json!(format!(
        "data:image/png;base64,{}",
        staging::encoded(&staging::png(1, 1, &lut))
    ));
    let (first, acquired) = staging::runtime(Backend::D3d11, false)
        .acquire(staging::batch(raw), &inputs, &AtomicBool::new(false))
        .unwrap();
    let super::super::shader_invocation::Invocation::Legacy(first) = first else {
        panic!("legacy recipe changed backend")
    };
    assert_eq!(first.uniforms, vec![("gain".into(), 1.0)]);
    let acquired = acquired.unwrap();
    let mut cache = Cache::default();
    cache.insert(
        Key::capture(&state, acquired.context).unwrap(),
        acquired.payload,
    );
    let mut job = execution(&state);
    job.inputs = staging::prepared(&Editor::default(), 1, 1, &[47, 59, 71, 128]);
    job.shader = Some(
        super::super::shader_prepare::Runtime::new(
            staging::runtime(Backend::D3d11, false).bundle,
            Backend::D3d11,
            "input_path".into(),
            Vec::new(),
        )
        .unwrap(),
    );
    cache.bind(&state, &mut job);
    let runtime = job.shader.as_ref().unwrap();
    assert!(
        runtime.is_cached(),
        "independent recipes do not depend on input content"
    );
    let next = runtime
        .prepare_cached(&job.inputs, &AtomicBool::new(false))
        .unwrap();
    let super::super::shader_invocation::Invocation::Legacy(next) = next else {
        panic!("legacy recipe changed backend")
    };
    assert_eq!(next.uniforms, vec![("gain".into(), 0.25)]);
    assert_eq!(&*next.textures[0].rgba, &[47, 59, 71, 128]);
    assert_eq!(&*next.textures[1].rgba, &lut);
    assert_eq!(
        runtime.cached.as_ref().unwrap().payload.uniforms["gain"],
        0.25
    );
    assert!(matches!(
        runtime.prepare_cached(&job.inputs, &AtomicBool::new(true)),
        Err(super::super::shader_payload::Error::Cancelled)
    ));
}
#[test]
fn shader_recipe_context_changes_miss_and_unfingerprinted_inputs_never_become_independent() {
    let _serial = crate::desktop::loom_input_job::TEST_SERIAL.lock().unwrap();
    let state = fixture_state();
    let raw = json!({"id":"shader","inputs":[{"name":"input_path","type":"image"},{"name":"reference_path","widget":"image_link"}]});
    let capability = crate::loom_capability::Capability::new(&raw).unwrap();
    let parameters = crate::graph_params::Params::new(&[], &[]).unwrap();
    let installation = super::super::shader_prepare::Installation {
        bundle: staging::runtime(Backend::D3d11, false).bundle,
        backend: Backend::D3d11,
    };
    let runtime = || {
        installation
            .capture(
                &capability,
                &["input_path", "reference_path"],
                &parameters,
                &[],
            )
            .unwrap()
    };
    let prepared = |input: u8, reference: u8| {
        let mut sources: Vec<_> = [("input_path", input), ("reference_path", reference)]
            .into_iter()
            .map(|(port, channel)| {
                super::super::loom_input_job::Source::capture(
                    port.into(),
                    staging::id(port),
                    1,
                    1,
                    vec![channel, 2, 3, 255].into(),
                    &Editor::default(),
                )
                .unwrap()
            })
            .collect();
        runtime().mark_context(&mut sources);
        staging::prepare_sources(&Editor::default(), sources)
    };
    let original = prepared(7, 11);
    let context = runtime().context(&original).unwrap();
    let mut cache = Cache::default();
    cache.insert(Key::capture(&state, context).unwrap(), payload());
    for (input, reference, expected) in
        [(7, 11, true), (8, 11, false), (7, 12, false), (7, 11, true)]
    {
        let mut job = execution(&state);
        job.shader = Some(runtime());
        job.inputs = prepared(input, reference);
        cache.bind(&state, &mut job);
        assert_eq!(job.shader.as_ref().unwrap().is_cached(), expected);
    }
    let mut job = execution(&state);
    job.shader = Some(runtime());
    job.inputs = staging::prepared(&Editor::default(), 1, 1, &[7, 2, 3, 255]);
    cache.insert(
        Key::capture(&state, Context::Independent).unwrap(),
        payload(),
    );
    assert!(job.shader.as_ref().unwrap().context(&job.inputs).is_none());
    cache.bind(&state, &mut job);
    assert!(!job.shader.as_ref().unwrap().is_cached());
}
#[test]
fn shader_recipe_cache_keys_unit_catalog_selection_and_exact_context_not_parameter_revision() {
    let state = fixture_state();
    let key = Key::capture(&state, Context::Independent).unwrap();
    state
        .selected_art
        .borrow_mut()
        .as_mut()
        .unwrap()
        .parameters
        .set("gain", Some(json!(9)))
        .unwrap();
    assert!(key.matches(&state));
    assert!(key.same(&Key::capture(&state, Context::Independent).unwrap()));
    assert!(!key.same(
        &Key::capture(
            &state,
            Context::Referenced {
                input: [1; 32],
                reference: [2; 32]
            }
        )
        .unwrap()
    ));
    let mut other = fixture_state();
    other.unit_id = state.unit_id.clone();
    assert!(!key.matches(&other));
    state.selected_art.borrow_mut().as_mut().unwrap().identity = staging::id("new-selection");
    assert!(!key.matches(&state));
}
#[test]
fn shader_recipe_cache_eviction_keeps_running_recipe_lease_and_raw_defaults() {
    let state = fixture_state();
    let mut cache = Cache::default();
    let raw = payload();
    let bytes = resident_bytes(&raw).unwrap();
    cache.budget = Budget::new(bytes);
    cache.insert(
        Key::capture(&state, Context::Independent).unwrap(),
        raw.clone(),
    );
    assert_eq!(cache.entries.len(), 1);
    let running = cache.entries[0].recipe.clone();
    cache.insert(
        Key::capture(
            &state,
            Context::Referenced {
                input: [3; 32],
                reference: [4; 32],
            },
        )
        .unwrap(),
        payload(),
    );
    assert!(cache.entries.is_empty());
    assert_eq!(cache.budget.used(), bytes);
    assert_eq!(running.payload.uniforms["gain"], 0.25);
    drop(running);
    assert_eq!(cache.budget.used(), 0);
    for index in 0..20 {
        cache.insert(
            Key::capture(
                &state,
                Context::Referenced {
                    input: [index; 32],
                    reference: [4; 32],
                },
            )
            .unwrap(),
            payload(),
        );
    }
    assert_eq!(cache.entries.len(), 1);
    assert_eq!(
        cache.entries[0].key.context,
        Context::Referenced {
            input: [19; 32],
            reference: [4; 32]
        }
    );
}

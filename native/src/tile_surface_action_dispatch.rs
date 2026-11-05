//! 新鲜state后复核同view/代际/命中/控件合同；同一个Art poll线程提交，不阻塞Live媒体。
use crate::{
    tile_assets::Job,
    tile_shared::Shared,
    tile_surface_actions::{Command, Kind},
    tile_surface_owner::Art,
    tile_surface_wire::State,
};
use std::{sync::Arc, time::Instant};
pub(crate) fn current(s: &crate::tile_shared::State, job: &Job) -> bool {
    !s.stopped
        && !s.expired
        && s.desired.as_ref() == Some(&job.key)
        && s.generation == job.generation
}
pub(crate) fn ready(s: &crate::tile_shared::State, state: &State, job: &Job, now: Instant) -> bool {
    current(s, job)
        && crate::tile_shared::admission::permits(s, &job.key)
        && !s.frozen
        && s.visible(now)
        && s.presented.as_ref().is_some_and(|p| p.0 == job.key)
        && s.presented_frame
            .as_ref()
            .and_then(|f| f.arts.get(&state.view.instance_id))
            .is_some_and(|a| {
                a.view == state.view
                    && a.generation == state.generation
                    && Arc::ptr_eq(&a.snapshot, &state.snapshot)
            })
}
/// 写前最后线性化围栏；不得跨昂贵布局准备复用早先的可见性或队列身份。
pub(crate) fn eligible(
    command: &Command,
    state: &State,
    job: &Job,
    s: &crate::tile_shared::State,
) -> bool {
    let now = Instant::now();
    let a = &command.anchor;
    if now >= command.until
        || state.view != a.view
        || state.generation != a.generation
        || !current(s, job)
    {
        return false;
    }
    if let Kind::Cancel { request, event } = &command.kind {
        return state
            .activity
            .pending
            .iter()
            .any(|p| p.cancelable && p.ack.request_id() == request && p.event == *event);
    }
    ready(s, state, job, now)
        && command.edit.as_ref().is_none_or(|e| e.active())
        && match &command.kind {
            Kind::Decision {
                confirmation: c, ..
            } => {
                s.actions
                    .owns_confirmation(c, state, crate::tile_runtime::now_ms())
                    .is_some()
                    && state
                        .activity
                        .confirmations
                        .iter()
                        .any(|current| current == c.as_ref())
            }
            Kind::Event(_) => true,
            Kind::Cancel { .. } => unreachable!(),
        }
}
fn node<'a>(v: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    if v["id"] == id {
        return Some(v);
    }
    v["children"].as_array()?.iter().find_map(|n| node(n, id))
}

#[cfg(test)]
pub(crate) fn trace(
    command: &Command,
    state: &State,
    art: Option<&Arc<Art>>,
    shared: &Shared,
    stage: &str,
) {
    let a = &command.anchor;
    let (event, node_same) = if let Kind::Event(event) = &command.kind {
        let before: serde_json::Value = serde_json::from_slice(&a.snapshot.scene).unwrap();
        let after: serde_json::Value = serde_json::from_slice(&state.snapshot.scene).unwrap();
        let id = event["nodeId"].as_str().unwrap();
        (
            event.clone(),
            node(&before, id).is_some() && node(&before, id) == node(&after, id),
        )
    } else {
        (serde_json::Value::Null, false)
    };
    let mut s = shared.lock();
    s.actions.test_dispatch = Some(
        serde_json::json!({"stage":stage,"event":event,"expired":Instant::now() >= command.until,
        "sameView":a.view == state.view,"sameGeneration":a.generation == state.generation,"nodeSame":node_same,
        "anchorRevision":a.snapshot.revision,"stateRevision":state.snapshot.revision,"stateSequence":state.sequence,
        "artMatchesState":art.is_some_and(|art|Arc::ptr_eq(&art.snapshot,&state.snapshot)),
        "visible":s.visible(Instant::now()),"frozen":s.frozen,"presentedRevision":s.presented.as_ref().map(|p|p.0.revision),
        "serviceLayoutAcknowledged":s.desired.as_ref().is_some_and(|key|crate::tile_shared::admission::permits(&s,key))}),
    );
}
pub(crate) fn validate(
    command: &mut Command,
    state: &State,
    art: Option<&Arc<Art>>,
    job: &Job,
    shared: &Shared,
) -> bool {
    let a = &command.anchor;
    let s = shared.lock();
    if !eligible(command, state, job, &s) {
        return false;
    }
    if matches!(command.kind, Kind::Cancel { .. }) {
        return true;
    }
    let Some((placement, uv)) =
        crate::tile_surface_hit::hit(&job.layout, &job.key.endpoint, job.size, a.pixel)
    else {
        return false;
    };
    if placement.placement_id != a.placement || placement.source.id != a.view.instance_id {
        return false;
    }
    match &mut command.kind {
        Kind::Decision { .. } => true,
        Kind::Event(event) => {
            let Some(art) = art.filter(|art| Arc::ptr_eq(&art.snapshot, &state.snapshot)) else {
                return false;
            };
            let Ok(before) = serde_json::from_slice::<serde_json::Value>(&a.snapshot.scene) else {
                return false;
            };
            let Ok(after) = serde_json::from_slice::<serde_json::Value>(&state.snapshot.scene)
            else {
                return false;
            };
            let Some(id) = event["nodeId"].as_str() else {
                return false;
            };
            let compatible = node(&before, id)
                .zip(node(&after, id))
                .is_some_and(|(a, b)| {
                    command
                        .edit
                        .as_ref()
                        .map_or(a == b, |e| e.same_contract(a, b))
                });
            if !compatible {
                return false;
            }
            drop(s);
            // 仅在用户排入动作时做最小新鲜布局；COM/Pixels仍留在此唯一worker内，不跨线程移交。
            let Ok(scene) = crate::surface_scene::Scene::parse(&state.snapshot.scene) else {
                return false;
            };
            let mut images = crate::surface_presentation::Images::new();
            for (id, image) in &art.images {
                let Ok(bitmap) = crate::surface_image_paint::Bitmap::new(image.clone()) else {
                    return false;
                };
                images.insert(id.clone(), bitmap);
            }
            let size = state.size.map(|v| v as f32);
            let Ok(mut p) = crate::surface_presentation::Presentation::with_images(
                scene,
                crate::surface_viewport::style(),
                crate::surface_viewport::context(size),
                size,
                images,
            ) else {
                return false;
            };
            // 坐标必须对应用户真实paint时的本地滚动；不把滚轮值或网络scene当作偏移证据。
            if p.restore_scroll(&a.scroll).is_err() {
                return false;
            }
            let point = [uv[0] * size[0], uv[1] * size[1]];
            let valid = if let Some(edit) = &command.edit {
                crate::tile_surface_edit::validates(&p, point, event, edit)
            } else if event["event"] == "click" {
                p.click_route(point)
                    .any(|a| a.node.id() == id && event["action"] == a.action)
            } else {
                crate::tile_surface_value::validates(&p, point, event)
            };
            if !valid {
                return false;
            }
            event["baseRevision"] = state.snapshot.revision.into();
            true
        }
        Kind::Cancel { .. } => unreachable!(),
    }
}

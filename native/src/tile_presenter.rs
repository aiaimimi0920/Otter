//! 一次未知写不重放；断线丢弃本地租约，读状态/CAS 后显式建立新的 presenter。
use super::*;
use crate::{
    device_session_client::Client,
    tile_assets::Job,
    tile_shared::Key,
    tile_wire::{self as wire, Endpoint, Mode, PresentationReport, SceneReport},
};
use std::time::Instant;
struct Presenter {
    endpoint: String,
    lease: String,
    sequence: u64,
    ttl: Duration,
}
pub(super) fn run(
    client: &Client,
    token: &Lease,
    shared: &Arc<Shared>,
    assets: &Assets,
    output: TileOutput,
) -> Result<(), &'static str> {
    let endpoint = Endpoint::image(token.device_id(), &output);
    let mut presenter: Option<Presenter> = None;
    loop {
        if shared.lock().stopped {
            break;
        }
        if token.validate(client.origin(), now_ms()).is_err() {
            break;
        }
        let mut step = || -> Result<Duration, &'static str> {
            let control = token.control(client).map_err(|_| "设备授权失效")?;
            let sent = Instant::now();
            let mut state = control
                .wall_state()
                .map_err(|e| failure(token, e, "墙面状态读取失败"))?;
            let received = Instant::now();
            {
                let mut s = shared.lock();
                let changed = state
                    .timing
                    .as_ref()
                    .is_some_and(|t| s.clock.observe(t, sent, received));
                if changed {
                    s.clear("服务时钟已改变");
                }
            }
            if shared.lock().stopped {
                return Ok(Duration::ZERO);
            }
            if state
                .endpoints
                .iter()
                .find(|e| e.endpoint.endpoint_id == endpoint.endpoint_id)
                .map(|e| &e.endpoint)
                != Some(&endpoint)
            {
                disconnect(&control, &mut presenter);
                state = control
                    .wall_register(state.revision, &endpoint)
                    .map_err(|e| failure(token, e, "输出注册失败"))?;
            }
            if presenter.is_none() {
                let lease = control.wall_connect(&endpoint.endpoint_id).map_err(|e| {
                    // 丢失 connect 回复时不知道 leaseId；不得自动再次创建未知租约。
                    if matches!(e, crate::live_relay_http::Error::Http(_)) {
                        shared.stop();
                    }
                    failure(token, e, "呈现租约申请失败")
                })?;
                presenter = Some(Presenter {
                    endpoint: endpoint.endpoint_id.clone(),
                    lease: lease.lease_id,
                    sequence: 0,
                    ttl: Duration::from_millis(lease.lease_ttl_ms),
                });
            }
            let p = presenter.as_mut().unwrap();
            if shared.lock().stopped {
                return Ok(Duration::ZERO);
            }
            let layout = state.layouts.iter().find(|l| {
                l.tiles
                    .iter()
                    .any(|t| t.endpoint_id == endpoint.endpoint_id)
            });
            let presentation = state
                .presentations
                .iter()
                .find(|c| layout.is_some_and(|l| l.wall_id == c.wall_id));
            let scene = state.timing.as_ref().and_then(|t| {
                t.scenes
                    .iter()
                    .find(|s| layout.is_some_and(|l| l.wall_id == s.wall_id))
            });
            let key = layout.map(|l| Key {
                endpoint: endpoint.endpoint_id.clone(),
                lease: p.lease.clone(),
                wall: l.wall_id.clone(),
                revision: l.revision,
                clock: state
                    .timing
                    .as_ref()
                    .map(|t| t.clock_id.clone())
                    .unwrap_or_default(),
            });
            let (applied, report, scene_report, job, identification_report, identifying, heartbeat) = {
                let mut s = shared.lock();
                if s.desired != key {
                    s.clear("正在准备墙面");
                    s.desired = key.clone();
                }
                s.expire(Instant::now());
                let command = state
                    .endpoints
                    .iter()
                    .find(|e| e.endpoint.endpoint_id == endpoint.endpoint_id)
                    .and_then(|e| e.identification.as_ref());
                s.identification
                    .observe(command, &endpoint, &p.lease, sent, Instant::now());
                let identifying = s.identification.blocked();
                if identifying {
                    s.presented = None;
                    s.presented_frame = None;
                    s.status = "正在识别物理输出";
                }
                // 没有新鲜控制心跳，独立窗口 watchdog 必须清屏，不能由下载延长许可。
                if s.until.is_none() {
                    s.until = Some(deadline(token, p.ttl));
                }
                s.activate_at = scene.map_or(0, |v| v.activate_at_ms);
                // 已广告 scheduledPresentation，缺失权威时钟必须拒绝而非立即提前显示。
                if state.timing.is_none() {
                    return Err("服务未提供 Wall 场景时钟");
                }
                let retained = s.presented_frame.as_ref().is_some_and(|r| {
                    Some(&r.key) == key.as_ref()
                        && (s.frozen || r.live_until.is_none_or(|v| Instant::now() <= v))
                });
                let black = presentation.is_some_and(|c| c.mode == Mode::Black);
                let frozen = presentation.is_some_and(|c| c.mode == Mode::Frozen);
                s.frozen = frozen;
                s.black = black || key.is_none() || (frozen && !retained);
                if identifying || s.frozen || s.black {
                    s.input_applied = None;
                    s.actions.suspend();
                }
                if frozen && retained {
                    s.frame = s.presented_frame.clone();
                }
                s.black_revision = presentation.filter(|_| black).map(|c| c.revision);
                if s.black_confirmed != s.black_revision {
                    s.black_confirmed = None;
                }
                if s.black {
                    s.frame = None;
                    s.presented = None;
                    s.presented_frame = None;
                }
                let applied = (!s.black && !identifying)
                    .then(|| {
                        s.presented
                            .as_ref()
                            .filter(|r| Some(&r.0) == key.as_ref())
                            .map(|r| r.0.revision)
                    })
                    .flatten();
                let report = presentation
                    .filter(|c| !black || s.black_confirmed == Some(c.revision))
                    .map(|c| PresentationReport {
                        revision: c.revision,
                        outcome: if frozen && !retained {
                            "frame_unavailable"
                        } else {
                            "applied"
                        }
                        .into(),
                    });
                let scene_report = scene
                    .filter(|_| presentation.is_none() && !identifying)
                    .map(|v| SceneReport {
                        revision: v.revision,
                        prepared: s.frame.is_some(),
                        applied_at_ms: s.presented.as_ref().map(|r| r.1),
                        clock_uncertainty_ms: s.clock.estimate(Instant::now()).map(|v| v.1),
                    });
                let job = if !s.black && !s.frozen && !identifying {
                    layout.zip(key.clone()).map(|(l, key)| Job {
                        generation: s.generation,
                        key,
                        layout: l.clone(),
                        size: endpoint.pixel_size,
                    })
                } else {
                    None
                };
                let identification_report =
                    s.identification
                        .take_report(&p.endpoint, &p.lease, Instant::now());
                (
                    applied,
                    report,
                    scene_report,
                    job,
                    identification_report,
                    identifying,
                    crate::tile_shared::admission::Heartbeat::capture(
                        &mut s,
                        key.as_ref(),
                        applied,
                    ),
                )
            };
            if let Some(job) = job {
                assets.submit(job);
            }
            if let Some(report) = identification_report {
                // 一次发送，不因未知报告结果重放；原错误链撤销此presenter并清屏。
                control
                    .wall_identification_report(&report)
                    .map_err(|e| failure(token, e, "输出识别回报失败"))?;
            }
            shared.trace.observe(if applied.is_some() {
                crate::tile_shared::diagnostics::Stage::HeartbeatPreparedWithApplied
            } else {
                crate::tile_shared::diagnostics::Stage::HeartbeatPreparedWithoutApplied
            });
            p.sequence = p
                .sequence
                .checked_add(1)
                .filter(|v| *v <= wire::MAX_SAFE)
                .ok_or("呈现序号已耗尽")?;
            control
                .wall_heartbeat(
                    &p.endpoint,
                    &p.lease,
                    p.sequence,
                    applied,
                    report.as_ref(),
                    scene_report.as_ref(),
                )
                .map_err(|e| failure(token, e, "呈现心跳失败"))?;
            let mut s = shared.lock();
            if !s.stopped {
                s.until = Some(deadline(token, p.ttl));
                s.expired = false;
                s.identification.finish_cycle();
                heartbeat.acknowledge(&mut s);
            }
            Ok(
                if identifying || (layout.is_some() && applied.is_none() && presentation.is_none())
                {
                    Duration::from_millis(250)
                } else {
                    Duration::from_secs(3).min(p.ttl / 3)
                },
            )
        };
        let delay = match step() {
            Ok(delay) => delay,
            Err(reason) => {
                shared.lock().clear(reason);
                if let Ok(control) = token.control(client) {
                    disconnect(&control, &mut presenter);
                } else {
                    presenter = None;
                }
                Duration::from_secs(2)
            }
        };
        if !shared.wait(delay) {
            break;
        }
    }
    if let Ok(control) = token.control(client) {
        disconnect(&control, &mut presenter);
    }
    shared.lock().clear("已停止");
    Ok(())
}
fn disconnect(control: &crate::live_relay_http::Control<'_>, p: &mut Option<Presenter>) {
    if let Some(p) = p.take() {
        let _ = control.wall_disconnect(&p.endpoint, &p.lease);
    }
}
fn deadline(token: &Lease, ttl: Duration) -> Instant {
    Instant::now()
        + ttl.min(Duration::from_secs(6)).min(Duration::from_millis(
            token.expires_at_ms().saturating_sub(now_ms()),
        ))
}
fn failure(
    token: &Lease,
    error: crate::live_relay_http::Error,
    reason: &'static str,
) -> &'static str {
    use crate::live_relay_http::Error;
    if matches!(
        error,
        Error::AuthorizationRequired
            | Error::Credential
            | Error::Input
            | Error::Status(401 | 403)
            | Error::Http(
                crate::device_session_client::Error::Trust
                    | crate::device_session_client::Error::Tls
                    | crate::device_session_client::Error::Limit
            )
    ) {
        token.revoke();
    }
    reason
}

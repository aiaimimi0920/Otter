//! 后台账号调用与候选准备；Loom 的持久 Create 按 actor/Unit 幂等，Accept 先记录精确接收身份。
use super::*;
use crate::device_authorization_lease::Lease;
pub(super) struct SourceInput {
    pub source: Source,
    pub unit: UnitId,
    pub scope: crate::session_projection_scope::Scope,
    pub snapshot: projection_sender::scope::Snapshot,
}
pub(super) struct ReceiveInput {
    pub source: Source,
    pub space: crate::session_placement::Space,
    pub scope: crate::session_projection_scope::Scope,
    pub units: Vec<UnitId>,
    pub root: std::path::PathBuf,
    pub envelope: account::Envelope,
    pub recovery: Option<account::journal::Intent>,
}
pub(super) fn image(input: &SourceInput, lease: &Lease) -> Result<account::Image> {
    let snapshot = &input.snapshot;
    let frame = crate::desktop::export::projection::target_frame_with_input(
        &input.source,
        input.unit.as_str(),
        &snapshot.image,
        &snapshot.editor,
        crate::desktop::export::projection::Presentation {
            opacity: snapshot.opacity,
            kind: snapshot.kind,
            geometry: snapshot.geometry,
        },
        snapshot.render_input(),
        lease.preparation_cancel(),
    )?;
    account::Image::from_png(
        &frame.png,
        frame.width,
        frame.height,
        lease.preparation_cancel(),
    )
    .map_err(job::failure)
}
pub(super) fn create(
    lease: &Lease,
    discovery: &crate::loom_discovery::Config,
    origin: &str,
    input: SourceInput,
) -> Result<Output> {
    let account = lease.account_projections(discovery).map_err(job::failure)?;
    let image = image(&input, lease)?;
    let content = job::content(
        account
            .execute(account::Operation::Create {
                unit: input.unit.as_str(),
                kind: match input.snapshot.kind {
                    crate::projection_delivery::sender::Kind::Art => "art",
                    _ => "sticker",
                },
                image: &image,
            })
            .map_err(job::failure)?,
    )?;
    let binding =
        Binding::sent(&content, account.context(), origin, input.scope).map_err(job::failure)?;
    let previous = account::scene::Index::capture(&input.source)
        .map_err(job::failure)?
        .bindings()
        .iter()
        .find(|b| b.unit() == input.unit.as_str())
        .cloned();
    // 同 Unit 的未停止关联不能被换邀请；已经停止的记录由显式新分享覆盖。
    let source = binding
        .replace(&input.source, previous.as_ref(), lease.preparation_cancel())
        .map_err(job::failure)?;
    Ok(Output::Patch {
        source,
        binding: Box::new(binding),
        image: None,
    })
}
pub(super) fn receive(
    lease: &Lease,
    discovery: &crate::loom_discovery::Config,
    origin: &str,
    input: ReceiveInput,
) -> Result<Output> {
    let account = lease.account_projections(discovery).map_err(job::failure)?;
    let cancel = lease.preparation_cancel();
    let mut journal = account::journal::Store::open(&input.root, cancel).map_err(job::failure)?;
    let existing = journal
        .entries()
        .iter()
        .find(|e| e.envelope == input.envelope)
        .cloned();
    let intent = if let Some(intent) = existing {
        if intent.scope != input.scope
            || intent.loom_origin != origin
            || intent.device != account.context().device_id
        {
            return Err(stale());
        }
        intent
    } else {
        account::journal::Intent {
            envelope: input.envelope.clone(),
            unit: format!("account-{:?}", unsafe {
                windows::Win32::System::Com::CoCreateGuid()?
            }),
            scope: input.scope,
            loom_origin: origin.into(),
            device: account.context().device_id.clone(),
        }
    };
    if input.units.iter().any(|unit| unit.as_str() == intent.unit) {
        return Err(stale());
    }
    let preview = if let Some(expected) = &input.recovery {
        if expected != &intent {
            return Err(stale());
        }
        // 只有权威 Read 已确认原接受存在才重取图像；未知接受绝不自动重放。
        job::content(
            account
                .execute(account::Operation::Read {
                    envelope: &intent.envelope,
                    known_revision: 1,
                    known_digest: intent.envelope.digest(),
                    receiver_unit: Some(&intent.unit),
                })
                .map_err(job::failure)?,
        )?
    } else {
        job::content(
            account
                .execute(account::Operation::Inspect {
                    envelope: &input.envelope,
                })
                .map_err(job::failure)?,
        )?
    };
    journal
        .begin(intent.clone(), cancel)
        .map_err(job::failure)?;
    let content = job::content(
        account
            .execute(account::Operation::Accept {
                envelope: &input.envelope,
                unit: &intent.unit,
                revision: preview.revision,
                digest: &preview.digest,
                confirmed: true,
            })
            .map_err(job::failure)?,
    )?;
    let unit = UnitId::parse(&intent.unit).ok_or_else(stale)?;
    let binding = Binding::received(&content, account.context(), origin, &unit, input.scope)
        .map_err(job::failure)?;
    let image = content.image.as_ref().ok_or_else(stale)?;
    let scale = (640.0 / f64::from(image.width()))
        .min(480.0 / f64::from(image.height()))
        .min(1.0);
    let rect = crate::session_window::Rect {
        x: 40.0,
        y: 40.0,
        w: f64::from(image.width()) * scale,
        h: f64::from(image.height()) * scale,
    };
    let (restore, retained) = crate::desktop::session_candidate::account_projection::prepare(
        *content,
        &binding,
        input.space,
        rect,
        crate::session_append::Target {
            original: &input.source,
            active: &input.units,
        },
        cancel,
    )?;
    Ok(Output::Receive(Box::new(account_projection::Prepared {
        restore,
        retained,
        binding,
    })))
}

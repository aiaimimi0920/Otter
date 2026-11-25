//! Correlated replies must match account/original invitation/revision/receiver Unit; decode PNG independently and distrust returned routes.
use super::*;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Wire<'a> {
    envelope: Envelope,
    revision: u64,
    digest: String,
    linked: bool,
    receiver_device_id: Option<String>,
    receiver_unit_id: Option<String>,
    #[serde(borrow)]
    snapshot: Option<crate::projection_delivery::sender::snapshot::Wire<'a>>,
    transport: String,
    error: Option<String>,
}
pub enum State {
    Unlinked,
    Content(Box<Content>),
}
pub struct Content {
    pub envelope: Envelope,
    pub revision: u64,
    pub digest: String,
    /// Loom linked means not stopped, not receiver acceptance or actual display.
    pub linked: bool,
    pub receiver_device_id: Option<String>,
    pub receiver_unit_id: Option<String>,
    pub image: Option<Image>,
    pub transport: String,
    pub error: Option<String>,
}
pub(super) fn parse(
    bytes: &[u8],
    operation: &Operation<'_>,
    context: &Context,
    cancel: &AtomicBool,
) -> Result<State, Error> {
    if bytes.len() > MAX_WIRE {
        return Err(Error::Response);
    }
    if matches!(operation, Operation::Unlink { .. }) {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Unlink {
            unlinked: bool,
        }
        let reply: Unlink = serde_json::from_slice(bytes).map_err(|_| Error::Response)?;
        return if reply.unlinked {
            Ok(State::Unlinked)
        } else {
            Err(Error::Response)
        };
    }
    let wire: Wire<'_> = serde_json::from_slice(bytes).map_err(|_| Error::Response)?;
    wire.envelope.validate()?;
    if !wire.envelope.matches_origin(context)
        || !revision(wire.revision)
        || !digest(&wire.digest)
        || (wire.revision == 1 && wire.digest != wire.envelope.digest())
        || operation
            .envelope()
            .is_some_and(|expected| expected != &wire.envelope)
        || wire.receiver_device_id.is_some() != wire.receiver_unit_id.is_some()
        || wire
            .receiver_device_id
            .as_deref()
            .is_some_and(|id| !envelope::device(id))
        || wire
            .receiver_unit_id
            .as_deref()
            .is_some_and(|id| !identifier(id))
        || !matches!(
            wire.transport.as_str(),
            "direct" | "relay" | "reconnecting" | "offline"
        )
        || wire
            .error
            .as_deref()
            .is_some_and(|code| !identifier(code) || wire.transport != "offline")
    {
        return Err(Error::Response);
    }
    let valid = match operation {
        Operation::Create { unit, kind, .. } => {
            wire.envelope.matches_source(context)
                && wire.envelope.source_unit() == *unit
                && wire.envelope.kind() == *kind
        }
        Operation::Inspect { .. } => {
            wire.envelope.source_device() != context.device_id
                && wire.snapshot.is_some()
                && wire
                    .receiver_device_id
                    .as_deref()
                    .is_none_or(|id| id == context.device_id)
        }
        Operation::Accept {
            unit,
            revision,
            digest,
            ..
        } => {
            wire.envelope.source_device() != context.device_id
                && wire.receiver_device_id.as_deref() == Some(&context.device_id)
                && wire.receiver_unit_id.as_deref() == Some(*unit)
                && wire.revision >= *revision
                && (wire.revision != *revision || wire.digest == *digest)
                && wire.snapshot.is_some()
        }
        Operation::Update {
            prior_revision,
            revision,
            image,
            ..
        } => {
            wire.envelope.matches_source(context)
                && matches!(wire.revision, n if n == *revision || n == *prior_revision)
                && wire.digest == image.digest()
        }
        Operation::Read {
            known_revision,
            known_digest,
            receiver_unit,
            ..
        } => {
            wire.revision >= *known_revision
                && (wire.revision != *known_revision || wire.digest == *known_digest)
                && receiver_unit.map_or(wire.envelope.matches_source(context), |unit| {
                    wire.envelope.source_device() != context.device_id
                        && wire.receiver_device_id.as_deref() == Some(&context.device_id)
                        && wire.receiver_unit_id.as_deref() == Some(unit)
                        && (wire.revision == *known_revision || wire.snapshot.is_some())
                })
        }
        Operation::Unlink { .. } => unreachable!(),
    };
    if !valid {
        return Err(Error::Response);
    }
    let image = wire
        .snapshot
        .map(|snapshot| {
            let value =
                crate::projection_delivery::sender::snapshot::Snapshot::decode(snapshot, cancel)
                    .map(Image)
                    .map_err(image_error)?;
            if value.digest() != wire.digest {
                return Err(Error::Image);
            }
            Ok(value)
        })
        .transpose()?;
    if cancel.load(std::sync::atomic::Ordering::Acquire) {
        return Err(Error::Stopped);
    }
    Ok(State::Content(Box::new(Content {
        envelope: wire.envelope,
        revision: wire.revision,
        digest: wire.digest,
        linked: wire.linked,
        receiver_device_id: wire.receiver_device_id,
        receiver_unit_id: wire.receiver_unit_id,
        image,
        transport: wire.transport,
        error: wire.error,
    })))
}

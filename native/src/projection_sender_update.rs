//! 既有投送的单次CAS更新；初始签名/PNG写前记录不变，失败不获得重建或解绑权。
use super::*;

pub(crate) struct Update {
    snapshot: snapshot::Snapshot,
    prior: u64,
}
pub(crate) fn validate(
    expected: &binding::Binding,
    intent: &Intent,
    response: &Response,
) -> Result<(), Error> {
    if expected.stopped()
        || !expected.matches_intent(intent)
        || response.revision() < expected.revision()
        || (response.revision() == expected.revision() && response.digest() != expected.digest())
    {
        return Err(Error::Changed);
    }
    Ok(())
}
impl Update {
    pub(crate) fn prepare(
        frame: Frame,
        response: &Response,
        cancel: &AtomicBool,
    ) -> Result<Option<Self>, Error> {
        let snapshot = snapshot::Snapshot::from_png(&frame.png, frame.width, frame.height, cancel)?;
        if snapshot.digest == response.digest() {
            return Ok(None);
        }
        if response.revision() >= envelope::MAX_REVISION {
            return Err(Error::Limit);
        }
        Ok(Some(Self {
            snapshot,
            prior: response.revision(),
        }))
    }
    pub(crate) fn matches(&self, response: &Response) -> bool {
        response.revision() == self.prior + 1 && response.digest() == self.snapshot.digest
    }
    pub(crate) fn body(&self, active: &Active) -> Result<Pixels, Error> {
        active.check()?;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            projection_id: &'a str,
            source_session_id: &'a str,
            prior_revision: u64,
            revision: u64,
            digest: &'a str,
            snapshot: &'a snapshot::Snapshot,
        }
        let envelope = active.intent().envelope();
        let bytes = crate::session_write::encode_bounded(
            &Body {
                projection_id: envelope.projection_id(),
                source_session_id: envelope.source_session(),
                prior_revision: self.prior,
                revision: self.prior + 1,
                digest: &self.snapshot.digest,
                snapshot: &self.snapshot,
            },
            active.lease().preparation_cancel(),
        )
        .map_err(|_| Error::Limit)?;
        if bytes.len() > super::super::accept::MAX_RESPONSE {
            return Err(Error::Limit);
        }
        active.check()?;
        Ok(bytes)
    }
}
#[cfg(test)]
#[path = "projection_sender_update_tests.rs"]
mod tests;

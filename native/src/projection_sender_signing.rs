//! 本机发送签名保留旧协议的十一项消息；重开只能验原签名，不能重新签名。
use super::*;
use crate::device_identity::Identity;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};

impl Envelope {
    pub(in crate::projection_delivery) fn sign_projection(
        identity: &Identity,
        origin: &str,
        unit: &str,
        kind: &str,
        digest: &str,
        now: u64,
    ) -> Result<Self, Error> {
        let actor = identity
            .device_id()
            .filter(|id| identifier(id))
            .ok_or(Error::Envelope)?;
        if canonical_origin(origin).as_deref() != Ok(origin)
            || !identifier(unit)
            || !matches!(kind, "sticker" | "art")
            || !super::digest(digest)
        {
            return Err(Error::Envelope);
        }
        let mut value = Self {
            protocol: "neuro.qr-projection.v1".into(),
            projection_id: format!("projection:{}", guid()?),
            server_origin: origin.into(),
            source: Source {
                device_id: actor.into(),
                session_id: format!("session:{}", guid()?),
                unit_id: unit.into(),
                revision: 1,
            },
            content: Content {
                kind: kind.into(),
                digest: digest.into(),
            },
            expires_at_ms: now
                .checked_add(270_000)
                .filter(|time| *time <= MAX_REVISION)
                .ok_or(Error::Envelope)?,
            nonce: guid()?,
            signature: Signature {
                algorithm: "ed25519".into(),
                key_id: actor.into(),
                value: String::new(),
            },
        };
        value.signature.value = URL_SAFE_NO_PAD.encode(
            identity
                .sign(value.signature_message().as_bytes())
                .map_err(|_| Error::Envelope)?,
        );
        value.validate(origin)?;
        Ok(value)
    }
    pub(in crate::projection_delivery) fn verify_source(
        &self,
        identity: &Identity,
    ) -> Result<(), Error> {
        self.validate(self.origin())?;
        if identity.device_id() != Some(self.source_device()) {
            return Err(Error::Envelope);
        }
        let key = STANDARD
            .decode(identity.public_key().map_err(|_| Error::Envelope)?)
            .map_err(|_| Error::Envelope)?;
        let signature = URL_SAFE_NO_PAD
            .decode(&self.signature.value)
            .map_err(|_| Error::Envelope)?;
        ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key)
            .verify(self.signature_message().as_bytes(), &signature)
            .map_err(|_| Error::Envelope)
    }
    pub(in crate::projection_delivery) fn source_unit(&self) -> &str {
        &self.source.unit_id
    }
    pub(in crate::projection_delivery) fn content_digest(&self) -> &str {
        &self.content.digest
    }
    fn signature_message(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            self.protocol,
            self.projection_id,
            self.server_origin,
            self.source.device_id,
            self.source.session_id,
            self.source.unit_id,
            self.source.revision,
            self.content.kind,
            self.content.digest,
            self.expires_at_ms,
            self.nonce
        )
    }
}
fn guid() -> Result<String, Error> {
    let guid =
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.map_err(|_| Error::Envelope)?;
    Ok(format!("{guid:?}").replace('-', "").to_ascii_lowercase())
}

//! Explicit-confirmation protocol adapter; local immutable-input fingerprints must match, and runtime progress cannot construct challenges.
use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub(super) const PROTOCOL: &str = "loom.art.local.service.confirmation.v1";
pub(super) const MAX_REPLY: usize = image_output::MAX_REPLY + 4096;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Challenge {
    pub nonce: String,
    pub fingerprint: String,
    pub plugin_id: String,
    pub command_id: String,
    pub package_digest: String,
    pub scope_id: String,
    pub expires_at_ms: u64,
}
#[derive(Serialize)]
pub(super) struct Decision {
    pub challenge: Challenge,
    pub approve: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    protocol_version: String,
    reply: Reply,
    image: Option<image_output::Image>,
    confirmation: Option<Challenge>,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
impl Call<'_> {
    pub(crate) fn request_confirmation(&mut self) -> Result<(), Error> {
        if self.submitted {
            return Err(Error::Sequence);
        }
        self.confirmable = true;
        Ok(())
    }
    pub(super) fn fingerprint(&self) -> Result<String, Error> {
        let hashes = self
            .images
            .as_ref()
            .map(|images| {
                images
                    .images
                    .iter()
                    .map(|image| hash(image.png.as_bytes()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(hash(
            &serde_json::to_vec(&(
                &self.input,
                self.timeout_ms,
                self.image_output,
                true,
                hashes,
            ))
            .map_err(|_| Error::Wire)?,
        ))
    }
    pub(super) fn encode_confirmation(
        &self,
        request: &Request<'_>,
        submit: bool,
        decide: bool,
    ) -> Result<Vec<u8>, Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Submit<'a, 'b> {
            protocol_version: &'static str,
            call: &'a Request<'b>,
            images: &'a [images::Image],
            image_output: bool,
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Control<'a, 'b> {
            protocol_version: &'static str,
            control: &'a Request<'b>,
            decision: Option<&'a Decision>,
        }
        if submit {
            serde_json::to_vec(&Submit {
                protocol_version: PROTOCOL,
                call: request,
                images: self
                    .images
                    .as_ref()
                    .map(|v| v.images.as_slice())
                    .unwrap_or_default(),
                image_output: self.image_output,
            })
        } else {
            serde_json::to_vec(&Control {
                protocol_version: PROTOCOL,
                control: request,
                decision: if decide { self.decision.as_ref() } else { None },
            })
        }
        .map_err(|_| Error::Wire)
    }
    pub(super) fn parse_confirmation(&mut self, bytes: &[u8]) -> Result<Reply, Error> {
        if bytes.len() > MAX_REPLY {
            return Err(Error::Limit);
        }
        let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| Error::Wire)?;
        if envelope.protocol_version != PROTOCOL {
            return Err(Error::Identity);
        }
        let reply = Reply::parse(
            &serde_json::to_vec(&envelope.reply).map_err(|_| Error::Wire)?,
            &self.owner,
            self.prior.as_ref(),
        )?;
        if (self.image_output && reply.status == Status::Succeeded) != envelope.image.is_some() {
            return Err(Error::Wire);
        }
        if let Some(challenge) = &envelope.confirmation {
            let safe = |v: &str| {
                !v.is_empty()
                    && v.len() <= 256
                    && v.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._:-/".contains(&b))
            };
            let digest = |v: &str| {
                v.len() == 64
                    && v.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            };
            if reply.status.terminal()
                || self.decision.is_some()
                || !safe(&challenge.nonce)
                || !safe(&challenge.plugin_id)
                || !safe(&challenge.scope_id)
                || !digest(&challenge.package_digest)
                || challenge.command_id
                    != self
                        .owner
                        .service
                        .strip_prefix("capability:")
                        .ok_or(Error::Identity)?
                        .replacen(':', "/", 1)
                || self.challenge.as_ref().is_some_and(|old| old != challenge)
                // Submitted-call inputs are immutable; skip recomputation only when fully matching the last validated challenge.
                || (self.challenge.is_none() && challenge.fingerprint != self.fingerprint()?)
                || challenge.expires_at_ms <= now().map_err(Error::Http)?
            {
                return Err(Error::Identity);
            }
        }
        self.challenge = envelope.confirmation;
        self.output_image = envelope
            .image
            .map(|image| image_output::decode(image, &self.owner))
            .transpose()?;
        Ok(reply)
    }
    pub(super) fn decide(&mut self, approve: bool) -> Result<Reply, Error> {
        if !self.confirmable || self.decision.is_some() {
            return Err(Error::Sequence);
        }
        self.decision = Some(Decision {
            challenge: self.challenge.take().ok_or(Error::Sequence)?,
            approve,
        });
        self.exchange("/v1/art/local/service/confirm", false)
    }
    pub(super) fn confirmation_text(&self) -> Result<String, Error> {
        let challenge = self.challenge.as_ref().ok_or(Error::Sequence)?;
        let input = serde_json::to_string_pretty(&self.input).map_err(|_| Error::Wire)?;
        let escape = |v: &str, multiline: bool| {
            v.chars().flat_map(|ch| {
            if multiline && ch == '\n' { vec!['\n'] }
            else if ch.is_control() || matches!(ch,'\u{061c}'|'\u{200e}'|'\u{200f}'|'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}') { ch.escape_unicode().collect::<Vec<_>>() } else { vec![ch] }
        }).collect::<String>()
        };
        let text = crate::ui_text::format(
            "native-art-confirmation",
            &[
                ("art", escape(&self.owner.binding.art_id, false).into()),
                (
                    "version",
                    escape(&self.owner.binding.art_version, false).into(),
                ),
                ("service", escape(&self.owner.service, false).into()),
                ("plugin", escape(&challenge.plugin_id, false).into()),
                ("package", escape(&challenge.package_digest, false).into()),
                ("fingerprint", escape(&challenge.fingerprint, false).into()),
                (
                    "count",
                    self.images.as_ref().map_or(0, |v| v.images.len()).into(),
                ),
                (
                    "output",
                    crate::ui_text::text(if self.image_output {
                        "common-yes"
                    } else {
                        "common-no"
                    })
                    .into(),
                ),
                ("input", escape(&input, true).into()),
            ],
        )
        .replace('\n', "\r\n");
        if text.len() > 512 * 1024 {
            return Err(Error::Limit);
        }
        Ok(text)
    }
}

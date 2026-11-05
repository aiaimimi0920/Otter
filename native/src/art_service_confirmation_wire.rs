//! 显式确认协议适配；本地不可变输入指纹必须匹配，runtime进度不能构造挑战。
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
                || challenge.fingerprint != self.fingerprint()?
                || challenge.expires_at_ms <= now().map_err(Error::Http)?
                || self.challenge.as_ref().is_some_and(|old| old != challenge)
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
            if multiline && ch == '\n' { vec!['\r','\n'] }
            else if ch.is_control() || matches!(ch,'\u{061c}'|'\u{200e}'|'\u{200f}'|'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}') { ch.escape_unicode().collect::<Vec<_>>() } else { vec![ch] }
        }).collect::<String>()
        };
        let text = format!("允许本次敏感服务调用？\r\nArt：{} @ {}\r\n服务：{}\r\n插件：{}\r\n包摘要：{}\r\n输入摘要：{}\r\n图片数：{}；图片输出：{}\r\n\r\n参数（由 Art 提供）：\r\n{}", escape(&self.owner.binding.art_id,false),escape(&self.owner.binding.art_version,false),escape(&self.owner.service,false),challenge.plugin_id,challenge.package_digest,challenge.fingerprint,self.images.as_ref().map_or(0,|v| v.images.len()),self.image_output,escape(&input,true));
        if text.len() > 512 * 1024 {
            return Err(Error::Limit);
        }
        Ok(text)
    }
}

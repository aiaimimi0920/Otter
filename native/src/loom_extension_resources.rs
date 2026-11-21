//! 命令上传的有界数据值；权限只来自已校验的目录，跨插件 opaque 附件不能进入请求。
use super::{catalog, Error};
use base64::Engine;
use serde::Serialize;
use serde_json::Value;
pub(super) const REQUEST_BYTES: usize = 24 * 1024 * 1024;
pub struct Access {
    pub(super) plugin: String,
    pub(super) image: bool,
    pub(super) attachments: bool,
    pub(super) ocr_text: bool,
}
impl Access {
    pub fn plugin(&self) -> &str {
        &self.plugin
    }
    pub fn image_allowed(&self) -> bool {
        self.image
    }
    pub fn attachments_allowed(&self) -> bool {
        self.attachments
    }
    pub fn input_context(&self) -> Option<&str> {
        self.ocr_text.then_some("ocr-text.v1")
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    kind: &'static str,
    mime: String,
    data_base64: String,
}
impl Image {
    pub fn new(mime: &str, bytes: &[u8]) -> Result<Self, Error> {
        let mime = mime.to_ascii_lowercase();
        if bytes.is_empty() || bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::Limit);
        }
        if !mime.strip_prefix("image/").is_some_and(|s| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".+-".contains(&b))
        }) {
            return Err(Error::Wire);
        }
        Ok(Self {
            kind: "image",
            mime,
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        })
    }
}
#[derive(Serialize)]
#[serde(transparent)]
pub struct Attachment(crate::unit_attachment::Attachment);
impl Attachment {
    pub fn from_validated(value: crate::unit_attachment::Attachment) -> Self {
        Self(value)
    }
    /// Invocation excludes host-only digest and visibility fields.
    pub fn parse(value: Value) -> Result<Self, Error> {
        crate::unit_attachment::Attachment::parse(value)
            .map(Self)
            .map_err(|error| match error {
                crate::unit_attachment::Error::Wire => Error::Wire,
                crate::unit_attachment::Error::Limit => Error::Limit,
            })
    }
}
pub struct Prepared {
    input: Value,
    image: Option<Image>,
    attachments: Vec<Attachment>,
}
impl Prepared {
    pub fn new(input: Value) -> Result<Self, Error> {
        bounded_json(&input, REQUEST_BYTES)?;
        Ok(Self {
            input,
            image: None,
            attachments: Vec::new(),
        })
    }
    pub fn image(mut self, image: Image) -> Self {
        self.image = Some(image);
        self
    }
    pub fn attachment(mut self, attachment: Attachment) -> Result<Self, Error> {
        if self.attachments.len() >= 32 {
            return Err(Error::Limit);
        }
        self.attachments.push(attachment);
        Ok(self)
    }
    pub(super) fn wire(
        self,
        access: &Access,
    ) -> Result<(Value, Option<Value>, Option<Value>), Error> {
        if self.image.is_some() && !access.image
            || !self.attachments.is_empty() && !access.attachments
            || self
                .attachments
                .iter()
                .any(|a| a.0.plugin() != access.plugin)
        {
            return Err(Error::Denied);
        }
        let attachments = if self.attachments.is_empty() {
            None
        } else {
            let value = serde_json::to_value(self.attachments).map_err(|_| Error::Wire)?;
            if serde_json::to_vec(&value).map_err(|_| Error::Wire)?.len() > catalog::MAX_BYTES {
                return Err(Error::Limit);
            }
            Some(value)
        };
        let image = self
            .image
            .map(|image| serde_json::to_value([image]).map_err(|_| Error::Wire))
            .transpose()?;
        Ok((self.input, image, attachments))
    }
}
fn bounded_json(value: &Value, bytes: usize) -> Result<(), Error> {
    crate::unit_attachment_json::bounded(value, bytes).map_err(|_| Error::Limit)
}
#[cfg(test)]
#[path = "loom_extension_resources_tests.rs"]
mod tests;

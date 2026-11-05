//! 私有 child 服务交换：脚本只提供服务/JSON，绝不提供执行许可或宿主身份。
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;

pub const MAX_CALLS: usize = 32;
pub const MAX_JSON: usize = 64 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Call {
    pub id: u32,
    pub service: String,
    pub input: Value,
    pub timeout_ms: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub image_output: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub confirmation: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) canvases: Vec<super::canvas::CanvasFrame>,
}
impl Call {
    pub fn validate(&self) -> io::Result<()> {
        if self.id == 0
            || self.id > MAX_CALLS as u32
            || self.service.is_empty()
            || self.service.len() > 256
            || !self
                .service
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
            || self.timeout_ms == 0
            || self.timeout_ms > 60_000
            || self.images.len() > 4
            || self.images.iter().enumerate().any(|(i, id)| {
                !crate::surface_javascript_event::safe(id) || self.images[..i].contains(id)
            })
            || !self.images.is_empty() && !self.canvases.is_empty()
            || self.canvases.len() > 4
            || self.canvases.iter().enumerate().any(|(i, frame)| {
                !crate::surface_javascript_event::safe(&frame.id)
                    || self.canvases[..i].iter().any(|v| v.id == frame.id)
                    || !matches!(super::canvas::bytes([frame.width, frame.height]), Ok(v) if v > 0)
                    || frame.png.len() > super::canvas::MAX_PNG * 4 / 3 + 4
            })
            || self.canvases.iter().map(|v| v.png.len()).sum::<usize>() > super::canvas::MAX_ENCODED
            || serde_json::to_vec(&self.input)
                .map_err(|_| super::invalid())?
                .len()
                > MAX_JSON
        {
            return Err(super::invalid());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResultValue {
    Succeeded {
        output: Value,
    },
    Image {
        output: Value,
        image: super::images::Header,
        #[serde(skip)]
        png: Option<crate::pixels::Pixels>,
    },
    Failed {
        error: String,
    },
}
impl ResultValue {
    pub fn validate(&self) -> io::Result<()> {
        match self {
            Self::Succeeded { output } | Self::Image { output, .. } => {
                if serde_json::to_vec(output)
                    .map_err(|_| super::invalid())?
                    .len()
                    > MAX_JSON
                {
                    return Err(super::invalid());
                }
            }
            Self::Failed { error } => {
                if error.is_empty()
                    || error.len() > 128
                    || !error
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
                {
                    return Err(super::invalid());
                }
            }
        }
        if let Self::Image { image, png, .. } = self {
            super::images::validate(std::slice::from_ref(image))?;
            if image.bytes > super::canvas::MAX_PNG {
                return Err(super::invalid());
            }
            if png.as_ref().is_some_and(|p| p.len() != image.bytes) {
                return Err(super::invalid());
            }
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reply {
    pub sequence: u64,
    pub id: u32,
    pub result: ResultValue,
}

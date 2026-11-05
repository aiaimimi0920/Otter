//! 已验证投送生成确定身份的单贴纸会话；保留完整 qrProjection，不能替代活动工作区。
use super::{Error, Verified};
use crate::{session_placement::Space, session_window::Rect};
use std::sync::atomic::{AtomicBool, Ordering};

impl Verified {
    pub(crate) fn into_session(
        self,
        intent: &crate::projection_delivery::journal::Ticket,
        scope: crate::session_projection_scope::Scope,
        space: Space,
        rect: Rect,
        cancel: &AtomicBool,
    ) -> Result<(crate::pixels::Pixels, crate::image_decode::Image), Error> {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Stopped);
        }
        let binding = super::super::binding::Binding::new(&self, intent)?;
        let encoded =
            crate::loom_base64::encode_png_data_url(&self.received.png, cancel).map_err(|_| {
                if cancel.load(Ordering::Acquire) {
                    Error::Stopped
                } else {
                    Error::Budget
                }
            })?;
        let collaboration = super::edit::initial(&self, intent, scope, rect)?;
        let serde_json::Value::Object(mut fields) = serde_json::json!({
            "id":self.received.unit,"type":"sticker","x":rect.x,"y":rect.y,"w":rect.w,"h":rect.h,
            "params":{},"opacityNormal":1.0,"opacityMini":0.9,"qrProjection":binding
        }) else {
            unreachable!()
        };
        if let Some((_, annotations)) = &collaboration {
            fields.insert("annotationState".into(), annotations.clone());
        }
        let mut window = crate::session_write::WindowCandidate::new(fields);
        window.insert_encoded("src", encoded);
        window
            .write_projection_edit(collaboration.as_ref().map(|v| &v.0))
            .map_err(|_| Error::Response)?;
        // 同一字符串allocation及租约移入window，序列化不再复制大Base64。
        #[derive(serde::Serialize)]
        struct Session {
            #[serde(rename = "documentSchemaVersion")]
            schema: u32,
            #[serde(rename = "documentRevision")]
            revision: u64,
            #[serde(rename = "otterNativeCoordinateSpace")]
            coordinates: serde_json::Value,
            #[serde(rename = "otterNativeProjectionScope")]
            scope: crate::session_projection_scope::Scope,
            stickers: [crate::session_write::WindowCandidate; 1],
            links: [(); 0],
            groups: [(); 0],
        }
        let session = Session {
            schema: 1,
            revision: 0,
            coordinates: {
                let mut root = serde_json::Map::new();
                crate::session_coordinates::install(&mut root, space)
                    .map_err(|_| Error::Response)?;
                root.remove(crate::session_coordinates::KEY).unwrap()
            },
            scope,
            stickers: [window],
            links: [],
            groups: [],
        };
        let bytes = crate::session_write::encode_bounded(&session, cancel).map_err(|_| {
            if cancel.load(Ordering::Acquire) {
                Error::Stopped
            } else {
                Error::Budget
            }
        })?;
        crate::session_document::decode(&bytes).map_err(|_| Error::Response)?;
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Stopped);
        }
        Ok((bytes, self.image))
    }
}

//! 普通捕获的显式Source创建/关闭；仅第二条显式菜单意图广告原窗口消息输入能力。
use super::*;
use serde_json::{json, Value};

pub struct Publication {
    session: String,
    hook: String,
    width: u32,
    height: u32,
    window: Option<(String, u32)>,
    interactive: bool,
    observing: bool,
    h264: bool,
    actor: std::sync::atomic::AtomicU64,
}
impl Publication {
    pub fn new(hook: &str, size: [u32; 2], window: Option<(String, u32)>) -> Result<Self, Error> {
        if !protocol::identifier(hook)
            || crate::viewport::checked_image_bytes(size[0], size[1])
                .is_none_or(|n| n > crate::live_relay_frame::MAX_FRAME_BYTES)
            || window
                .as_ref()
                .is_some_and(|(id, pid)| !protocol::identifier(id) || *pid == 0)
        {
            return Err(Error::Input);
        }
        let mut bytes = [0; 24];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| Error::Random)?;
        Ok(Self {
            session: format!("live:{}", URL_SAFE_NO_PAD.encode(bytes)),
            hook: hook.into(),
            width: size[0],
            height: size[1],
            window,
            interactive: false,
            observing: false,
            h264: false,
            actor: std::sync::atomic::AtomicU64::new(1),
        })
    }
    pub fn session(&self) -> &str {
        &self.session
    }
    pub(crate) fn allow_interaction(&mut self) -> Result<(), Error> {
        if self.window.is_none() {
            return Err(Error::Input);
        }
        self.interactive = true;
        Ok(())
    }
    pub(crate) fn interactive(&self) -> bool {
        self.interactive
    }
    pub(crate) fn allow_observation(&mut self) -> Result<(), Error> {
        if self.window.is_none() {
            return Err(Error::Input);
        }
        self.observing = true;
        Ok(())
    }
    pub(crate) fn observing(&self) -> bool {
        self.observing
    }
    pub(crate) fn matches_observation(&self, target: &crate::live_window_target::Target) -> bool {
        self.window.as_ref().is_some_and(|(id, pid)| {
            *id == format!("{:x}", target.hwnd().0 as usize) && *pid == target.process_id()
        })
    }
    pub fn size(&self) -> [u32; 2] {
        [self.width, self.height]
    }
    pub(crate) fn set_h264(&mut self, enabled: bool) {
        self.h264 = enabled;
    }
    pub(crate) fn h264(&self) -> bool {
        self.h264
    }
    fn body(&self, device: &str, now: u64) -> Value {
        let window = self.window.is_some();
        let (id, pid) = self
            .window
            .as_ref()
            .map(|(id, pid)| (id.as_str(), *pid))
            .unwrap_or((&self.session, std::process::id()));
        json!({"surfaceInstanceId":null,"sourceAttachmentId":null,"envelope":{
            "protocolVersion":protocol::PROTOCOL,"sessionId":self.session,"epoch":1,"sequence":1,
            "messageType":"session_start","payload":{"session":{
                "protocolVersion":protocol::PROTOCOL,"sessionId":self.session,
                "sourceDeviceId":device,"sourceHookId":self.hook,
                "sourceKind":if window {"window"} else {"region"},
                "sourceWindowIdentity":{"windowId":id,"processId":pid},
                "sourceRegion":{"x":0,"y":0,"width":self.width,"height":self.height},
                "regionAnchor":if window {"window"} else {"screen"},
                "frameStream":{"streamId":format!("stream:{}",self.session),
                    "transport":"websocket_binary","endpoint":"/v1/live/media","codec":if self.h264 {"h264"} else {"raw_bgra"},
                    "colorSpace":"srgb","width":self.width,"height":self.height,"targetFps":if self.h264 {30} else {60},
                    "maxBufferedFrames":3,"keyframeInterval":if self.h264 {30} else {1}},
                "interactionCapabilities":if self.interactive { vec!["pointer_move","pointer_button","double_click","drag","wheel","keyboard","cancel"] } else { vec![] },"observationCapabilities":if self.observing { vec!["uia_tree"] } else { vec![] },"triggerBindings":[],
                "viewerDevices":[],"visibilityState":"visible",
                "captureStrategy":if window {"persistent_window_wgc"} else {"persistent_display_wgc"},
                "renderPreservationStrategy":if window {"visible_offscreen"} else {"visible"},
                "revision":1,"createdAtMs":now,"lastSeenAtMs":now},
                "requestedByDeviceId":device,"requestNonce":format!("request:{}",self.session)}}})
    }
}
impl Control<'_> {
    pub fn publish_source(&self, publication: &Publication) -> Result<Snapshot, Error> {
        let expected = publication.body(self.session.device_id(), now()?);
        let bytes = self.send(
            "POST",
            "/v1/live/sessions",
            &serde_json::to_vec(&expected).map_err(|_| Error::Input)?,
            Duration::from_secs(10),
        )?;
        self.validate_source(publication, &bytes)
            .map(|(snapshot, _)| snapshot)
    }
    fn validate_source(
        &self,
        publication: &Publication,
        bytes: &[u8],
    ) -> Result<(Snapshot, Value), Error> {
        let expected = publication.body(self.session.device_id(), now()?);
        let snapshot = Snapshot::parse(bytes, publication.session()).map_err(Error::Protocol)?;
        let id = snapshot.identity();
        if id.source_device() != self.session.device_id()
            || id.source_hook() != publication.hook
            || id.epoch() != 1
        {
            return Err(Error::Input);
        }
        let actual: Value = serde_json::from_slice(bytes).map_err(|_| Error::Input)?;
        if actual["closed"] != false {
            return Err(Error::Input);
        }
        // 不只核 sessionId；服务响应不能替换采集来源、尺寸、codec或广告尚未接通的能力。
        for key in [
            "sourceKind",
            "sourceWindowIdentity",
            "sourceRegion",
            "regionAnchor",
            "frameStream",
            "interactionCapabilities",
            "observationCapabilities",
            "triggerBindings",
        ] {
            if actual["session"][key] != expected["envelope"]["payload"]["session"][key] {
                return Err(Error::Input);
            }
        }
        Ok((snapshot, actual))
    }
    /// 仅确认本次发送水位；不采用服务端actor/epoch，也不补发创建或帧。
    pub(crate) fn source_progress(
        &self,
        publication: &Publication,
        sent: u64,
    ) -> Result<Option<u64>, Error> {
        let bytes = self.send(
            "GET",
            &path(publication.session(), None)?,
            &[],
            Duration::from_secs(3),
        )?;
        let (_, value) = self.validate_source(publication, &bytes)?;
        let frame = value["lastFrameId"].as_u64().ok_or(Error::Input)?;
        if frame > sent {
            return Err(Error::Cursor);
        }
        match value["sourceConnected"].as_bool() {
            Some(true) => Ok((frame == sent && sent > 0).then_some(frame)),
            Some(false) => Ok(None),
            None => Err(Error::Input),
        }
    }
    pub fn close_source(&self, publication: &Publication) -> Result<(), Error> {
        // 观察线程已join，才可对账并关闭；取消/丢ack不以本地旧游标猜close=2。
        if publication.observing {
            self.source_cursor(publication)?;
        }
        let sequence = publication
            .actor
            .load(std::sync::atomic::Ordering::Acquire)
            .checked_add(1)
            .ok_or(Error::Cursor)?;
        let body = json!({"envelope":{"protocolVersion":protocol::PROTOCOL,
            "sessionId":publication.session,"epoch":1,"sequence":sequence,"messageType":"session_end",
            "payload":{"reason":"closed","endedByDeviceId":self.session.device_id(),
                "detail":"Otter publisher stopped"}}});
        let bytes = self.send(
            "POST",
            &path(publication.session(), Some("close"))?,
            &serde_json::to_vec(&body).map_err(|_| Error::Input)?,
            Duration::from_secs(3),
        )?;
        let actual: Value = serde_json::from_slice(&bytes).map_err(|_| Error::Input)?;
        if actual["closed"] != true
            || actual["epoch"] != 1
            || actual["session"]["sessionId"] != publication.session
            || actual["session"]["sourceDeviceId"] != self.session.device_id()
            || actual["session"]["sourceHookId"] != publication.hook
        {
            return Err(Error::Input);
        }
        Ok(())
    }
}
#[path = "live_source_observation_http.rs"]
mod observation;
pub(crate) use observation::ObservationSubmission;

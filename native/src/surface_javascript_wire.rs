//! 私有持久 JS 会话帧；长度先验检查，序号匹配由父侧负责，不接受路径/宿主 API。
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Read, Write};
pub(crate) const MAX_FRAME: usize = 4 * 1024 * 1024;
pub(crate) const MAX_SOURCE: usize = 1024 * 1024;
#[path = "surface_javascript_canvas_wire.rs"]
pub(crate) mod canvas;
#[path = "surface_javascript_focus.rs"]
pub(crate) mod focus;
#[path = "surface_javascript_images.rs"]
pub(crate) mod images;
#[path = "surface_javascript_result.rs"]
pub(crate) mod result;
#[path = "surface_javascript_service_wire.rs"]
pub mod service;
#[path = "surface_javascript_text_input.rs"]
mod text_input;
pub(crate) use text_input::{InputPhase, TextInput};
#[path = "surface_javascript_local_input.rs"]
mod local_input;
pub(crate) use local_input::{KeyPhase, KeyboardInput, LocalInput};
#[path = "surface_javascript_pointer_input.rs"]
mod pointer_input;
pub(crate) use pointer_input::{PointerInput, PointerPhase};
#[path = "surface_javascript_hover_input.rs"]
mod hover_input;
pub(crate) use hover_input::HoverInput;
#[cfg(test)]
#[path = "surface_javascript_diagnostics.rs"]
pub(crate) mod diagnostics;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub sequence: u64,
    pub viewport: Option<crate::surface_viewport::Viewport>,
    pub command: Command,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Command {
    InstallImages {
        headers: Vec<images::Header>,
    },
    Mount {
        source: String,
        snapshot: Value,
        resources: Value,
    },
    Update {
        snapshot: Value,
        resources: Value,
    },
    Click {
        target: String,
    },
    LocalInput {
        input: LocalInput,
    },
    LiveTrigger {
        event: Box<crate::art_live_trigger::Event>,
    },
    AnimationFrame,
    TimerTick,
    Suspend,
    Resume,
    Dispose,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Emitted {
    pub node_id: String,
    pub event: String,
    pub action: String,
    pub class: String,
    pub payload: Value,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Output {
    #[serde(default)]
    pub result: Option<result::Request>,
    pub scene: Value,
    pub focus: Vec<focus::Request>,
    #[serde(rename = "defaultAllowed")]
    pub default_allowed: Option<bool>,
    pub canvases: Vec<canvas::CanvasFrame>,
    pub events: Vec<Emitted>,
    pub disposed: bool,
    #[serde(rename = "layoutRead")]
    pub layout_read: bool,
    #[serde(rename = "animationPending")]
    pub animation_pending: bool,
    #[serde(rename = "timerDelayMs")]
    pub timer_delay_ms: Option<u32>,
    #[serde(rename = "resizeObserved")]
    pub resize_observed: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Response {
    pub sequence: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<service::Call>,
    // 失败只返回固定状态；不向父侧日志泄露插件源码、快照或异常中的敏感值。
    pub output: Option<Output>,
    // 仅库测试child交付固定诊断；实际产品字节/协议不增加此字段。
    #[cfg(test)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<diagnostics::Report>,
}
pub(crate) fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid JavaScript Surface operation",
    )
}
struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_FRAME.saturating_sub(self.0.len()) {
            return Err(invalid());
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) fn encode(value: &impl Serialize) -> io::Result<Vec<u8>> {
    let mut output = Bounded(Vec::new());
    serde_json::to_writer(&mut output, value).map_err(|_| invalid())?;
    Ok(output.0)
}
pub(crate) fn write(writer: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    let bytes = encode(value)?;
    writer.write_all(b"OJS1")?;
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()
}
pub(crate) fn read<T: DeserializeOwned>(reader: &mut impl Read) -> io::Result<T> {
    let mut header = [0; 8];
    reader.read_exact(&mut header)?;
    let count = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    if &header[..4] != b"OJS1" || count == 0 || count > MAX_FRAME {
        return Err(invalid());
    }
    let mut bytes = vec![0; count];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes).map_err(|_| invalid())
}

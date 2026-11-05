//! 实际 Loom WebSocket 协议，区别于内部 action/payload；资源 owner/传输不属于编码器。
use crate::{
    graph_execution_wire::{json::Json, Error},
    graph_image::{Output, Prepared},
    graph_requests::{Request, Requests},
    workspace_id::UnitId,
};
#[path = "loom_art_config.rs"]
mod config;
#[path = "loom_art_input.rs"]
mod input;
pub use config::Configuration;
pub use input::Input;
const MAX_INLINE_CHARS: usize = (64usize * 1024 * 1024).div_ceil(3) * 4;
const MAX_WIRE_BYTES: usize = MAX_INLINE_CHARS * 2 + 4 * 1024 * 1024;
#[derive(Clone, Copy)]
pub enum OutputTransport {
    Websocket,
    SharedMemoryAndWebsocket,
}
fn start(method: &str, unit: &UnitId, request: &Request) -> Result<Json, Error> {
    let mut json = Json::with_limit(MAX_WIRE_BYTES);
    json.raw("{\"method\":")?;
    json.string(method)?;
    json.raw(",\"params\":{\"protocolVersion\":\"loom.hook.v1\",\"requestId\":")?;
    json.string(request.id.as_str())?;
    json.raw(",\"nodeId\":")?;
    json.string(unit.as_str())?;
    json.raw(",\"generation\":")?;
    json.raw(&request.generation.to_string())?;
    json.raw(",\"deviceId\":\"device:local\"")?;
    Ok(json)
}
pub fn cancel(unit: &UnitId, request: &Request) -> Result<String, Error> {
    let mut json = start("loom.hook.art.cancel", unit, request)?;
    json.raw("}}")?;
    Ok(json.0)
}
/// 资源准备及新鲜度检查完成后串行调用；返回的是待发送消息，不代表服务已接受执行。
pub fn stage(
    registry: &mut Requests,
    unit: &UnitId,
    request_id: UnitId,
    art_id: &str,
    prepared: &Prepared<'_>,
    inputs: &[(&str, Input<'_>)],
    transport: OutputTransport,
) -> Result<crate::graph_execution_wire::Outgoing, Error> {
    crate::graph_execution_wire::stage_encoded(
        registry,
        unit,
        request_id,
        |request| execute(unit, art_id, request, prepared, inputs, transport),
        |previous| cancel(unit, previous),
    )
}
pub fn execute(
    unit: &UnitId,
    art_id: &str,
    request: &Request,
    prepared: &Prepared<'_>,
    inputs: &[(&str, Input<'_>)],
    transport: OutputTransport,
) -> Result<String, Error> {
    let config = Configuration::capture(art_id, prepared)?;
    execute_config(unit, request, &config, inputs, transport)
}
pub fn stage_config(
    registry: &mut Requests,
    unit: &UnitId,
    request_id: UnitId,
    config: &Configuration,
    inputs: &[(&str, Input<'_>)],
    transport: OutputTransport,
) -> Result<crate::graph_execution_wire::Outgoing, Error> {
    crate::graph_execution_wire::stage_encoded(
        registry,
        unit,
        request_id,
        |request| execute_config(unit, request, config, inputs, transport),
        |previous| cancel(unit, previous),
    )
}
pub fn execute_config(
    unit: &UnitId,
    request: &Request,
    config: &Configuration,
    inputs: &[(&str, Input<'_>)],
    transport: OutputTransport,
) -> Result<String, Error> {
    input::validate(&config.image_ports, inputs)?;
    let mut json = start("loom.hook.art.execute", unit, request)?;
    json.raw(",\"artId\":")?;
    json.string(&config.art_id)?;
    json.raw(",\"outputTransports\":")?;
    json.raw(match transport {
        OutputTransport::Websocket => "[\"websocket\"]",
        OutputTransport::SharedMemoryAndWebsocket => "[\"shared_memory\",\"websocket\"]",
    })?;
    json.raw(",\"inputs\":{")?;
    let mut first = true;
    for (name, input) in inputs {
        json.field(name, &mut first)?;
        input.write(&mut json)?;
    }
    json.raw("}")?;
    json.raw(&config.tail)?;
    json.raw("}}")?;
    Ok(json.0)
}
#[cfg(test)]
#[path = "loom_art_wire_tests.rs"]
mod tests;

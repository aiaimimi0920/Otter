//! Formal-request encoding and registration transaction; replace the current generation only after all messages are constructed successfully, without owning actual network sends.
use crate::{
    graph_image::{Output, Prepared},
    graph_requests::{Request, Requests},
    workspace_id::UnitId,
};
#[path = "graph_wire_json.rs"]
pub mod json;
use json::Json;
pub const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
#[derive(Debug, PartialEq)]
pub enum Error {
    MessageTooLarge,
    Allocation,
    InvalidValue,
    InvalidArtId,
    Registry(crate::graph_requests::Error),
}
pub struct Outgoing {
    pub request: Request,
    pub cancel: Option<String>,
    pub execute: String,
}
fn identity(json: &mut Json, unit: &UnitId, request: &Request) -> Result<(), Error> {
    json.raw("\"node_id\":")?;
    json.string(unit.as_str())?;
    json.raw(",\"request_id\":")?;
    json.string(request.id.as_str())?;
    json.raw(",\"generation\":")?;
    json.raw(&request.generation.to_string())
}
pub fn cancel(unit: &UnitId, request: &Request) -> Result<String, Error> {
    let mut json = Json::new();
    json.raw("{\"action\":\"cancel_art\",\"payload\":{")?;
    identity(&mut json, unit, request)?;
    json.raw("}}")?;
    Ok(json.0)
}
pub fn execute(
    unit: &UnitId,
    art_id: &str,
    request: &Request,
    prepared: &Prepared<'_>,
) -> Result<String, Error> {
    if art_id.is_empty() || art_id.len() > 256 || art_id.chars().any(char::is_control) {
        return Err(Error::InvalidArtId);
    }
    let mut json = Json::new();
    json.raw("{\"action\":\"execute_art\",\"payload\":{")?;
    identity(&mut json, unit, request)?;
    json.raw(",\"art_id\":")?;
    json.string(art_id)?;
    json.raw(",\"inputs\":{")?;
    let mut first = true;
    for (name, value) in &prepared.images {
        json.field(name, &mut first)?;
        json.string(value)?;
    }
    json.raw("},\"parameters\":{")?;
    let mut first = true;
    for (name, value) in prepared.active_params() {
        let Some(value) = value else {
            continue;
        };
        if matches!(value, Output::Other) {
            continue;
        }
        json.field(name, &mut first)?;
        json.value(value, 0)?;
    }
    json.raw("},\"disabled_parameters\":[")?;
    for (index, name) in prepared.disabled.iter().enumerate() {
        if index != 0 {
            json.raw(",")?;
        }
        json.string(name)?;
    }
    json.raw("]}}")?;
    Ok(json.0)
}
/// The UI owner calls serially; no reentry or I/O waits during message generation, so the generation cannot change between preflight and commit.
pub fn stage(
    registry: &mut Requests,
    unit: &UnitId,
    request_id: UnitId,
    art_id: &str,
    prepared: &Prepared<'_>,
) -> Result<Outgoing, Error> {
    stage_encoded(
        registry,
        unit,
        request_id,
        |request| execute(unit, art_id, request, prepared),
        |previous| cancel(unit, previous),
    )
}
/// Both protocols share the registration transaction; encoding failure must not clear the old request, and closures must not reenter the registry or perform I/O.
pub fn stage_encoded(
    registry: &mut Requests,
    unit: &UnitId,
    request_id: UnitId,
    encode_execute: impl FnOnce(&Request) -> Result<String, Error>,
    encode_cancel: impl FnOnce(&Request) -> Result<String, Error>,
) -> Result<Outgoing, Error> {
    let request = Request {
        id: request_id,
        generation: registry.next_generation(unit).map_err(Error::Registry)?,
    };
    let execute = encode_execute(&request)?;
    let cancel = registry.active(unit).map(encode_cancel).transpose()?;
    registry
        .begin(unit.clone(), request.id.clone())
        .map_err(Error::Registry)?;
    Ok(Outgoing {
        request,
        cancel,
        execute,
    })
}
#[cfg(test)]
#[path = "graph_execution_wire_tests.rs"]
mod tests;

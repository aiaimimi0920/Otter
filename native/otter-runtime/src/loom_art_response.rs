//! 主干 Art 响应合同；持有单条有界 JSON 的输出，不执行图片解码或 UI 发布。
use crate::{graph_requests::Request, workspace_id::UnitId};
use serde_json::Value;
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
#[derive(Debug, PartialEq)]
pub enum Error {
    TooLarge,
    InvalidResult,
}
#[derive(Debug, PartialEq)]
pub enum Event {
    Ignore,
    Progress(f64),
    Preview {
        revision: u64,
        value: Value,
    },
    Result {
        revision: u64,
        outputs: Value,
        candidates: Option<Value>,
    },
    Failure(String),
}

pub fn parse(bytes: &[u8], unit: &UnitId, request: &Request) -> Result<Event, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    let Ok(mut json) = serde_json::from_slice::<Value>(bytes) else {
        return Ok(Event::Ignore);
    };
    if json["protocolVersion"].as_str() != Some("loom.hook.v1")
        || (json["requestId"].as_str() != Some(request.id.as_str())
            && json["params"]["requestId"].as_str() != Some(request.id.as_str()))
    {
        return Ok(Event::Ignore);
    }
    if let Some(method) = json["method"].as_str() {
        match method {
            "loom.hook.art.progress" => Ok(Event::Progress(
                json["params"]["value"].as_f64().unwrap_or(0.0),
            )),
            "loom.hook.art.preview" => {
                let params = &mut json["params"];
                let Some(revision) = revision(params, unit, request, "previewRevision") else {
                    return Ok(Event::Ignore);
                };
                Ok(Event::Preview {
                    revision,
                    value: take(params, "value"),
                })
            }
            "loom.hook.art.result" => result(&mut json["params"], unit, request),
            "loom.hook.art.failure" => Ok(failure(&json["params"])),
            _ => Ok(Event::Ignore),
        }
    } else {
        match json["status"].as_str() {
            Some("failed" | "cancelled") => Ok(failure(&json)),
            Some("succeeded") => result(&mut json["data"], unit, request),
            _ => Ok(Event::Ignore),
        }
    }
}

fn revision(value: &Value, unit: &UnitId, request: &Request, field: &str) -> Option<u64> {
    if value["protocolVersion"].as_str() != Some("loom.hook.v1")
        || value["requestId"].as_str() != Some(request.id.as_str())
        || value["nodeId"].as_str() != Some(unit.as_str())
        || value["generation"].as_u64() != Some(request.generation)
    {
        return None;
    }
    value[field].as_u64().filter(|revision| *revision > 0)
}
fn result(value: &mut Value, unit: &UnitId, request: &Request) -> Result<Event, Error> {
    let revision = revision(value, unit, request, "resultRevision").ok_or(Error::InvalidResult)?;
    let candidates = value
        .as_object_mut()
        .and_then(|value| value.remove("candidates"));
    Ok(Event::Result {
        revision,
        outputs: take(value, "outputs"),
        candidates,
    })
}
fn take(value: &mut Value, field: &str) -> Value {
    value
        .as_object_mut()
        .and_then(|value| value.remove(field))
        .unwrap_or(Value::Null)
}
fn failure(value: &Value) -> Event {
    // 远端错误仅作文本，限制 UI 驻留长度；不得当作格式串、路径或脚本执行。
    Event::Failure(
        value["error"]["message"]
            .as_str()
            .unwrap_or("Art execution failed")
            .chars()
            .take(1024)
            .collect(),
    )
}

#[cfg(test)]
#[path = "loom_art_response_tests.rs"]
mod tests;

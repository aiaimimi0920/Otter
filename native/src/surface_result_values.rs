//! Surface 正式输出到节点值：描述符仍是数据，不能套用 Loom 图片/共享内存解码器。
use super::*;
pub(super) fn output_values(
    raw: &RawValue,
) -> Result<serde_json::Map<String, serde_json::Value>, Error> {
    if raw.get().len() > MAX_BYTES {
        return Err(Error::Limit);
    }
    let wire: Wire<'_> = serde_json::from_str(raw.get()).map_err(|_| Error::Protocol)?;
    // 重用端口身份/类型/数量规则，不能用 Value 的 map 解码掩盖重复端口。
    let ports = crate::surface_result_ports::parse(wire.outputs)?;
    let raw_ports: std::collections::HashMap<String, &RawValue> =
        serde_json::from_str(wire.outputs.get()).map_err(|_| Error::Protocol)?;
    let mut outputs = serde_json::Map::new();
    for port in ports {
        let descriptor = match port.value {
            crate::surface_result_ports::Value::Value { value } => {
                serde_json::from_str(value.get()).map_err(|_| Error::Protocol)?
            }
            // 保留原描述符未知字段，与前端写入 resource/stream 对象的行为一致。
            crate::surface_result_ports::Value::Resource { .. }
            | crate::surface_result_ports::Value::Stream { .. } => {
                let mut value: serde_json::Value =
                    serde_json::from_str(raw_ports[port.id.as_str()].get())
                        .map_err(|_| Error::Protocol)?;
                let field = match port.value {
                    crate::surface_result_ports::Value::Resource { .. } => "resource",
                    _ => "stream",
                };
                value
                    .as_object_mut()
                    .and_then(|value| value.remove(field))
                    .ok_or(Error::Protocol)?
            }
        };
        outputs.insert(port.id, descriptor);
    }
    Ok(outputs)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn raw(outputs: &str) -> Box<RawValue> {
        RawValue::from_string(format!(r#"{{"protocolVersion":"loom.surface.v1","instanceId":"instance:test","requestId":"request:test","generation":1,"resultRevision":2,"outputs":{outputs}}}"#)).unwrap()
    }
    #[test]
    fn values_preserve_false_zero_null_and_empty_instead_of_treating_them_as_missing() {
        let raw = raw(
            r#"{"null":{"kind":"value","value":null},"false":{"kind":"value","value":false},"zero":{"kind":"value","value":0},"empty":{"kind":"value","value":""}}"#,
        );
        let values = output_values(&raw).unwrap();
        assert_eq!(values["null"], serde_json::Value::Null);
        assert_eq!(values["false"], false);
        assert_eq!(values["zero"], 0);
        assert_eq!(values["empty"], "");
        assert!(output_values(&self::raw("{}")).unwrap().is_empty());
        assert_eq!(
            output_values(&self::raw(r#"{"ke\u0079":{"kind":"value","value":7}}"#)).unwrap()["key"],
            7
        );
    }
    #[test]
    fn resources_and_streams_stay_descriptors_and_keep_extension_fields() {
        let raw = raw(&format!(
            r#"{{"image":{{"kind":"resource","resource":{{"resourceId":"sha256:{}","kind":"image","mime":"image/png","size":12,"extra":false}}}},"stream":{{"kind":"stream","stream":{{"streamId":"stream:test","itemType":"json","sequence":0,"extra":3}}}}}}"#,
            "a".repeat(64)
        ));
        let values = output_values(&raw).unwrap();
        assert_eq!(values["image"]["kind"], "image");
        assert_eq!(values["image"]["extra"], false);
        assert_eq!(values["stream"]["sequence"], 0);
        assert_eq!(values["stream"]["extra"], 3);
        assert!(values["image"].get("resource").is_none());
    }
    #[test]
    fn duplicate_ports_and_unknown_kinds_are_not_silently_normalized() {
        for outputs in [
            r#"{"x":{"kind":"value","value":1},"x":{"kind":"value","value":2}}"#,
            r#"{"x":{"kind":"shared_memory","handle":"untrusted"}}"#,
        ] {
            let raw = raw(outputs);
            let original = raw.get().to_owned();
            assert_eq!(output_values(&raw), Err(Error::Protocol));
            assert_eq!(raw.get(), original);
        }
    }
}

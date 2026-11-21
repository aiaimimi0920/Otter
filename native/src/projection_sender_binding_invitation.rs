//! 分享源沿主干 primary qrProjection 保存；原生 marker 才能进入持续同步索引，不改旧未知关联。
use super::*;
use serde_json::{value::RawValue, Value};
use std::{borrow::Cow, collections::BTreeMap};
type Object<'a> = BTreeMap<String, Cow<'a, RawValue>>;

pub(super) fn capture(
    object: &Object<'_>,
    unit: &str,
    scope: Option<&str>,
) -> Result<Option<Binding>, Error> {
    let Some(raw) = object.get("qrProjection").filter(|raw| raw.get() != "null") else {
        return Ok(None);
    };
    if raw.get().len() > MAX_BINDING_BYTES {
        return Err(Error::Limit);
    }
    let link = fields(raw.get()).map_err(|_| Error::Format)?;
    // 非本实现拥有的 source/receiver 原文不能自动获得发送许可。
    if !link.contains_key("otterNativeSend") {
        return Ok(None);
    }
    let wrapped = format!("{{\"link\":{}}}", raw.get());
    let binding = Binding::decode(&wrapped, unit, scope.ok_or(Error::Format)?)?;
    if !binding.is_invitation() {
        return Err(Error::Format);
    }
    Ok(Some(binding))
}
pub(super) fn prepare(
    source: &Source,
    next: &Binding,
    cancel: &AtomicBool,
) -> Result<Source, Error> {
    patch::transform_field(source, next.unit(), cancel, "qrProjection", |object| {
        let old = capture(object, next.unit(), Some(next.scope()))?;
        let mut value = if let Some(old) = old {
            if old.envelope() != next.envelope()
                || old.stopped()
                || next.revision() < old.revision()
                || (next.revision() == old.revision() && next.digest() != old.digest())
                || (old.linked()
                    && (!next.linked()
                        || old.link.receiver_device_id != next.link.receiver_device_id
                        || old.link.receiver_unit_id != next.link.receiver_unit_id))
            {
                return Err(Error::Changed);
            }
            serde_json::from_str(object.get("qrProjection").ok_or(Error::Changed)?.get())
                .map_err(|_| Error::Format)?
        } else {
            if object
                .get("qrProjection")
                .is_some_and(|raw| raw.get() != "null")
            {
                return Err(Error::Changed);
            }
            serde_json::to_value(&next.link).map_err(|_| Error::Format)?
        };
        let updated = serde_json::to_value(&next.link).map_err(|_| Error::Format)?;
        for key in [
            "revision",
            "digest",
            "linked",
            "receiverDeviceId",
            "receiverUnitId",
        ] {
            if let Some(entry) = updated.get(key) {
                value[key] = entry.clone();
            }
        }
        Ok(value)
    })
}
pub(super) fn stop(
    source: &Source,
    expected: &Binding,
    remove: bool,
    cancel: &AtomicBool,
) -> Result<Source, Error> {
    let result =
        patch::transform_field(source, expected.unit(), cancel, "qrProjection", |object| {
            if capture(object, expected.unit(), Some(expected.scope()))?.as_ref() != Some(expected)
            {
                return Err(Error::Changed);
            }
            if remove {
                return Ok(Value::Null);
            }
            let mut value: Value =
                serde_json::from_str(object.get("qrProjection").ok_or(Error::Changed)?.get())
                    .map_err(|_| Error::Format)?;
            value["stopPending"] = true.into();
            Ok(value)
        })?;
    checkpoint::Index::capture(&result).ok_or(Error::Format)?;
    Ok(result)
}

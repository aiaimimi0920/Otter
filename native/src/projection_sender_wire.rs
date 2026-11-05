//! 大日志只借用图片/字段，数组在第 65 项即拒绝；小 envelope 过字节门后才分配。
use super::super::snapshot::Wire as Snapshot;
use super::{Envelope, Error, Selection, MAX_ENTRIES};
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::value::RawValue;
use std::{fmt, marker::PhantomData};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document<'a> {
    version: u32,
    #[serde(borrow)]
    entries: Entries<'a>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Key<'a> {
    #[serde(borrow)]
    scope: &'a str,
    #[serde(borrow)]
    origin: &'a str,
    #[serde(borrow)]
    unit: &'a str,
    #[serde(borrow)]
    target: &'a str,
    #[serde(default, borrow, deserialize_with = "super::super::route::present")]
    offline: Option<&'a RawValue>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record<'a> {
    #[serde(borrow)]
    selection: Key<'a>,
    #[serde(borrow)]
    envelope: &'a RawValue,
    #[serde(borrow)]
    pub(super) snapshot: Snapshot<'a>,
    pub(super) cancelled: bool,
    #[serde(default)]
    pub(super) stop: Option<super::Stop>,
}
impl Record<'_> {
    pub(super) fn selection(&self) -> Result<Selection, Error> {
        let key = &self.selection;
        super::super::route::decode(key.offline, key.scope, key.origin, key.unit, key.target)
    }
    pub(super) fn envelope(&self) -> Result<Envelope, Error> {
        if self.envelope.get().len() > 8192 {
            return Err(Error::Limit);
        }
        serde_json::from_str(self.envelope.get()).map_err(|_| Error::Format)
    }
}
struct Entries<'a>(Vec<Record<'a>>);
impl<'de: 'a, 'a> Deserialize<'de> for Entries<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Limited<'a>(PhantomData<&'a ()>);
        impl<'de: 'a, 'a> Visitor<'de> for Limited<'a> {
            type Value = Entries<'a>;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("at most 64 sender intents")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::with_capacity(MAX_ENTRIES);
                for _ in 0..MAX_ENTRIES {
                    match seq.next_element()? {
                        Some(record) => entries.push(record),
                        None => return Ok(Entries(entries)),
                    }
                }
                if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(serde::de::Error::custom("Sender journal count limit"));
                }
                Ok(Entries(entries))
            }
        }
        deserializer.deserialize_seq(Limited(PhantomData))
    }
}
pub(super) fn decode(bytes: &[u8]) -> Result<Vec<Record<'_>>, Error> {
    let document: Document<'_> = serde_json::from_slice(bytes).map_err(|_| Error::Format)?;
    if document.version != 1 {
        return Err(Error::Format);
    }
    Ok(document.entries.0)
}

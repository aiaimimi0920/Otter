//! 有界journal恢复：不复制Base64字符串，真实PNG尺寸/摘要/像素租约沿sender唯一codec。
use super::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire<'a> {
    #[serde(borrow)]
    anchor: &'a str,
    #[serde(borrow)]
    session: &'a str,
    layout: Layout,
    initial: Objects,
    #[serde(borrow)]
    snapshot: sender::snapshot::Wire<'a>,
}
impl<'de> Deserialize<'de> for Frozen {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let wire = Wire::deserialize(decoder)?;
        let fail = |_| serde::de::Error::custom("invalid frozen projection basis");
        // 外层先限64MiB/逐项8MiB；codec再限2MiB PNG与16M像素，不产生无预算字符串副本。
        // serde本身不携带取消token；此步与既有有界JSON解析一样在原worker内，外层前后检查取消。
        let snapshot = sender::snapshot::Snapshot::decode(wire.snapshot, &AtomicBool::new(false))
            .map_err(|_| serde::de::Error::custom("invalid frozen PNG"))?;
        let value = Frozen(Arc::new(Data {
            anchor: wire.anchor.into(),
            session: wire.session.into(),
            layout: wire.layout,
            initial: wire.initial,
            snapshot,
        }));
        value.validate().map_err(fail)?;
        Ok(value)
    }
}

//! Bounded journal recovery without Base64-string copies; actual PNG dimensions/digests/pixel leases use the sender's sole codec.
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
        // Outer limits are 64MiB total/8MiB per entry; codec further limits PNG to 2MiB and pixels to 16M, without unbudgeted string copies.
        // serde itself carries no cancellation token; like existing bounded JSON parsing, this runs in the original worker with outer pre/post cancellation checks.
        let snapshot = sender::snapshot::Snapshot::decode(wire.snapshot, &AtomicBool::new(false))
            .map_err(|_| serde::de::Error::custom("invalid frozen PNG"))?;
        let value = Frozen(Arc::new(Data {
            anchor: wire.anchor.into(),
            session: wire.session.into(),
            layout: wire.layout,
            initial: wire.initial,
            snapshot: Arc::new(snapshot),
        }));
        value.validate().map_err(fail)?;
        Ok(value)
    }
}

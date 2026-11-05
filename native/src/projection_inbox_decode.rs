//! 收件元数据全量校验后合并；offline origin只取发起请求，跨路由ID冲突或超限不截断。
use super::*;
use serde_json::value::RawValue;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireInbox<'a> {
    #[serde(borrow)]
    invitations: Vec<WireEntry<'a>>,
    #[serde(default)]
    offline_peers: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireEntry<'a> {
    envelope: Envelope,
    revision: u64,
    digest: String,
    source_name: Option<String>,
    delivery: Delivery,
    receive_policy: Option<InboxPolicy>,
    #[serde(default, borrow, deserialize_with = "receive_route::some")]
    route: Option<&'a RawValue>,
    #[serde(default, borrow, deserialize_with = "receive_route::some")]
    peer_id: Option<&'a RawValue>,
    #[serde(default, borrow, deserialize_with = "receive_route::some")]
    offline_transport: Option<&'a RawValue>,
}
#[derive(Deserialize)]
struct Delivery {
    status: String,
}
impl Inbox {
    pub fn parse(bytes: &[u8], origin: &str) -> Result<Self, Error> {
        Self::decode(bytes, origin, false)
    }
    pub(crate) fn parse_offline(bytes: &[u8], origin: &str) -> Result<Self, Error> {
        Self::decode(bytes, origin, true)
    }
    fn decode(bytes: &[u8], origin: &str, offline: bool) -> Result<Self, Error> {
        if bytes.len() > MAX_JSON {
            return Err(Error::Limit);
        }
        let origin = envelope::canonical_origin(origin)?;
        let wire: WireInbox<'_> = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        if wire.invitations.len() > MAX_INVITATIONS {
            return Err(Error::Limit);
        }
        let mut entries = Vec::with_capacity(wire.invitations.len());
        let mut ids = std::collections::BTreeSet::new();
        for entry in wire.invitations {
            let routing = if offline {
                if !receive_route::text(entry.route, "offline_peer") {
                    return Err(Error::Entry);
                }
                let peer = entry
                    .peer_id
                    .and_then(|raw| serde_json::from_str::<&str>(raw.get()).ok())
                    .ok_or(Error::Entry)?;
                Some(receive_route::Offline::new(&origin, peer)?)
            } else {
                if entry.peer_id.is_some()
                    || (entry.route.is_some() && !receive_route::text(entry.route, "shared_loom"))
                {
                    return Err(Error::Entry);
                }
                None
            };
            entry.envelope.validate(if offline {
                entry.envelope.origin()
            } else {
                &origin
            })?;
            if !ids.insert(entry.envelope.projection_id().to_owned()) {
                return Err(Error::Duplicate);
            }
            if entry.offline_transport.is_some()
                || !(entry.envelope.source_revision()..=envelope::MAX_REVISION)
                    .contains(&entry.revision)
                || !envelope::digest(&entry.digest)
                || !matches!(
                    entry.delivery.status.as_str(),
                    "awaiting_confirmation" | "accepted"
                )
                || entry.receive_policy == Some(InboxPolicy::Disabled)
            {
                return Err(Error::Entry);
            }
            let source_name = entry
                .source_name
                .unwrap_or_else(|| entry.envelope.source_device().into());
            if source_name.is_empty()
                || source_name.len() > 1024
                || source_name.chars().any(char::is_control)
            {
                return Err(Error::Entry);
            }
            entries.push(Invitation {
                envelope: entry.envelope,
                revision: entry.revision,
                digest: entry.digest,
                source_name,
                accepted: entry.delivery.status == "accepted",
                offline: routing,
            });
        }
        Ok(Self {
            entries,
            offline_peers: !offline && wire.offline_peers,
        })
    }
    pub(crate) fn merge_offline(&mut self, other: Self) -> Result<(), Error> {
        if self.entries.len() + other.entries.len() > MAX_INVITATIONS {
            return Err(Error::Limit);
        }
        let mut ids: std::collections::BTreeSet<_> = self
            .entries
            .iter()
            .map(|entry| entry.envelope().projection_id())
            .collect();
        if other
            .entries
            .iter()
            .any(|entry| entry.offline.is_none() || !ids.insert(entry.envelope().projection_id()))
        {
            return Err(Error::Duplicate);
        }
        self.entries.extend(other.entries);
        self.offline_peers = false;
        Ok(())
    }
}

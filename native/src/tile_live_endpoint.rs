//! Wall独立媒体合同；沿原固定origin/Device/nonce，不伪造普通Viewer或Surface附件。
use super::*;
impl Endpoint {
    pub(crate) fn wall(
        &self,
        key: &crate::tile_shared::Key,
        session: &str,
        token: &str,
        device: &str,
        format: crate::tile_live_packet::WallMediaFormat,
    ) -> Result<Prepared, Error> {
        if !crate::tile_wire::identifier(&key.endpoint)
            || !crate::tile_wire::identifier(&key.lease)
            || !crate::tile_wire::identifier(session)
            || key.revision == 0
            || key.revision > crate::tile_wire::MAX_SAFE
        {
            return Err(Error::Identity);
        }
        // 复用认证头与随机nonce构造；随后明确替换专属path/query/profile，不继承普通Live参数。
        let mut p = self.prepare(session, device, None, Credential::Device(token))?;
        p.url.set_path("/v1/walls/live/media");
        p.url.set_query(None);
        p.url
            .query_pairs_mut()
            .append_pair("endpointId", &key.endpoint)
            .append_pair("leaseId", &key.lease)
            .append_pair("revision", &key.revision.to_string())
            .append_pair("sessionId", session)
            .append_pair("format", format.name());
        let mut request = p
            .url
            .as_str()
            .into_client_request()
            .map_err(|_| Error::Request)?;
        for name in ["authorization", "x-loom-device-nonce"] {
            request
                .headers_mut()
                .insert(name, p.request.headers()[name].clone());
        }
        request.headers_mut().insert(
            "sec-websocket-protocol",
            HeaderValue::from_static(crate::tile_live_packet::PROTOCOL),
        );
        p.request = request;
        Ok(p)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wall_identity_profile_and_nonce_do_not_inherit_ordinary_live_query() {
        let e = Endpoint::new("https://127.0.0.1:9443", RootCertStore::empty()).unwrap();
        let k = crate::tile_shared::Key {
            endpoint: "tile:a".into(),
            lease: "lease/b".into(),
            wall: "wall".into(),
            revision: 2,
            clock: "c".into(),
        };
        let a = e
            .wall(
                &k,
                "session/source",
                "test-token",
                "device",
                crate::tile_live_packet::WallMediaFormat::RawBgra,
            )
            .unwrap();
        let b = e
            .wall(
                &k,
                "session/source",
                "test-token",
                "device",
                crate::tile_live_packet::WallMediaFormat::Png,
            )
            .unwrap();
        assert_eq!(a.url.as_str(),"wss://127.0.0.1:9443/v1/walls/live/media?endpointId=tile%3Aa&leaseId=lease%2Fb&revision=2&sessionId=session%2Fsource&format=raw_bgra");
        assert_eq!(a.request.headers()["authorization"], "Device test-token");
        assert!(a.request.headers()["authorization"].is_sensitive());
        assert_ne!(
            a.request.headers()["x-loom-device-nonce"],
            b.request.headers()["x-loom-device-nonce"]
        );
        assert_eq!(
            a.request.headers()["sec-websocket-protocol"],
            "loom.wall.media.v1"
        );
        assert_eq!(b.url.query_pairs().last().unwrap().1, "png");
        let mut invalid = k;
        invalid.revision = 0;
        assert!(e
            .wall(
                &invalid,
                "session",
                "test-token",
                "device",
                crate::tile_live_packet::WallMediaFormat::RawBgra
            )
            .is_err());
    }
}

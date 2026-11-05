//! 当前签名包的源码缓存键；缓存不授执行许可，也没有独立的旧准备入口。
use super::*;
use sha2::{Digest, Sha256};
pub(super) fn key(request: &Request, pin: &Pin) -> [u8; 32] {
    let b = &request.binding;
    // 缓存不含workspace/request代际，也不提供这些身份的授权；当前完整grant另行核验。
    Sha256::digest(
        serde_json::to_vec(&(
            &b.origin,
            &b.device_id,
            &b.art_id,
            &b.art_version,
            &b.package_digest,
            &b.sdk_version,
            &pin.key_id,
            pin.public_key,
        ))
        .expect("bounded host cache key"),
    )
    .into()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_key_separates_origin_device_package_sdk_and_issuer_but_not_request() {
        let request = crate::art_local_requests::tests::request();
        let pin = Pin {
            key_id: "issuer".into(),
            public_key: [7; 32],
        };
        let baseline = key(&request, &pin);
        for field in 0..8 {
            let mut next = crate::art_local_requests::tests::request();
            let binding = &mut next.binding;
            match field {
                0 => binding.origin = "https://other.example".into(),
                1 => binding.device_id = "other-device".into(),
                2 => binding.art_id = "vendor/other".into(),
                3 => binding.art_version = "2.0.0".into(),
                4 => binding.package_digest = "b".repeat(64),
                5 => binding.sdk_version = "otter.art.v2".into(),
                6 => {
                    assert_ne!(
                        baseline,
                        key(
                            &next,
                            &Pin {
                                key_id: "rotated".into(),
                                public_key: pin.public_key
                            }
                        )
                    );
                    continue;
                }
                _ => {
                    assert_ne!(
                        baseline,
                        key(
                            &next,
                            &Pin {
                                key_id: pin.key_id.clone(),
                                public_key: [8; 32]
                            }
                        )
                    );
                    continue;
                }
            }
            assert_ne!(baseline, key(&next, &pin));
        }
        let mut next = request;
        next.binding.request_id = "new-request".into();
        next.binding.workspace_id = "new-workspace".into();
        next.binding.instance_id = "new-instance".into();
        next.binding.generation += 1;
        assert_eq!(baseline, key(&next, &pin));
    }
}

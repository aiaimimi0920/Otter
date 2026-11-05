//! 已验证原始像素身份随 Binding 保存；重编码 PNG 不改变证明，烘焙本地擦除则不匹配。
use super::*;
use crate::image_decode::Image;
use sha2::{Digest, Sha256};
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Proof {
    width: u32,
    height: u32,
    sha256: String,
}
impl Proof {
    pub(super) fn capture(image: &Image, mut cancelled: impl FnMut() -> bool) -> Option<Self> {
        let mut digest = Sha256::new();
        for chunk in image.pixels.chunks(64 * 1024) {
            if cancelled() {
                return None;
            }
            digest.update(chunk);
        }
        if cancelled() {
            return None;
        }
        Some(Self {
            width: image.width,
            height: image.height,
            sha256: format!("{:x}", digest.finalize()),
        })
    }
}
impl Binding {
    pub(crate) fn source_dimensions(&self) -> Option<(u32, u32)> {
        self.source_pixels
            .as_ref()
            .map(|proof| (proof.width, proof.height))
    }
    /// 只在后台计算大像素 SHA；缺旧证明时仍可 read 元数据/终态，不能覆盖不明本地图。
    pub(crate) fn source_matches(&self, image: &Image, cancelled: impl FnMut() -> bool) -> bool {
        self.source_pixels
            .as_ref()
            .is_some_and(|proof| Proof::capture(image, cancelled).as_ref() == Some(proof))
    }
    pub(crate) fn same_snapshot(&self, expected: &Self) -> bool {
        let mut copy = self.clone();
        if expected.source_pixels.is_none() {
            copy.source_pixels = None;
        }
        &copy == expected
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_sync_source_proof_stops_between_hash_chunks_without_returning_partial_proof() {
        let image = Image {
            width: 256,
            height: 128,
            pixels: vec![255; 256 * 128 * 4].into(),
        };
        let mut checks = 0;
        assert!(Proof::capture(&image, || {
            checks += 1;
            checks > 1
        })
        .is_none());
        assert_eq!(checks, 2);
        assert!(Proof::capture(&image, || false).is_some());
    }
}

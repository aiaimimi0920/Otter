//! 普通双EXE的有界一次性合成身份/独立profile；只用隔离CA，不读取真实用户配置。
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input<'a> {
    origin: &'a str,
    certificate: &'a str,
    target_device_id: &'a str,
    #[serde(borrow)]
    identity: &'a serde_json::value::RawValue,
    peer_id: &'a str,
    source_origin: &'a str,
    revoke_peer: bool,
}
pub(super) struct Side {
    pub files: fixture::Fixture,
    pub origin: String,
    pub source: String,
    pub peer: String,
    pub target: String,
}
pub(super) fn root() -> PathBuf {
    let root = PathBuf::from(
        std::env::var_os("OTTER_OFFLINE_PRODUCT_INPUT_DIR")
            .expect("explicit ordinary product joint input"),
    );
    assert!(
        root.is_absolute()
            && root
                .canonicalize()
                .unwrap()
                .starts_with(std::env::temp_dir().canonicalize().unwrap())
    );
    root
}
impl Side {
    pub(super) fn load(root: &Path, sender: bool) -> Self {
        let parent = root.join(if sender { "sender" } else { "receiver" });
        let input = parent.join("input.json");
        assert!(fs::metadata(&input).unwrap().len() <= 64 * 1024);
        let bytes = zeroize::Zeroizing::new(fs::read(input).unwrap());
        let data: Input<'_> = serde_json::from_slice(&bytes).unwrap();
        let url = url::Url::parse(data.origin).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("127.0.0.1"));
        assert!(
            !data.revoke_peer
                && !data.target_device_id.is_empty()
                && data.peer_id.starts_with("loom-")
        );
        let identity =
            native::device_identity::Identity::parse(data.identity.get().as_bytes()).unwrap();
        let profile = parent.join("otter-profile");
        fs::create_dir(&profile).unwrap();
        fs::write(
            profile.join("device-identity.json"),
            identity.protected_document().unwrap(),
        )
        .unwrap();
        fs::write(
            profile.join("ca.pem"),
            format!(
                "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
                data.certificate
            ),
        )
        .unwrap();
        if !sender {
            // 默认手动是无写入的no-op；从显式拒绝起步，才能证明真实菜单产生持久confirm。
            fs::write(
                profile.join("projection-receive-policy.json"),
                br#"{"version":1,"policy":"reject"}"#,
            )
            .unwrap();
        }
        let encoded = STANDARD.encode(oracle::png());
        let source = json!({"documentSchemaVersion":1,"documentRevision":7,
            "otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},
            "stickers":[{"id":"product-source","x":40,"y":40,"w":160,"h":80,"src":format!("data:image/png;base64,{encoded}"),
                "imageEditState":{"sourceSize":{"w":32,"h":16},"cropRect":{"x":4,"y":2,"w":16,"h":8}}}],
            "links":[],"groups":[],"extension":{"offlineProductPreserved":true}});
        fs::write(
            profile.join("workspace.json"),
            serde_json::to_vec(&source).unwrap(),
        )
        .unwrap();
        Self {
            files: fixture::Fixture { root: profile },
            origin: data.origin.into(),
            source: data.source_origin.into(),
            peer: data.peer_id.into(),
            target: data.target_device_id.into(),
        }
    }
    pub(super) fn launch(&self, name: &str) -> fixture::Product {
        self.files
            .launch_with_settings(name, &self.origin, &json!({"schemaVersion":2}))
    }
}
pub(super) fn publish(root: &Path, name: &str, value: &Value) {
    let bytes = serde_json::to_vec(value).unwrap();
    let path = root.join(name);
    let staging = root.join(format!("{name}.pending"));
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .unwrap();
    file.write_all(&bytes).unwrap();
    file.sync_all().unwrap();
    drop(file);
    assert!(!path.exists());
    fs::rename(staging, path).unwrap();
}
pub(super) fn wait(root: &Path, name: &str) {
    let end = Instant::now() + Duration::from_secs(15);
    while !root.join(name).exists() {
        assert!(
            Instant::now() < end,
            "product parent marker deadline {name}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

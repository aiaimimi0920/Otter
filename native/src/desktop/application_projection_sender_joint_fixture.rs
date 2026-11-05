//! 联合门只读有界一次性合成身份，真实Client/Store/profile仍走原owner；不读取用户配置。
use super::*;
use base64::Engine;
use serde::Deserialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input<'a> {
    origin: &'a str,
    certificate: &'a str,
    target_device_id: &'a str,
    #[serde(borrow)]
    identity: &'a serde_json::value::RawValue,
    peer_id: Option<&'a str>,
    source_origin: Option<&'a str>,
    #[serde(default)]
    revoke_peer: bool,
}
pub(super) struct Fixture {
    pub(super) root: PathBuf,
    pub(super) profile: PathBuf,
    pub(super) origin: String,
    pub(super) target: String,
    pub(super) peer: Option<String>,
    pub(super) source: String,
    pub(super) revoke: bool,
}
impl Fixture {
    pub(super) fn load() -> (
        Self,
        crate::device_session_client::Client,
        crate::device_identity_store::Store,
    ) {
        let input = PathBuf::from(
            std::env::var_os("OTTER_PROJECTION_JOINT_INPUT").expect("explicit isolated input"),
        );
        assert!(input.is_absolute() && fs::metadata(&input).unwrap().len() <= 64 * 1024);
        let root = input.parent().unwrap().to_path_buf();
        // canonicalize只用于隔离范围核验；不把Windows verbatim路径交给原profile准入。
        assert!(root
            .canonicalize()
            .unwrap()
            .starts_with(std::env::temp_dir().canonicalize().unwrap()));
        let bytes = zeroize::Zeroizing::new(fs::read(input).unwrap());
        let data: Input<'_> = serde_json::from_slice(&bytes).unwrap();
        let url = url::Url::parse(data.origin).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("127.0.0.1"));
        let identity =
            crate::device_identity::Identity::parse(data.identity.get().as_bytes()).unwrap();
        let certificate = base64::engine::general_purpose::STANDARD
            .decode(data.certificate)
            .unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(rustls::pki_types::CertificateDer::from(certificate))
            .unwrap();
        let client = crate::device_session_client::Client::new(
            data.origin,
            roots,
            vec![format!("127.0.0.1:{}", url.port().unwrap())
                .parse()
                .unwrap()],
        )
        .unwrap();
        let profile = root.join("otter-profile");
        fs::create_dir(&profile).unwrap();
        write_once(
            &profile.join("device-identity.json"),
            &identity.protected_document().unwrap(),
        );
        let pem = format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
            data.certificate
        );
        write_once(&profile.join("ca.pem"), pem.as_bytes());
        let store = crate::device_identity_store::Store::new(&profile).unwrap();
        (
            Self {
                root,
                profile,
                origin: data.origin.to_owned(),
                target: data.target_device_id.to_owned(),
                peer: data.peer_id.map(str::to_owned),
                source: data.source_origin.unwrap_or(data.origin).to_owned(),
                revoke: data.revoke_peer,
            },
            client,
            store,
        )
    }
    pub(super) fn publish(&self, name: &str, value: &serde_json::Value) {
        write_once(&self.root.join(name), &serde_json::to_vec(value).unwrap());
    }
    pub(super) fn wait(&self, name: &str) {
        let end = Instant::now() + Duration::from_secs(10);
        while !self.root.join(name).exists() {
            assert!(Instant::now() < end, "joint peer marker deadline: {name}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
fn write_once(path: &Path, bytes: &[u8]) {
    let staging = path.with_extension("pending");
    assert!(!path.exists());
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
    drop(file);
    fs::rename(staging, path).unwrap();
}

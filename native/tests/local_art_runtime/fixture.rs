//! 独立profile和自启子进程；失败材料保留，不访问系统证书库或真实用户设置。
use super::package;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{
    fs,
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
};
pub struct Fixture {
    pub root: PathBuf,
}
pub struct Product(pub Child);
impl Fixture {
    pub fn new() -> Self {
        let root = PathBuf::from(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-art-product-runtime",
        )
        .join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        fs::create_dir_all(&root).unwrap();
        eprintln!("art-product-evidence: {}", root.display());
        let image = STANDARD.encode(include_bytes!("../../../src-tauri/icons/32x32.png"));
        let source = json!({"documentSchemaVersion":1,"documentRevision":7,
            "otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},
            "stickers":[{"id":"product-source","x":40,"y":40,"w":160,"h":160,"src":format!("data:image/png;base64,{image}")}],
            "links":[],"groups":[],"extension":{"productArtPreserved":true}});
        fs::write(
            root.join("workspace.json"),
            serde_json::to_vec(&source).unwrap(),
        )
        .unwrap();
        Self { root }
    }
    pub fn launch(&self, name: &str, origin: &str) -> Product {
        self.launch_with_settings(name, origin, &package::settings(origin))
    }
    pub fn launch_with_settings(&self, name: &str, origin: &str, settings: &Value) -> Product {
        let exe = std::env::var_os("OTTER_ART_PRODUCT_EXE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_otter-native")));
        assert!(exe.is_absolute());
        if let Ok(expected) = std::env::var("OTTER_ART_PRODUCT_SHA256") {
            use sha2::{Digest, Sha256};
            use std::io::Read;
            let mut input = fs::File::open(&exe).unwrap();
            let mut hash = Sha256::new();
            let mut chunk = [0; 64 * 1024];
            loop {
                let length = input.read(&mut chunk).unwrap();
                if length == 0 {
                    break;
                }
                hash.update(&chunk[..length]);
            }
            assert_eq!(
                format!("{:x}", hash.finalize()),
                expected.to_ascii_lowercase()
            );
        } else {
            assert!(
                std::env::var_os("OTTER_ART_PRODUCT_EXE").is_none(),
                "explicit product requires digest"
            );
        }
        fs::write(
            self.root.join("settings.json"),
            serde_json::to_vec(settings).unwrap(),
        )
        .unwrap();
        let error = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join(format!("{name}-stderr.log")))
            .unwrap();
        Product(
            Command::new(exe)
                .args(["--settings"])
                .arg(self.root.join("settings.json"))
                .env("OTTER_NATIVE_PROFILE_DIR", &self.root)
                .env("OTTER_LOOM_TLS_ORIGIN", origin)
                .env("OTTER_LOOM_TLS_CA_FILE", self.root.join("ca.pem"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(error)
                .creation_flags(0x08000000)
                .spawn()
                .unwrap(),
        )
    }
    pub fn workspace(&self) -> Option<Value> {
        serde_json::from_slice(&fs::read(self.root.join("workspace.json")).ok()?).ok()
    }
    pub fn record(&self, name: &str, data: &Value) {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join(name))
            .unwrap();
        serde_json::to_writer_pretty(file, data).unwrap();
    }
}
impl Drop for Product {
    fn drop(&mut self) {
        if self.0.try_wait().is_ok_and(|status| status.is_none()) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

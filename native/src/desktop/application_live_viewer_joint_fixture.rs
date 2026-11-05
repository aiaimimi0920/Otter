//! 真实联合门的隔离身份/固定CA/消息泵；仅使用linshi下新profile，不读取真实配置。
use super::*;
use base64::Engine;
use std::path::PathBuf;
pub(super) struct Context {
    pub root: PathBuf,
    profile: PathBuf,
    pub origin: String,
    pub data: serde_json::Value,
}
impl Context {
    pub fn load() -> Self {
        let input = PathBuf::from(std::env::var_os("OTTER_LIVE_VIEWER_JOINT_INPUT").unwrap());
        assert!(input.is_absolute() && std::fs::metadata(&input).unwrap().len() <= 64 * 1024);
        let root = input.parent().unwrap().to_path_buf();
        let canonical = std::fs::canonicalize(&root).unwrap();
        let base = std::fs::canonicalize("C:/Users/Public/nas_home/AI/GameEditor/linshi").unwrap();
        assert!(canonical != base && canonical.starts_with(base));
        let data: serde_json::Value =
            serde_json::from_slice(&zeroize::Zeroizing::new(std::fs::read(input).unwrap()))
                .unwrap();
        let origin = data["origin"].as_str().unwrap().to_owned();
        let url = url::Url::parse(&origin).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("127.0.0.1"));
        let profile = root.join("native-viewer-profile");
        std::fs::create_dir(&profile).unwrap();
        std::fs::write(
            profile.join("device-identity.json"),
            serde_json::to_vec(&data["identity"]).unwrap(),
        )
        .unwrap();
        std::fs::write(
            profile.join("fixture-ca.pem"),
            format!(
                "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
                data["certificate"].as_str().unwrap()
            ),
        )
        .unwrap();
        Self {
            root,
            profile,
            origin,
            data,
        }
    }
    pub fn config(
        &self,
        origin: &str,
    ) -> std::result::Result<
        crate::device_authorization_config::Config,
        crate::device_authorization_config::Error,
    > {
        fixture::config(origin, &self.profile)
    }
    pub fn authorize(&self, app: &mut Application) {
        let url = url::Url::parse(&self.origin).unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(rustls::pki_types::CertificateDer::from(
                base64::engine::general_purpose::STANDARD
                    .decode(self.data["certificate"].as_str().unwrap())
                    .unwrap(),
            ))
            .unwrap();
        let client = crate::device_session_client::Client::new(
            &self.origin,
            roots,
            vec![format!("127.0.0.1:{}", url.port().unwrap())
                .parse()
                .unwrap()],
        )
        .unwrap();
        app.authorization.borrow_mut().origin = Some(self.origin.clone());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew {
                client,
                store: crate::device_identity_store::Store::new(&self.profile).unwrap(),
            })
            .unwrap();
        for handle in app.authorization_owner.wait_handles() {
            assert_eq!(
                unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000) },
                WAIT_OBJECT_0
            );
        }
        assert!(matches!(
            app.authorization_owner.poll().as_slice(),
            [crate::device_authorization_owner::Event::Authorized { .. }]
        ));
    }
    pub unsafe fn spin(&self, app: &mut Application, ready: impl Fn(&mut Application) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            app.poll_remote_live_with(false, |origin| self.config(origin))
                .unwrap();
            if ready(app) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "real viewer phase: {}",
                app.authorization.borrow().live.status
            );
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    pub unsafe fn signal(&self, app: &mut Application, name: &str) {
        self.spin(app, |_| self.root.join(name).exists());
    }
    pub fn record(&self, name: &str, value: serde_json::Value) {
        let temporary = self.root.join(format!("{name}.tmp"));
        assert!(!temporary.exists() && !self.root.join(name).exists());
        std::fs::write(&temporary, serde_json::to_vec(&value).unwrap()).unwrap();
        std::fs::rename(temporary, self.root.join(name)).unwrap();
    }
    pub unsafe fn offscreen(hwnd: HWND) {
        SetWindowPos(
            hwnd,
            None,
            -10000,
            -10000,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
}

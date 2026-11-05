//! 接收 owner 测试只用既有合成身份/CA/隐藏窗口；服务器仅接受授权两步和一次 accept。
use super::*;
use crate::projection_delivery_runtime::{Job, Operation, Output, Ready};
use crate::{
    desktop::workspace_profile::Profile,
    projection_delivery::{
        accept::tests as png,
        journal::{Store, Ticket},
    },
};

pub(super) struct Fixture {
    pub app: Application,
    _class: Class,
    pub store: Store,
    pub ticket: Option<Ticket>,
    pub origin: String,
    pub offline: bool,
    pub(super) listener: Option<std::net::TcpListener>,
    pub(super) server: Option<std::thread::JoinHandle<std::net::TcpListener>>,
    pub(super) tls: Arc<rustls::ServerConfig>,
    identity: Vec<u8>,
    pub files: Files,
}
impl Fixture {
    pub(super) unsafe fn offline(read: bool) -> Self {
        Self::with_request(
            if read {
                "/v1/offline-projections/read"
            } else {
                "/v1/offline-projections/accept"
            },
            |stream, _| {
                reply(
                    stream,
                    200,
                    &serde_json::to_vec(&crate::projection_delivery::tests::offline::response(
                        "device-old",
                    ))
                    .unwrap(),
                );
            },
        )
    }
    pub(super) fn invitation(&self) -> crate::projection_delivery::Invitation {
        if self.offline {
            crate::projection_delivery::tests::offline::invitation(&self.origin, false)
        } else {
            png::invitation(&self.origin)
        }
    }
    pub(super) unsafe fn new() -> Self {
        Self::with_accept(|stream, origin| {
            let mut response = png::response(origin);
            response["receiverDeviceId"] = "device-old".into();
            reply(stream, 200, &serde_json::to_vec(&response).unwrap());
        })
    }
    pub(super) unsafe fn with_accept(
        reply_accept: impl FnOnce(
                &mut StreamOwned<ServerConnection, crate::loom_bridge_stream::BridgeStream>,
                &str,
            ) + Send
            + 'static,
    ) -> Self {
        Self::with_delivery(false, reply_accept)
    }
    pub(super) unsafe fn with_delivery(
        read: bool,
        reply_delivery: impl FnOnce(
                &mut StreamOwned<ServerConnection, crate::loom_bridge_stream::BridgeStream>,
                &str,
            ) + Send
            + 'static,
    ) -> Self {
        Self::with_request(
            if read {
                "/v1/projections/read"
            } else {
                "/v1/projections/accept"
            },
            reply_delivery,
        )
    }
    pub(super) unsafe fn with_request(
        path: &'static str,
        reply_delivery: impl FnOnce(
                &mut StreamOwned<ServerConnection, crate::loom_bridge_stream::BridgeStream>,
                &str,
            ) + Send
            + 'static,
    ) -> Self {
        let (files, identity, _) = Files::new();
        let before = std::fs::read(files.root().join("device-identity.json")).unwrap();
        let (listener, client, tls, pem) = fixture_with_pem();
        let origin = client.origin().to_owned();
        let offline = path.starts_with("/v1/offline-projections/");
        std::fs::write(files.root().join("ca.pem"), pem).unwrap();
        let server_origin = origin.clone();
        let retained_tls = tls.clone();
        let mut reply_accept = Some(reply_delivery);
        let server = std::thread::spawn(move || {
            for index in 0..3 {
                let mut stream = StreamOwned::new(
                    ServerConnection::new(tls.clone()).unwrap(),
                    accept(&listener),
                );
                let (head, body) = request(&mut stream);
                match index {
                    0 => {
                        assert!(head.starts_with("POST /v1/device-sessions/challenges "));
                        reply(
                            &mut stream,
                            200,
                            br#"{"deviceId":"device-old","challengeId":"id","challenge":"value"}"#,
                        );
                    }
                    1 => {
                        assert!(head.starts_with("POST /v1/device-sessions "));
                        let now = crate::desktop::surface_windows::now_ms().unwrap();
                        reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
                            "deviceId":"device-old", "token":"receive-owner-token", "expiresAtMs":now+60_000
                        })).unwrap());
                    }
                    _ => {
                        assert!(head.starts_with(&format!("POST {path} ")));
                        assert!(head
                            .to_ascii_lowercase()
                            .contains("authorization: device receive-owner-token\r\n"));
                        let invitation = if offline {
                            crate::projection_delivery::tests::offline::invitation(
                                &server_origin,
                                false,
                            )
                        } else {
                            png::invitation(&server_origin)
                        };
                        assert_eq!(
                            body,
                            match path {
                                "/v1/projections/inbox" => serde_json::json!({"policy":"confirm"}),
                                "/v1/projections/read" | "/v1/offline-projections/read" =>
                                    serde_json::json!({"projectionId":invitation.envelope().projection_id(),"knownRevision":0}),
                                "/v1/projections/accept" | "/v1/offline-projections/accept" =>
                                    serde_json::from_slice::<serde_json::Value>(
                                        &invitation.accept_body(&server_origin).unwrap(),
                                    )
                                    .unwrap(),
                                _ => panic!("unsupported fixture operation"),
                            }
                        );
                        reply_accept.take().unwrap()(&mut stream, &server_origin);
                    }
                }
            }
            listener
        });
        let (class, mut app) = application();
        app.install_workspace_checkpoint(Some(Profile::open(files.root()).unwrap()), false)
            .unwrap();
        until(&mut app, |app| {
            !matches!(
                app.projection_profile,
                projection::profile::Owner::Opening(_)
            )
        });
        let store = match &app.projection_profile {
            projection::profile::Owner::Ready { store, .. } => store.clone(),
            _ => panic!("profile not ready"),
        };
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.defer_projection_poll_for_test(&origin);
        app.authorization_owner
            .start(Task::Renew {
                client,
                store: identity,
            })
            .unwrap();
        let handle = app.authorization_owner.wait_handles().next().unwrap();
        assert_eq!(
            WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
            WAIT_OBJECT_0
        );
        app.process_authorization();
        Self {
            app,
            _class: class,
            store,
            ticket: None,
            origin,
            offline,
            listener: None,
            server: Some(server),
            tls: retained_tls,
            identity: before,
            files,
        }
    }
    pub(super) unsafe fn accepted(&mut self, remote: bool) -> Option<Ready> {
        let invitation = self.invitation();
        let scope = projection::receive::Scope::capture(&mut self.app).unwrap();
        self.app.projection_receive.waiting(Some(scope));
        let lease = self
            .app
            .authorization_owner
            .lease(
                &self.origin,
                crate::desktop::surface_windows::now_ms().unwrap(),
            )
            .unwrap();
        let config = config(&self.origin, self.files.root()).unwrap();
        let result = if remote {
            self.app
                .remote_sessions
                .accept_projection(
                    config,
                    lease,
                    Arc::new(invitation.clone()),
                    self.store.clone(),
                )
                .unwrap();
            wait_remote(&self.app);
            self.app.process_authorization();
            None
        } else {
            let mut job = Job::start(
                config,
                lease,
                Operation::Accept {
                    invitation: Box::new(invitation),
                    journal: self.store.clone(),
                },
            )
            .unwrap();
            let handle = job.wait_handle().unwrap();
            assert_eq!(
                WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
                WAIT_OBJECT_0
            );
            let Output::Received(ready) = job.try_join().unwrap().unwrap() else {
                panic!("missing received pixels")
            };
            self.app.projection_profile.refresh();
            Some(*ready)
        };
        self.finish_accept();
        result
    }
    pub(super) fn finish_accept(&mut self) {
        self.ticket = Some(self.store.pending().unwrap().pop().unwrap());
        self.listener = Some(self.server.take().unwrap().join().unwrap());
    }
    pub(super) fn ticket(&self) -> &Ticket {
        self.ticket.as_ref().unwrap()
    }
    pub(super) fn journal(&self) -> Vec<u8> {
        std::fs::read(self.files.root().join("projection-receive.json")).unwrap()
    }
    pub(super) fn unit_present(&self) -> bool {
        self.app
            .workspace
            .units()
            .iter()
            .any(|unit| unit.as_str() == self.ticket().unit())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.app.shutdown();
        if let Some(server) = self.server.take() {
            match server.join() {
                Ok(listener) => self.listener = Some(listener),
                Err(error) => {
                    if std::thread::panicking() {
                        // 保主失败可被 harness 领取；次生 peer panic 不能在 Drop 中 abort 进程。
                        eprintln!(
                            "receive-owner-peer-panic-during-unwind: {}",
                            self.files.root().display()
                        );
                    } else {
                        std::panic::resume_unwind(error);
                    }
                }
            }
        }
        if !std::thread::panicking() {
            assert!(
                matches!(self.listener.as_ref().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
            );
            assert_eq!(
                std::fs::read(self.files.root().join("device-identity.json")).unwrap(),
                self.identity
            );
        }
    }
}
#[test]
fn projection_fixture_primary_panic_is_preserved_when_peer_also_panics() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        fixture.accepted(true);
        until(&mut fixture.app, |app| !app.projection_receive.preparing());
        fixture.app.reset_projection_inbox();
        let listener = fixture.listener.take().unwrap();
        fixture.server = Some(std::thread::spawn(move || {
            let _listener = listener;
            std::panic::panic_any("projection fixture peer panic");
        }));
        let root = fixture.files.root().to_owned();
        let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _fixture = fixture;
            std::panic::panic_any("projection fixture primary panic");
        }))
        .unwrap_err();
        assert_eq!(
            error.downcast_ref::<&str>(),
            Some(&"projection fixture primary panic")
        );
        assert!(root.join("projection-receive.json").is_file());
        assert_eq!(Store::open(&root).unwrap().pending().unwrap().len(), 1);
    }
}
pub(super) fn until(app: &mut Application, done: impl Fn(&Application) -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        app.process_authorization();
        if done(app) {
            break;
        }
        assert!(Instant::now() < end, "receive owner completion deadline");
        std::thread::yield_now();
    }
}
pub(super) unsafe fn checkpoint(app: &mut Application) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        app.pending_workspace_save_with(|_, _, message, warning| assert!(!warning, "{message}"));
        if app.save_job.is_none() {
            break;
        }
        assert!(Instant::now() < end, "receive checkpoint deadline");
        std::thread::yield_now();
    }
    // 只完成原保存 owner；下一轮 UI 结算才允许 show/present，测试可在该真实边界注入取消。
}

//! 由真实 Loom 父进程启动：全新 DPAPI 身份、生产连接 owner 与原菜单发送管线。
use super::*;
use serde_json::json;
use std::path::{Path, PathBuf};
fn marker(root: &Path, name: &str, value: serde_json::Value) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))
        .unwrap();
    file.write_all(&serde_json::to_vec(&value).unwrap())
        .unwrap();
    file.sync_all().unwrap();
}
unsafe fn authorize(app: &mut Application) -> String {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        app.process_authorization();
        if let Some(origin) = app.authorization.borrow().origin.clone() {
            let now = crate::desktop::surface_windows::now_ms().unwrap();
            if app.authorization_owner.token(&origin, now).is_some() {
                return origin;
            }
        }
        assert!(
            Instant::now() < until,
            "{}",
            app.authorization.borrow().status
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
#[ignore = "isolated real Loom parent supplies only local manifest, never identity or TLS overrides"]
fn native_local_loom_bootstrap_and_sender() {
    let _serial = serial();
    let _com = Apartment::new().unwrap();
    let root = PathBuf::from(std::env::var_os("OTTER_LOOM_BOOTSTRAP_ROOT").unwrap());
    assert!(
        root.is_absolute()
            && root
                .canonicalize()
                .unwrap()
                .starts_with(std::env::temp_dir().canonicalize().unwrap())
    );
    let profile = root.join("profile");
    assert!(!profile.exists());
    std::fs::create_dir(&profile).unwrap();
    if root.join("remote-config.json").exists() {
        std::fs::copy(
            root.join("remote-config.json"),
            profile.join("loom-remote.json"),
        )
        .unwrap();
        assert!(!PathBuf::from(std::env::var_os("LOOM_MANIFEST_PATH").unwrap()).exists());
    }
    assert_eq!(
        std::env::var_os("OTTER_NATIVE_PROFILE_DIR").unwrap(),
        profile.as_os_str()
    );
    assert!(std::env::var_os("OTTER_LOOM_TLS_CA_FILE").is_none());
    unsafe {
        let mut app = application(&profile);
        app.projection_profile
            .start(profile.clone(), app.execution_wake.callback());
        app.enable_local_loom();
        let origin = authorize(&mut app);
        assert!(app.authorization.borrow().connected);
        let loom_id = app.authorization.borrow().loom_id.clone().unwrap();
        // 再次选择同一服务不能停止自动续签；不需要用户再点“获取授权”。
        app.authorization.borrow_mut().request = Some(ui::Request::Select(origin.clone()));
        app.process_authorization();
        assert!(app.local_loom.deadline().is_some());
        assert!(app.authorization.borrow().connected);
        assert_eq!(
            app.authorization.borrow().loom_id.as_deref(),
            Some(loom_id.as_str())
        );
        let now = crate::desktop::surface_windows::now_ms().unwrap();
        let device = app
            .authorization_owner
            .lease(&origin, now)
            .unwrap()
            .device_id()
            .to_owned();
        // 同一 owner 的远端续签也必须保留已签出的在途租约。
        let prior = app.authorization_owner.lease(&origin, now).unwrap();
        app.local_loom.refresh();
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            app.process_authorization();
            if app
                .local_loom
                .deadline()
                .is_some_and(|at| at > Instant::now() + Duration::from_secs(60))
            {
                break;
            }
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(!prior.is_revoked());
        let until = Instant::now() + Duration::from_secs(10);
        while app.authorization.borrow().projection.status != "投送收件箱：0 项" {
            app.process_authorization();
            assert!(
                Instant::now() < until,
                "{}",
                app.authorization.borrow().projection.status
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        // 产品启动自动发现；不通过测试手工排一个刷新请求来掩盖缺失的入口。
        let until = Instant::now() + Duration::from_secs(10);
        while app.authorization.borrow().targets.directory.is_none() {
            app.process_authorization();
            assert!(
                Instant::now() < until,
                "{}",
                app.authorization.borrow().targets.status
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        let target = std::env::var("OTTER_LOOM_BOOTSTRAP_TARGET").unwrap();
        let index = app
            .authorization
            .borrow()
            .targets
            .directory
            .as_ref()
            .unwrap()
            .targets()
            .iter()
            .position(|t| t.id() == target)
            .unwrap();
        ui::execute(
            app.documents[0].window.0,
            &app.documents[0].state,
            ui::sender::BASE + index,
        )
        .unwrap();
        app.process_authorization();
        wait_remote(&app);
        app.process_authorization();
        settle(&mut app);
        app.process_authorization();
        let binding = app
            .workspace_checkpoint
            .as_ref()
            .unwrap()
            .sender_index()
            .unwrap()
            .entries()[0]
            .clone();
        marker(
            &root,
            "created.json",
            json!({"envelope":binding.envelope(),"deviceId":device,"digest":binding.digest(),"productionBootstrap":true,"automaticInbox":true,"automaticTargets":true}),
        );
        let end = Instant::now() + Duration::from_secs(10);
        while !root.join("read-verified").exists() {
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(5));
        }
        stop::run(&mut app, &profile, &binding, 9);
        app.shutdown();
        drop(app);
        drain_messages();
        // 同一 profile 重连不重复批准，不能更换设备 epoch 或公钥。
        let mut again = application(&profile);
        again
            .authorization
            .borrow_mut()
            .consent
            .install_paused(true);
        again.enable_local_loom();
        assert_eq!(authorize(&mut again), origin);
        let now = crate::desktop::surface_windows::now_ms().unwrap();
        assert_eq!(
            again
                .authorization_owner
                .lease(&origin, now)
                .unwrap()
                .device_id(),
            device
        );
        again.authorization.borrow_mut().request = Some(ui::Request::Cancel);
        again.process_authorization();
        assert!(again.authorization_owner.token(&origin, now).is_none());
        assert!(!again.local_loom.enabled());
        // 自动续签已停；应用等待器仍须回收真实 Loom 断开任务及其完成通知。
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            again.process_authorization();
            let now = crate::desktop::surface_windows::now_ms().unwrap();
            assert!(!again.local_loom.enabled());
            assert!(again.authorization_owner.token(&origin, now).is_none());
            let view = again.authorization.borrow();
            if view.status == "Loom 已断开" {
                break;
            }
            assert!(
                !view.status.starts_with("本地已断开，Loom 离线确认失败：")
                    && Instant::now() < until,
                "{}",
                view.status
            );
            drop(view);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(again.local_loom.deadline().is_none());
        again.shutdown();
        drop(again);
        drain_messages();
        let mut paused = application(&profile);
        paused.enable_local_loom();
        assert!(!paused.local_loom.enabled());
        assert!(paused.local_loom.deadline().is_none());
        paused.shutdown();
        marker(
            &root,
            "finished.json",
            json!({"sameDeviceAfterReopen":true,"explicitCancelStopsAutomaticConnection":true,"stopped":true}),
        );
    }
}

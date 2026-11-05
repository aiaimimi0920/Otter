use super::*;

#[test]
fn isolated_profile_and_explicit_origin_are_selected_without_filesystem_side_effects() {
    let config = Config::from_lookup("https://LOOM.example:443/", |key| match key {
        "LOCALAPPDATA" => Some("C:/Fixture/Local".into()),
        "COMPUTERNAME" => Some(" 中文电脑 ".into()),
        // CA scope 不能覆盖所选服务地址。
        "OTTER_LOOM_TLS_ORIGIN" => Some("https://different.example".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(config.origin(), "https://loom.example");
    assert_eq!(
        config.identity_root(),
        Path::new("C:/Fixture/Local/OtterNative")
    );
    assert_eq!(config.name, "中文电脑");
    let config = Config::from_lookup("https://localhost", |key| {
        (key == "OTTER_NATIVE_PROFILE_DIR").then(|| "C:/Explicit".into())
    })
    .unwrap();
    assert_eq!(config.identity_root(), Path::new("C:/Explicit"));
    assert_eq!(config.name, "Otter 设备");
}

#[test]
fn invalid_profile_origin_and_name_fail_before_preparation() {
    assert!(matches!(
        Config::from_lookup("https://localhost", |_| None),
        Err(Error::Profile)
    ));
    assert!(matches!(
        Config::new("http://localhost", Path::new("C:/Fixture"), "PC"),
        Err(Error::Origin)
    ));
    assert!(matches!(
        Config::new("https://localhost/path", Path::new("C:/Fixture"), "PC"),
        Err(Error::Origin)
    ));
    assert!(matches!(
        Config::new("https://localhost", Path::new("relative"), "PC"),
        Err(Error::Profile)
    ));
    assert!(matches!(
        Config::new("https://localhost", Path::new("C:/Fixture"), "bad\nname"),
        Err(Error::Name)
    ));
}

#[test]
fn background_preparation_resolves_localhost_but_does_not_create_identity() {
    let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
        "otter-prepare-{:?}",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    let config = Config::new("https://localhost:8443", &root, "PC").unwrap();
    assert!(matches!(
        config.prepare(Action::Renew, &Cancel::new()),
        Ok(Task::Renew { .. })
    ));
    assert!(!root.exists());
    let cancel = Cancel::new();
    cancel.cancel();
    let config = Config::new("https://localhost", &root, "PC").unwrap();
    assert!(matches!(
        config.prepare(Action::RequestPairing, &cancel),
        Err(Error::Stopped)
    ));
    assert!(!root.exists());
}

#[test]
fn incomplete_canonical_tls_configuration_does_not_fall_back_to_legacy() {
    let config = Config::from_lookup("https://localhost", |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some("C:/Fixture".into()),
        "OTTER_LOOM_TLS_ORIGIN" => Some("".into()),
        "HOOK_LOOM_TLS_ORIGIN" => Some("https://localhost".into()),
        _ => None,
    })
    .unwrap();
    assert!(matches!(
        config.prepare(Action::Renew, &Cancel::new()),
        Err(Error::Trust)
    ));
}

//! 检查 RC 生成和暂存发布边界；实际 EXE 资源由 exe_resources 集成测试验证。
#[allow(dead_code)] // 真实编译器入口由 build.rs 调用。
#[path = "../../build_support/windows_resources.rs"]
mod resources;
use std::{fs, path::Path, process, time::SystemTime};

#[test]
fn pe_version_stays_public_and_icon_path_is_quoted_unicode() {
    let rc = resources::source("0.2.33", Path::new("C:\\有空格 目录\\icon.ico")).unwrap();
    assert!(rc.contains("1 ICON \"C:/有空格 目录/icon.ico\""));
    assert!(rc.contains("FILEVERSION 0,2,33,0"));
    assert!(rc.contains("PRODUCTVERSION 0,2,33,0"));
    assert!(rc.contains("VALUE \"ProductVersion\", \"0.2.33\""));
    assert!(rc.contains("#pragma code_page(65001)"));
    assert!(!rc.as_bytes().starts_with(&[0xef, 0xbb, 0xbf]));
}

#[test]
fn invalid_versions_and_resource_injection_paths_fail_closed() {
    for version in [
        "",
        "1.2",
        "1.2.3.4",
        "65536.0.0",
        "1.2.3-beta",
        "1.2.3\n1 ICON evil",
    ] {
        assert!(resources::source(version, Path::new("icon.ico")).is_err());
    }
    for icon in ["icon\".ico", "icon\n1 VERSIONINFO", "icon\0.ico"] {
        assert!(resources::source("0.2.33", Path::new(icon)).is_err());
    }
}

#[test]
fn reused_output_rejects_no_write_and_publishes_only_fresh_resource() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("otter-rc-test-{}-{nonce}", process::id()));
    fs::create_dir(&root).unwrap();
    let stable = root.join("otter-native.res");
    fs::write(&stable, b"old-good-resource").unwrap();

    let missing = resources::compile_with(&root, &root, "0.2.33", |_, stage| {
        assert_ne!(stage, stable);
        assert_eq!(fs::metadata(stage)?.len(), 0);
        Ok(())
    });
    assert_eq!(
        missing.unwrap_err().to_string(),
        "Resource compiler produced no .res file"
    );
    assert_eq!(fs::read(&stable).unwrap(), b"old-good-resource");

    let failed = resources::compile_with(&root, &root, "0.2.33", |_, stage| {
        fs::write(stage, b"partial-resource")?;
        Err(std::io::Error::other("compiler failed"))
    });
    assert_eq!(failed.unwrap_err().to_string(), "compiler failed");
    assert_eq!(fs::read(&stable).unwrap(), b"old-good-resource");

    let output = resources::compile_with(&root, &root, "0.2.33", |_, stage| {
        assert_eq!(fs::metadata(stage)?.len(), 0);
        fs::write(stage, b"fresh-resource")
    })
    .unwrap();
    assert_eq!(output, stable);
    assert_eq!(fs::read(output).unwrap(), b"fresh-resource");
}

//! 检查 RC 生成边界；实际资源编译/链接由 exe_resources 集成测试验证。
#[allow(dead_code)] // compile 由 build.rs 调用；本目标只验证纯文本生成。
#[path = "../build_support/windows_resources.rs"]
mod resources;
use std::path::Path;

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

//! 原生宿主与测试使用相同的 Common Controls v6 激活上下文。
#[path = "build_support/windows_resources.rs"]
mod windows_resources;
fn main() {
    println!("cargo:rerun-if-changed=windows.manifest");
    println!("cargo:rerun-if-changed=../src-tauri/icons/icon.ico");
    println!("cargo:rerun-if-env-changed=OTTER_NATIVE_RC");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        let manifest = std::path::PathBuf::from(
            std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo package directory"),
        )
        .join("windows.manifest");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        let root = manifest.parent().expect("Manifest parent");
        let out =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
        windows_resources::compile(root, &out, env!("CARGO_PKG_VERSION"))
            .expect("Compile native Windows icon and version resources");
    }
}

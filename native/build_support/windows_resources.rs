//! 构建期 PE 资源 owner；复用品牌图标，不引入 GUI 框架或运行时依赖。
use std::{env, ffi::OsString, fs, io, path::Path, process::Command};

pub fn compile(root: &Path, out: &Path, version: &str) -> io::Result<()> {
    let icon = root.join("../src-tauri/icons/icon.ico");
    let script = out.join("otter-native.rc");
    let resource = out.join("otter-native.res");
    fs::write(&script, source(version, &icon)?)?;
    let explicit = env::var_os("OTTER_NATIVE_RC");
    let compilers = explicit.map_or_else(
        || vec![OsString::from("rc.exe"), OsString::from("llvm-rc.exe")],
        |path| vec![path],
    );
    // 只有工具不存在才尝试下一工具；语法/资源错误不得被回退掩盖。
    for compiler in compilers {
        let output = Command::new(&compiler)
            .arg("/FO")
            .arg(&resource)
            .arg(&script)
            .current_dir(root)
            .output();
        match output {
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
            Ok(output) if !output.status.success() => {
                return Err(io::Error::other(format!(
                    "Resource compiler {compiler:?} failed: {}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
            Ok(_) => {
                if !resource.is_file() {
                    return Err(io::Error::other("Resource compiler produced no .res file"));
                }
                println!(
                    "cargo:rustc-link-arg-bin=otter-native={}",
                    resource.display()
                );
                return Ok(());
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Windows resource compiler unavailable: add rc.exe or llvm-rc.exe to PATH, or set OTTER_NATIVE_RC to its executable path",
    ))
}

pub fn source(version: &str, icon: &Path) -> io::Result<String> {
    let parts = version
        .split('.')
        .map(str::parse::<u16>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid package version"))?;
    if parts.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Expected three-part package version",
        ));
    }
    let icon = icon
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Non-Unicode icon path"))?;
    if icon.chars().any(|c| c.is_control() || c == '"') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid icon path",
        ));
    }
    let icon = icon.replace('\\', "/");
    let fixed = format!("{},{},{},0", parts[0], parts[1], parts[2]);
    // 三段公共 SemVer 是 PE 文件版本；内部四段身份由候选 provenance 绑定。
    // 常量取 Win32 VERSIONINFO 定义的数值，避免要求额外的 SDK include 环境。
    Ok(format!(
        r#"#pragma code_page(65001)
1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {fixed}
PRODUCTVERSION {fixed}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "Otter Windows Native"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "otter-native"
      VALUE "OriginalFilename", "otter-native.exe"
      VALUE "ProductName", "Otter"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
    ))
}

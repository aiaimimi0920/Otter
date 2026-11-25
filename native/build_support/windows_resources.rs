//! ??? PE ?? owner??????????? GUI ?????????
use std::{
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{self, Command},
};

pub fn compile(root: &Path, out: &Path, version: &str) -> io::Result<()> {
    let resource = compile_with(root, out, version, |script, staged| {
        run_compiler(root, script, staged)
    })?;
    println!(
        "cargo:rustc-link-arg-bin=otter-native={}",
        resource.display()
    );
    Ok(())
}

pub(super) fn compile_with(
    root: &Path,
    out: &Path,
    version: &str,
    run: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> io::Result<PathBuf> {
    let script = out.join("otter-native.rc");
    fs::write(
        &script,
        source(version, &root.join("assets/branding/icon.ico"))?,
    )?;
    let staged = reserve_output(out)?;
    run(&script, &staged)?;
    if !fs::metadata(&staged).is_ok_and(|file| file.is_file() && file.len() > 0) {
        return Err(io::Error::other("Resource compiler produced no .res file"));
    }
    // ??????????????????????? .res????????
    let resource = out.join("otter-native.res");
    fs::rename(staged, &resource)?;
    Ok(resource)
}

fn reserve_output(out: &Path) -> io::Result<PathBuf> {
    // create_new ??????????????????? PID ????????
    for attempt in 0..64 {
        let path = out.join(format!("otter-native-{}-{attempt}.res", process::id()));
        match fs::File::create_new(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "No fresh .res staging path",
    ))
}

fn run_compiler(root: &Path, script: &Path, staged: &Path) -> io::Result<()> {
    let explicit = env::var_os("OTTER_NATIVE_RC");
    let compilers = explicit.map_or_else(
        || vec![OsString::from("rc.exe"), OsString::from("llvm-rc.exe")],
        |path| vec![path],
    );
    // ?????????????????/????????????
    for compiler in compilers {
        let output = Command::new(&compiler)
            .arg("/FO")
            .arg(staged)
            .arg(script)
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
            Ok(_) => return Ok(()),
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

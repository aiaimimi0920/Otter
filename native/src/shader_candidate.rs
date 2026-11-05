//! 显式内部候选准入；代码 pin 不是许可/发行批准，不接受 CLI、环境或 Art 提供的摘要。
use crate::{
    shader_angle::{Fingerprint, Identity},
    shader_process::TrustedBundle,
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};
#[path = "shader_candidate_acl.rs"]
mod acl;
#[path = "shader_candidate_files.rs"]
mod files;

// v332 build.json ae24c4b8…，v333 materials.json 95322473… 的已复核本地构建。
// 更新必须同时复核构建/材料及开发侧运行；不能从候选目录自动导入新 pin。
fn identity() -> Identity {
    fn fingerprint(bytes: u64, hex: &str) -> Fingerprint {
        let mut sha256 = [0; 32];
        for (index, byte) in sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).expect("static pin");
        }
        Fingerprint { bytes, sha256 }
    }
    Identity {
        egl: fingerprint(
            199680,
            "fb538bb38dae68c612cfc309881143cdbd3ace9498e207f4cb2f6d38c3721639",
        ),
        gles: fingerprint(
            5295616,
            "beeb45725d5ef558eaaa1e3a222df31e9d53103c0ee154f6853380e2a362e5ff",
        ),
    }
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Shader candidate path, ACL, file set or pinned identity rejected",
    )
}
fn verify(file: &mut File, pin: Fingerprint) -> io::Result<()> {
    if file.metadata()?.len() != pin.bytes {
        return Err(invalid());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 16384];
    let mut remaining = pin.bytes;
    while remaining != 0 {
        let count = file.read(&mut buffer[..(remaining as usize).min(16384)])?;
        if count == 0 {
            return Err(invalid());
        }
        hash.update(&buffer[..count]);
        remaining -= count as u64;
    }
    if hash.finalize().as_slice() != pin.sha256 {
        return Err(invalid());
    }
    Ok(())
}
/// 不安装、复制或修改权限；调用者显式选择已准备且仅可信用户可写的两文件目录。
/// 保护其他本机用户，不将同用户恶意进程、管理员或系统视为可隔离的攻击者。
pub fn open(directory: &Path) -> io::Result<TrustedBundle> {
    let mut leases = files::directories(directory)?;
    acl::check(leases.last().ok_or_else(invalid)?)?;
    let entries: Vec<_> = std::fs::read_dir(directory)?
        .take(3)
        .collect::<io::Result<_>>()?;
    if entries.len() != 2
        || entries.iter().any(|entry| {
            !matches!(
                entry.file_name().to_str(),
                Some("libEGL.dll" | "libGLESv2.dll")
            )
        })
    {
        return Err(invalid());
    }
    let identity = identity();
    for (name, pin) in [
        ("libEGL.dll", identity.egl),
        ("libGLESv2.dll", identity.gles),
    ] {
        let mut file = files::binary(&directory.join(name))?;
        acl::check(&file)?;
        verify(&mut file, pin)?;
        leases.push(file);
    }
    Ok(TrustedBundle::new(directory, identity)?.with_files(leases))
}

// 固定候选在 child 重新取得租约；父异常退出时不依赖跨进程 Arc 的存活。
// 其他已有可信清单调用方仍沿原 wire 合同，不把该兼容分支作为候选授权入口。
#[cfg(test)]
pub(crate) fn from_worker(directory: &Path, supplied: Identity) -> io::Result<TrustedBundle> {
    let pinned = identity();
    if [(supplied.egl, pinned.egl), (supplied.gles, pinned.gles)]
        .iter()
        .all(|(a, b)| a.bytes == b.bytes && a.sha256 == b.sha256)
    {
        let plain = directory.to_str().and_then(|s| s.strip_prefix(r"\\?\"));
        let directory = plain.map(Path::new).unwrap_or(directory);
        open(directory)
    } else {
        TrustedBundle::new(directory, supplied)
    }
}

#[cfg(test)]
#[path = "shader_candidate_tests.rs"]
mod tests;

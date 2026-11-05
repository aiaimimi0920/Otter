//! 启动阶段只读候选准入；必须先于截图/建窗；可选 smoke peer 不读默认发现文件。
use super::super::{shader_prepare::Installation, Startup};
use crate::loom_bridge_identity::Identity;
use std::{fs::File, io::Read};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};
#[derive(Default)]
pub(in crate::desktop) struct Loaded {
    pub installation: Option<Installation>,
    pub peer: Option<Identity>,
}
pub(in crate::desktop) fn resolve(startup: &Startup<'_>, smoke: bool) -> Result<Loaded> {
    if startup.shader_smoke_peer.is_some() && !smoke {
        return Err(Error::new(
            E_INVALIDARG,
            "Shader smoke peer requires explicit smoke mode",
        ));
    }
    // 即使候选可信也不再打开目录；保留参数只用于给出明确迁移错误。
    if startup.shader_candidate.is_some() {
        return Err(Error::new(
            E_INVALIDARG,
            "Shader candidate is retired; native HLSL is required",
        ));
    }
    let installation = Some(Installation::native());
    let peer = startup
        .shader_smoke_peer
        .map(|path| {
            // 复用 Loom 发现格式/身份验证，但路径只来自显式 smoke 参数，不作默认发现。
            let mut bytes = Vec::new();
            File::open(path)
                .and_then(|file| {
                    file.take((crate::loom_discovery_manifest::MAX_BYTES + 1) as u64)
                        .read_to_end(&mut bytes)
                })
                .map_err(|_| {
                    Error::new(E_INVALIDARG, "Shader smoke peer manifest read rejected")
                })?;
            crate::loom_discovery_manifest::parse(&bytes)
                .map_err(|_| Error::new(E_INVALIDARG, "Shader smoke peer identity rejected"))
        })
        .transpose()?;
    Ok(Loaded { installation, peer })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_installation_uses_native_without_a_dll_candidate() {
        let loaded = resolve(&Startup::default(), false).unwrap();
        assert!(loaded.installation.unwrap().bundle.is_none());
        assert!(loaded.peer.is_none());
    }
    #[test]
    fn invalid_explicit_legacy_candidate_does_not_silently_become_native() {
        let path = std::path::Path::new("missing-explicit-shader-candidate.json");
        assert!(resolve(
            &Startup {
                shader_candidate: Some(path),
                ..Default::default()
            },
            false
        )
        .is_err());
    }
}

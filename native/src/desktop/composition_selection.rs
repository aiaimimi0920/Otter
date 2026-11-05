//! 呈现链创建事务：只有完整候选可发布；失败候选先释放，才尝试下一个实际已实现后端。
use super::super::composition_creation;
use windows::core::{Error, HRESULT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Backend {
    D3d11Hardware,
    D3d11Warp,
}
impl Backend {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::D3d11Hardware => "d3d11-hardware",
            Self::D3d11Warp => "d3d11-warp",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stage {
    Device,
    Presentation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Failure {
    pub backend: Backend,
    pub stage: Stage,
    pub code: HRESULT,
}
pub(super) struct Selected<T> {
    pub value: T,
    pub backend: Backend,
    pub fallback: Option<Failure>,
}
pub(super) fn select<T>(
    mut create: impl FnMut(Backend) -> std::result::Result<T, (Stage, composition_creation::Failure)>,
) -> composition_creation::Result<Selected<T>> {
    // 本层只负责 D3D11 的硬件/WARP 兼容链；Canvas 的 D3D12 优先选择由 RenderSurface 拥有。
    let mut first = None;
    for backend in [Backend::D3d11Hardware, Backend::D3d11Warp] {
        match create(backend) {
            Ok(value) => {
                return Ok(Selected {
                    value,
                    backend,
                    fallback: first,
                })
            }
            Err((stage, creation)) => {
                let allowed = creation.fallback_allowed();
                if !allowed {
                    eprintln!("native-presentation-fallback: selected=none rejected={} fallback=denied reason=unconfirmed-association", backend.label());
                    return Err(creation);
                }
                let error = creation.into_error();
                let failure = Failure {
                    backend,
                    stage,
                    code: error.code(),
                };
                if let Some(previous) = first {
                    return Err(Error::new(
                        error.code(),
                        format!("Native presentation unavailable: {previous:?}; {failure:?}"),
                    )
                    .into());
                }
                first = Some(failure);
            }
        }
    }
    unreachable!("finite nonempty backend sequence returns success or final failure")
}

#[cfg(test)]
#[path = "composition_selection_tests.rs"]
mod tests;

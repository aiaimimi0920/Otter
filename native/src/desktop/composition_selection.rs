//! Presentation-chain creation transaction: publish complete candidates only; release failed candidates before trying the next implemented backend.
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
    // This layer owns only the D3D11 hardware/WARP compatibility chain; RenderSurface owns Canvas's D3D12 preference.
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "composition_selection_tests.rs"]
mod tests;

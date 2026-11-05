//! Distinct server Surface attachments and online native Art proofs; no fake attachment IDs.
use super::*;
use crate::live_relay_http::art_prepare::viewer::Proof;

#[derive(Clone)]
pub(crate) enum ViewerHost {
    Surface(Binding),
    LocalArt(Arc<Proof>),
}
impl ViewerHost {
    pub(crate) fn origin(&self) -> &str {
        match self {
            Self::Surface(binding) => binding.origin(),
            Self::LocalArt(proof) => &proof.binding().origin,
        }
    }
    pub(crate) fn device(&self) -> &str {
        match self {
            Self::Surface(binding) => binding.device(),
            Self::LocalArt(proof) => &proof.binding().device_id,
        }
    }
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Surface(binding) => binding.node(),
            Self::LocalArt(proof) => &proof.binding().art_id,
        }
    }
    pub(crate) fn same_host(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Surface(left), Self::Surface(right)) => same_binding(left, right),
            (Self::LocalArt(left), Self::LocalArt(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }
    pub(crate) fn kind(&self) -> &str {
        match self {
            Self::Surface(_) => "Surface",
            Self::LocalArt(_) => "Art",
        }
    }
}
impl PartialEq for ViewerHost {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Surface(left), Self::Surface(right)) => left == right,
            (Self::LocalArt(left), Self::LocalArt(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }
}

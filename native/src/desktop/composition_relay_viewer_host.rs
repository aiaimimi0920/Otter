//! Distinct server Surface attachments and online native Art proofs; no fake attachment IDs.
use super::*;
use crate::live_relay_http::art_prepare::viewer::Proof;

#[derive(Clone)]
pub(crate) enum ViewerHost {
    Surface(Binding),
    LocalArt(Arc<Proof>),
    // Only a request selection, never an attachment or a pending host binding.
    Direct { origin: String, device: String },
}
impl ViewerHost {
    pub(crate) fn origin(&self) -> &str {
        match self {
            Self::Surface(binding) => binding.origin(),
            Self::LocalArt(proof) => &proof.binding().origin,
            Self::Direct { origin, .. } => origin,
        }
    }
    pub(crate) fn device(&self) -> &str {
        match self {
            Self::Surface(binding) => binding.device(),
            Self::LocalArt(proof) => &proof.binding().device_id,
            Self::Direct { device, .. } => device,
        }
    }
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Surface(binding) => binding.node(),
            Self::LocalArt(proof) => &proof.binding().art_id,
            Self::Direct { device, .. } => device,
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
            Self::Direct { .. } => "Live",
        }
    }
}
impl PartialEq for ViewerHost {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Surface(left), Self::Surface(right)) => left == right,
            (Self::LocalArt(left), Self::LocalArt(right)) => Arc::ptr_eq(left, right),
            (
                Self::Direct {
                    origin: a,
                    device: b,
                },
                Self::Direct {
                    origin: c,
                    device: d,
                },
            ) => a == c && b == d,
            _ => false,
        }
    }
}

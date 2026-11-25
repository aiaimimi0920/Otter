//! Account-projection UI holds only public associations and explicit requests; existing local Loom/workspaces own accounts, networking, and persistence.
use super::*;
use crate::projection_account::{scene::Binding, Envelope};
use std::sync::{atomic::AtomicBool, Arc};

pub(in crate::desktop) enum Request {
    Create(share::SourceSelection),
    Import {
        envelope: Envelope,
        cancel: Arc<AtomicBool>,
    },
    Stop(Binding),
    Recover(crate::projection_account::journal::Intent),
}
#[derive(Default)]
pub(in crate::desktop) struct View {
    pub bindings: Vec<Binding>,
    pub busy: bool,
    pub status: String,
    pub received: Option<Envelope>,
}

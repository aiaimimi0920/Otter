//! Pure merging of same-document responses/local views; no disk/network/checkpoint acknowledgement or silent same-object conflict rebase.
use super::*;
use crate::projection_edit::Code;

#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    code: Code,
    changes: Vec<Change>,
}
impl Conflict {
    pub fn code(&self) -> Code {
        self.code
    }
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Reconciliation {
    authoritative: View,
    visible: View,
    changes: Vec<Change>,
    conflict: Option<Conflict>,
}
impl Reconciliation {
    /// Upper layers must persist this authoritative baseline, then reobserve local view before applying; this result issues no disk receipt.
    pub fn authoritative(&self) -> &View {
        &self.authoritative
    }
    pub fn visible(&self) -> &View {
        &self.visible
    }
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }
    pub fn conflict(&self) -> Option<&Conflict> {
        self.conflict.as_ref()
    }
}

/// observed is the acknowledged baseline, with apply responses tied to original pending views; reread current after every asynchronous save.
/// Calling this pure function again includes intervening edits; Application still owns cancellation/host identity/actual checkpoint.
pub fn reconcile(
    previous: &Document,
    observed: &View,
    document: &Document,
    current: &View,
    sticky: Option<Code>,
) -> Result<Reconciliation, Error> {
    previous.validate().map_err(Error::Protocol)?;
    document.validate().map_err(Error::Protocol)?;
    document.follows(previous).map_err(Error::Protocol)?;
    if observed.basis() != previous.basis() || current.basis() != previous.basis() {
        return Err(Error::Protocol(super::super::Error::Basis));
    }
    let changes = observed.changes_to(current)?;
    let remote = View::from_document(document, current.size())?;
    let race = changes.iter().any(|change| {
        !equal_optional(
            observed.basis_objects().get(&change.object_id),
            remote.basis_objects().get(&change.object_id),
        )
    });
    // current is the sole complete local truth; do not overlay old conflict payloads again or undo will revive old edits.
    let visible = remote.overlay(&changes)?;
    let code = sticky.or(race.then_some(Code::ObjectConflict));
    let conflict = code.map(|code| Conflict {
        code,
        changes: changes.clone(),
    });
    Ok(Reconciliation {
        authoritative: remote,
        visible,
        changes,
        conflict,
    })
}

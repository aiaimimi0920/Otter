//! Single-field asynchronous draft; separate authoritative-value updates from request completion without prematurely overwriting local edits during submission.
use std::{collections::BTreeSet, rc::Rc};

pub const MAX_VALUE_BYTES: usize = 48 * 1024;
pub const MAX_PENDING: usize = 64;
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Value,
    Pending,
}
#[derive(Clone)]
pub struct Ticket {
    scope: Rc<()>,
    sequence: u64,
}
pub struct Draft {
    scope: Rc<()>,
    sequence: u64,
    source: String,
    value: String,
    pending: BTreeSet<u64>,
}
fn validate(value: &str) -> Result<(), Error> {
    if value.len() > MAX_VALUE_BYTES || value.contains('\0') {
        Err(Error::Value)
    } else {
        Ok(())
    }
}
impl Draft {
    pub fn new(source: &str) -> Result<Self, Error> {
        validate(source)?;
        Ok(Self {
            scope: Rc::new(()),
            sequence: 0,
            source: source.to_owned(),
            value: source.to_owned(),
            pending: BTreeSet::new(),
        })
    }
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn pending(&self) -> usize {
        self.pending.len()
    }
    pub fn edit(&mut self, value: &str) -> Result<(), Error> {
        validate(value)?;
        self.value = value.to_owned();
        Ok(())
    }
    /// Within the same scope, an unchanged authoritative string must not overwrite local input without an asynchronous handler.
    pub fn synchronize(&mut self, source: &str) -> Result<(), Error> {
        validate(source)?;
        if source != self.source {
            self.source = source.to_owned();
            if self.pending.is_empty() {
                self.value.clone_from(&self.source);
            }
        }
        Ok(())
    }
    /// Reserve capacity before asynchronous dispatch; cancel if dispatch never starts, and settle when the actual result returns. This is not action permission.
    pub fn begin(&mut self) -> Result<Ticket, Error> {
        if self.pending.len() >= MAX_PENDING {
            return Err(Error::Pending);
        }
        let sequence = self.sequence.checked_add(1).ok_or(Error::Pending)?;
        self.pending.insert(sequence);
        self.sequence = sequence;
        Ok(Ticket {
            scope: self.scope.clone(),
            sequence,
        })
    }
    /// Old-scope, foreign, or duplicate replies have no effect; adopt the latest authoritative value only after all current requests settle.
    pub fn settle(&mut self, ticket: Ticket) -> bool {
        if !self.cancel(ticket) {
            return false;
        }
        if self.pending.is_empty() {
            self.value.clone_from(&self.source);
        }
        true
    }
    /// Release reservations for tasks that did not start successfully, without rolling back the user's unsubmitted local value to an old authoritative value.
    pub fn cancel(&mut self, ticket: Ticket) -> bool {
        Rc::ptr_eq(&self.scope, &ticket.scope) && self.pending.remove(&ticket.sequence)
    }
    /// The upper layer explicitly resets when binding/node identity changes; a revision change alone must not reset it.
    pub fn reset(&mut self, source: &str) -> Result<(), Error> {
        let next = Self::new(source)?;
        *self = next;
        Ok(())
    }
}
#[cfg(test)]
#[path = "surface_input_draft_tests.rs"]
mod tests;

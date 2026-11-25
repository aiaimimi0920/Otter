//! Source staging precedes all PNG CAS; reopen only restores, without automatic networking or overwriting old session/initial/unknown pending.
use super::super::source::Frozen;
use super::*;
impl Store {
    pub(crate) fn stage_source(
        &self,
        context: Context,
        frozen: Frozen,
        cancel: &AtomicBool,
    ) -> Result<Frozen, Error> {
        context.validate()?;
        frozen.validate().map_err(|_| Error::Format)?;
        let mut state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        if let Some(old) = state
            .entries
            .iter()
            .find(|entry| entry.context.same_key(&context))
        {
            if old.context != context {
                return Err(Error::Changed);
            }
            let source = old.source.as_ref().ok_or(Error::Changed)?;
            if !source.same_basis(&frozen) {
                return Err(Error::Changed);
            }
            return Ok(source.clone()); // Never reconstruct original anchor/session/initial because current Editor changed before retry.
        }
        if state.entries.len() >= MAX_ENTRIES {
            return Err(Error::Limit);
        }
        let mut entries = state.entries.clone();
        entries.push(Entry {
            context,
            generation: 1,
            document: None,
            pending: None,
            source: Some(frozen.clone()),
        });
        self.persist(&mut state, entries, cancel)?;
        Ok(frozen)
    }
    pub(crate) fn source(
        &self,
        context: &Context,
        cancel: &AtomicBool,
    ) -> Result<Option<Frozen>, Error> {
        let state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        let Some(entry) = state
            .entries
            .iter()
            .find(|entry| entry.context.same_key(context))
        else {
            return Ok(None);
        };
        if entry.context != *context {
            return Err(Error::Changed);
        }
        Ok(entry.source.clone())
    }
    /// Read both inputs to the source probe under one verified journal baseline.
    pub(crate) fn source_state(
        &self,
        context: &Context,
        cancel: &AtomicBool,
    ) -> Result<(Option<Document>, Option<Ticket>), Error> {
        let state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        let Some(entry) = state
            .entries
            .iter()
            .find(|entry| entry.context.same_key(context))
        else {
            return Ok((None, None));
        };
        let document = (entry.context == *context)
            .then(|| entry.document.clone())
            .flatten();
        let pending = entry.pending.as_ref().map(|_| Ticket {
            store: self.clone(),
            entry: entry.clone(),
        });
        Ok((document, pending))
    }
}

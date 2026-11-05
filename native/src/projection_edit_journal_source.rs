//! 源端stage先于任何PNG CAS；重开只恢复，不自动发网，不覆盖旧session/initial或未知pending。
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
        let mut entries = state.entries.clone();
        if let Some(old) = entries
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
            return Ok(source.clone()); // 原anchor/session/initial不可因重试时当前Editor变化而重造。
        }
        if entries.len() >= MAX_ENTRIES {
            return Err(Error::Limit);
        }
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
    pub(crate) fn source_document(
        &self,
        context: &Context,
        cancel: &AtomicBool,
    ) -> Result<Option<Document>, Error> {
        let state = self.0.state.lock().map_err(|_| Error::Changed)?;
        self.baseline(&state, cancel)?;
        Ok(state
            .entries
            .iter()
            .find(|entry| entry.context == *context)
            .and_then(|entry| entry.document.clone()))
    }
}

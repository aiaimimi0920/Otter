//! Explicit settings overwrite: full-byte baseline, unknown-field preservation, and failure recovery; two renames are not called atomic.
use crate::{
    file_naming_settings::Settings,
    session_file_replace::{Failure, Published, Replacement},
    settings_source::Source,
};
use std::sync::{atomic::AtomicBool, Arc};

pub struct Prepared {
    transaction: Replacement,
    next: Arc<Source>,
}
impl Prepared {
    /// Explicitly confirmed issuer settings; reuse the original settings replacement transaction/full-byte baseline without another identity-file write.
    pub fn prepare_issuer_trust(
        source: &Source,
        trust: crate::art_issuer_trust::Document,
        cancel: &AtomicBool,
    ) -> Result<Self, String> {
        if source.recovery_target().is_some() {
            return Err("Recovered settings require Save As".into());
        }
        let mut document = (**source.document()).clone();
        document.set_art_issuer_trust(trust)?;
        Self::prepare_document(source, document, cancel)
    }
    pub fn prepare(source: &Source, naming: Settings, cancel: &AtomicBool) -> Result<Self, String> {
        Self::prepare_fields(source, naming, None, None, cancel)
    }
    pub(crate) fn prepare_with_policy(
        source: &Source,
        naming: Settings,
        policy: crate::cache_policy::Policy,
        recycle: crate::session_library::recycle::policy::Policy,
        cancel: &AtomicBool,
    ) -> Result<Self, String> {
        Self::prepare_fields(source, naming, Some(policy), Some(recycle), cancel)
    }
    fn prepare_fields(
        source: &Source,
        naming: Settings,
        policy: Option<crate::cache_policy::Policy>,
        recycle: Option<crate::session_library::recycle::policy::Policy>,
        cancel: &AtomicBool,
    ) -> Result<Self, String> {
        if source.recovery_target().is_some() {
            return Err(
                "Recovered settings require Save As; recovery files are not update targets".into(),
            );
        }
        let mut document = (**source.document()).clone();
        document.set_naming(naming)?;
        if let Some(policy) = policy {
            document.set_cache_policy(policy)?;
        }
        if let Some(policy) = recycle {
            document.set_recycle_policy(policy)?;
        }
        Self::prepare_document(source, document, cancel)
    }
    pub(crate) fn prepare_document(
        source: &Source,
        document: crate::app_settings_document::Document,
        cancel: &AtomicBool,
    ) -> Result<Self, String> {
        if source.recovery_target().is_some() {
            return Err("Recovered settings require Save As".into());
        }
        let next = Source::from_document(source.path().to_path_buf(), document)?;
        let transaction =
            Replacement::prepare_bytes(source.path(), source.bytes(), next.bytes().clone(), cancel)
                .map_err(|e| e.to_string())?;
        Ok(Self { transaction, next })
    }
    /// Return a new baseline only after successful publication; UI must present warnings about retained recovery files.
    pub fn commit(self, cancel: &AtomicBool) -> Result<(Arc<Source>, Published), Failure> {
        let published = self.transaction.commit(cancel)?;
        Ok((self.next, published))
    }
    pub(crate) fn commit_checked(
        self,
        cancel: &AtomicBool,
        authorize: impl FnOnce() -> std::io::Result<()>,
    ) -> Result<(Arc<Source>, Published), Failure> {
        let published = self.transaction.commit_checked(cancel, authorize)?;
        Ok((self.next, published))
    }
}

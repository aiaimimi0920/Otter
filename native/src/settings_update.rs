//! 显式设置覆盖更新：完整字节基线、保留未知字段、失败恢复，不将双 rename 称为原子操作。
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
    /// 明确确认的 issuer 设置；复用原设置替换事务和完整字节基线，不另写身份文件。
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
    fn prepare_document(
        source: &Source,
        document: crate::app_settings_document::Document,
        cancel: &AtomicBool,
    ) -> Result<Self, String> {
        let next = Source::from_document(source.path().to_path_buf(), document)?;
        let transaction =
            Replacement::prepare_bytes(source.path(), source.bytes(), next.bytes().clone(), cancel)
                .map_err(|e| e.to_string())?;
        Ok(Self { transaction, next })
    }
    /// 只有成功发布才返回新基线；保留恢复文件的警告必须交给 UI 呈现。
    pub fn commit(self, cancel: &AtomicBool) -> Result<(Arc<Source>, Published), Failure> {
        let published = self.transaction.commit(cancel)?;
        Ok((self.next, published))
    }
}

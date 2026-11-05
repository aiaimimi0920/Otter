//! 普通Source共享远程四槽/编码预算和退出等待；停止未join仍占来源身份。
use super::*;
use crate::{
    live_relay_http::source::Publication,
    live_relay_source_runtime::{Outcome, Runtime},
    workspace_id::UnitId,
};
pub(crate) struct SourceCapture {
    pub publication: Publication,
    pub consumer: crate::live_frames::Consumer<crate::live_texture::Texture>,
    pub interaction: Option<Arc<crate::live_relay_source_runtime::Interaction>>,
    pub observation: Option<Arc<crate::live_relay_source_runtime::Observation>>,
}
pub(super) struct SourceEntry {
    origin: String,
    unit: UnitId,
    identity: Arc<()>,
    #[cfg(test)]
    session: String,
    runtime: Runtime,
}
pub(crate) struct SourceEvent {
    pub unit: UnitId,
    pub identity: Arc<()>,
    pub outcome: Outcome,
}
impl Sessions {
    pub(crate) fn start_source(
        &mut self,
        config: Config,
        lease: Lease,
        unit: UnitId,
        identity: Arc<()>,
        capture: SourceCapture,
    ) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if self
            .sources
            .iter()
            .any(|entry| entry.unit == unit || Arc::ptr_eq(&entry.identity, &identity))
        {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        if self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.sources.try_reserve(1).map_err(io::Error::other)?;
        let origin = config.origin().into();
        #[cfg(test)]
        let session = capture.publication.session().to_owned();
        let runtime = Runtime::start_with_observation(
            config,
            lease,
            capture.publication,
            capture.consumer,
            viewers::encoded_budget(),
            capture.interaction,
            capture.observation,
        )?;
        self.sources.push(SourceEntry {
            origin,
            unit,
            identity,
            #[cfg(test)]
            session,
            runtime,
        });
        Ok(())
    }
    pub(crate) fn source_status(&self, identity: &Arc<()>) -> Option<(bool, bool)> {
        self.sources
            .iter()
            .find(|entry| Arc::ptr_eq(&entry.identity, identity))
            .map(|entry| (entry.runtime.ready(), entry.runtime.stopping()))
    }
    pub(crate) fn source_recovering(&self, identity: &Arc<()>) -> bool {
        self.sources
            .iter()
            .find(|e| Arc::ptr_eq(&e.identity, identity))
            .is_some_and(|e| e.runtime.recovering())
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_source(&self, identity: &Arc<()>) -> bool {
        self.sources
            .iter()
            .find(|e| Arc::ptr_eq(&e.identity, identity))
            .is_some_and(|e| e.runtime.test_disconnect_media())
    }
    #[cfg(test)]
    pub(crate) fn test_disconnect_source_control(&self, identity: &Arc<()>) -> bool {
        self.sources
            .iter()
            .find(|e| Arc::ptr_eq(&e.identity, identity))
            .is_some_and(|e| e.runtime.test_disconnect_control())
    }
    #[cfg(test)]
    pub(crate) fn source_session(&self, identity: &Arc<()>) -> Option<&str> {
        self.sources
            .iter()
            .find(|entry| Arc::ptr_eq(&entry.identity, identity))
            .map(|entry| entry.session.as_str())
    }
    pub(crate) fn cancel_sources(&self, mut keep: impl FnMut(&str, &UnitId, &Arc<()>) -> bool) {
        for entry in &self.sources {
            if !keep(&entry.origin, &entry.unit, &entry.identity) {
                entry.runtime.request_stop();
            }
        }
    }
    pub(crate) fn stop_source(&self, identity: &Arc<()>) -> bool {
        let Some(entry) = self
            .sources
            .iter()
            .find(|entry| Arc::ptr_eq(&entry.identity, identity))
        else {
            return false;
        };
        entry.runtime.request_stop();
        true
    }
    pub(crate) fn poll_sources(&mut self) -> Vec<SourceEvent> {
        let mut result = Vec::new();
        let mut index = 0;
        while index < self.sources.len() {
            if let Some(outcome) = self.sources[index].runtime.try_join() {
                let entry = self.sources.swap_remove(index);
                result.push(SourceEvent {
                    unit: entry.unit,
                    identity: entry.identity,
                    outcome,
                });
            } else {
                index += 1;
            }
        }
        result
    }
    pub(super) fn source_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        self.sources
            .iter()
            .flat_map(|entry| entry.runtime.wait_handles())
    }
}

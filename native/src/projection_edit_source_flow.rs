//! 源端单目标probe→原PNG CAS→initialize/attach；每次沿原Job/Slot，精确Checkpoint前不退休。
use super::*;
use crate::projection_delivery::sender::{
    self,
    binding::{Binding, Candidate},
    Active, Request as SenderRequest,
};
use protocol::{
    journal::Context,
    source::{Frozen, Layout},
    Operation,
};
pub(crate) type Preparation = Box<dyn FnOnce(&Lease, &str, &str) -> Result<Frozen, Error> + Send>;
pub(crate) struct Target {
    pub request: SenderRequest,
    pub binding: Binding,
}
pub(crate) struct Input {
    pub root: PathBuf,
    pub context: Context,
    pub targets: Vec<Target>,
    pub source: crate::session_source::Source,
    pub prepare: Preparation,
    pub explicit: bool,
    pub cached: bool,
    pub expected_session: Option<String>,
}
pub(crate) struct Ready {
    pub context: Context,
    pub frozen: Frozen,
    pub layout: Layout,
    pub target: Binding,
    pub candidate: Option<Candidate>,
    _active: Active,
}
impl Ready {
    pub(super) fn check(&self) -> Result<(), sender::Error> {
        self._active.check()
    }
}
impl Job {
    pub(crate) fn start_edit_source(
        config: Config,
        lease: Lease,
        input: Input,
    ) -> io::Result<Self> {
        Self::spawn_admitted(
            Connection::Configured(config),
            Arc::new(lease),
            Work::EditSource(Box::new(input)),
            Some(Slot::acquire()?),
            None,
        )
    }
}
impl Input {
    pub(in crate::projection_delivery_runtime) fn run(
        mut self,
        control: &crate::live_relay_http::Control<'_>,
        lease: &Arc<Lease>,
    ) -> Result<Completed, Error> {
        let cancel = lease.preparation_cancel();
        if self.context.device() != lease.device_id()
            || self.targets.is_empty()
            || self.targets.len() > 8
        {
            return Err(identity());
        }
        let store = Store::open(&self.root, cancel).map_err(journal)?;
        let anchor = self.targets[0].binding.envelope().projection_id();
        let session = format!(
            "edit:{}",
            &crate::session_projection_scope::Scope::generate()
                .map_err(|_| identity())?
                .to_string()[..32]
        );
        let rendered = if self.cached {
            store
                .source(&self.context, cancel)
                .map_err(journal)?
                .ok_or_else(identity)?
        } else {
            (self.prepare)(lease, anchor, &session)?
        };
        let layout = rendered.layout();
        let frozen = store
            .stage_source(self.context.clone(), rendered, cancel)
            .map_err(journal)?;
        if self
            .expected_session
            .as_ref()
            .is_some_and(|session| session != frozen.session())
        {
            return Err(identity());
        }
        let previous = store
            .source_document(&self.context, cancel)
            .map_err(journal)?;
        let pending = store
            .pending(cancel)
            .map_err(journal)?
            .into_iter()
            .find(|ticket| ticket.context().same_key(&self.context));
        if pending.is_some() && !self.explicit {
            return Err(journal(protocol::journal::Error::Busy));
        }
        let wanted = pending
            .as_ref()
            .map(|ticket| ticket.request().projection_id())
            .unwrap_or_else(|| {
                if previous.is_none() {
                    frozen.anchor()
                } else {
                    self.targets[0].binding.envelope().projection_id()
                }
            })
            .to_owned();
        let index = self
            .targets
            .iter()
            .position(|target| target.binding.envelope().projection_id() == wanted)
            .ok_or_else(identity)?;
        let target = self.targets.swap_remove(index);
        if target.binding.stopped()
            || !target.binding.matches_intent(target.request.intent())
            || target.binding.unit() != self.context.unit()
            || target.binding.scope() != self.context.scope().to_string()
            || target.binding.envelope().origin() != self.context.origin()
        {
            return Err(identity());
        }
        Binding::admit_update(&self.source, &target.binding).map_err(Error::Sender)?;
        let active = target
            .request
            .activate(lease.clone(), false)
            .map_err(Error::Sender)?;
        let mut response = control
            .read_sender_update(&active, &target.binding)
            .map_err(Error::Http)?;
        let route = target.binding.edit_route();
        let mut observed = response.editing().cloned();
        if let Some(doc) = &observed {
            validate(doc, &frozen, previous.as_ref())?;
        }
        let ticket = if let Some(ticket) = pending {
            if ticket.context() != &self.context
                || ticket.route() != route
                || !matches!(
                    ticket.request().operation(),
                    Operation::Initialize { .. } | Operation::Attach { .. }
                )
                || response.digest() != frozen.basis().digest()
            {
                return Err(identity());
            }
            Some(ticket) // 只重放精确原请求；不重新CAS、不重造initial。
        } else {
            if observed.is_none() {
                match control.read_projection_edit(&self.context, &wanted, route, previous.as_ref())
                {
                    Ok(doc) => {
                        validate(&doc, &frozen, previous.as_ref())?;
                        observed = Some(doc);
                    }
                    Err(Failure::Rejected {
                        status: 404,
                        code: protocol::Code::NotFound,
                    }) => {
                        if let Some(update) = sender::update::Update::prepare(
                            frozen
                                .frame(sender::Kind::Sticker, cancel)
                                .map_err(Error::Sender)?,
                            &response,
                            cancel,
                        )
                        .map_err(Error::Sender)?
                        {
                            response = control
                                .update_sender(&active, &target.binding, &update)
                                .map_err(Error::Http)?;
                        }
                    }
                    Err(error) => return Err(Error::Edit(error)),
                }
            }
            if response.digest() != frozen.basis().digest() {
                return Err(identity());
            }
            // 兼容未在普通read内携带editing的服务；probe已确认的会话只读恢复，不重复写入。
            if observed.is_some() {
                None
            } else {
                let operation = if wanted == frozen.anchor() {
                    Operation::Initialize {
                        projection_id: wanted.clone(),
                        session_id: frozen.session().into(),
                        expected_digest: frozen.basis().digest().into(),
                        objects: frozen.initial().clone(),
                    }
                } else {
                    if previous.is_none() {
                        return Err(identity());
                    }
                    Operation::Attach {
                        projection_id: wanted.clone(),
                        session_id: frozen.session().into(),
                    }
                };
                let request =
                    Request::prepare(operation).map_err(|e| Error::Edit(Failure::Protocol(e)))?;
                Some(
                    store
                        .begin(
                            self.context.clone(),
                            route,
                            request,
                            previous.clone(),
                            frozen.initial().clone(),
                            cancel,
                        )
                        .map_err(journal)?,
                )
            }
        };
        let document = if let Some(ticket) = &ticket {
            control.projection_edit(ticket).map_err(Error::Edit)?
        } else {
            observed.ok_or_else(identity)?
        };
        validate(&document, &frozen, previous.as_ref())?;
        active.check().map_err(Error::Sender)?;
        let candidate = if response.revision() == target.binding.revision()
            && response.digest() == target.binding.digest()
            && response.receiver_device().is_some() == target.binding.linked()
        {
            None
        } else {
            Some(
                Binding::prepare(
                    &self.source,
                    target.request.intent(),
                    &response,
                    target.binding.name(),
                    cancel,
                )
                .map_err(Error::Sender)?,
            )
        };
        Ok(Completed::Metadata(Output::Edited(Box::new(
            super::Ready {
                ticket,
                document,
                resolution: None,
                rejection: None,
                source: Some(Ready {
                    context: self.context,
                    frozen,
                    layout,
                    target: target.binding,
                    candidate,
                    _active: active,
                }),
                _store: store,
            },
        ))))
    }
}
fn journal(error: protocol::journal::Error) -> Error {
    Error::Edit(Failure::Journal(error))
}
fn identity() -> Error {
    Error::Edit(Failure::Protocol(protocol::Error::Identity))
}
fn validate(
    document: &Document,
    frozen: &Frozen,
    previous: Option<&Document>,
) -> Result<(), Error> {
    if document.session_id() != frozen.session()
        || document.basis() != &frozen.basis()
        || previous.is_some_and(|old| document.follows(old).is_err())
    {
        return Err(identity());
    }
    Ok(())
}

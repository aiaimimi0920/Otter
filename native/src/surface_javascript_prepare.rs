//! Job 与持久后台 owner 共用的同步取码/求值边界；调用者必须在后台持有原请求租约。
use super::*;
use crate::{device_authorization_config::Config, device_session_client::Client};
pub(super) enum Connection {
    Configured(Config),
    Prepared(Client),
}
pub(super) fn prepare(
    connection: Connection,
    task: Arc<Lease>,
    current: &MountedSurface,
    viewport: Option<crate::surface_viewport::Viewport>,
) -> Result<Mounted, Error> {
    task.validate(current.binding.origin(), now()?)
        .map_err(|_| Error::Stopped)?;
    if task.device_id() != current.binding.device() {
        return Err(Error::Stopped);
    }
    let entry = current.javascript_entry(now()?).map_err(Error::Source)?;
    let client = match connection {
        Connection::Configured(config) => {
            task.prepare_client(&config).map_err(|_| Error::Stopped)?
        }
        Connection::Prepared(client) => client,
    };
    let source = task
        .control(&client)
        .map_err(|_| Error::Stopped)?
        .fetch_javascript_entry(entry)
        .map_err(Error::Source)?;
    source
        .entry
        .current(current, now()?, false)
        .map_err(Error::Source)?;
    task.validate(current.binding.origin(), now()?)
        .map_err(|_| Error::Stopped)?;
    let text =
        std::str::from_utf8(&source.bytes).map_err(|_| Error::Source(ResourceError::Input))?;
    let value = serde_json::from_str(current.snapshot.get())
        .map_err(|_| Error::Source(ResourceError::Input))?;
    let (session, frame) = Session::mount_in(
        current.binding.clone(),
        text.to_owned(),
        value,
        serde_json::json!({}),
        viewport,
        &|| stopped(&task, source.entry.expires_at_ms()),
    )
    .map_err(execution)?;
    let mut mounted = Mounted {
        notice: task.revocation_notification().map_err(execution)?,
        lease: task,
        entry: source.entry,
        session: Some(session),
        initial: Some(frame),
    };
    mounted.refresh(current)?;
    if mounted
        .initial
        .as_ref()
        .is_some_and(|frame| !frame.events.is_empty())
        && current.binding.lifecycle() != Lifecycle::Active
    {
        return Err(Error::Inactive);
    }
    Ok(mounted)
}

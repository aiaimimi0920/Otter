//! Session initial-admission/resource-installation transaction; local Session reaps the original Job on failure and never replays a partial protocol stream.
use super::*;
impl Session {
    pub(super) fn mount_options(
        binding: Binding,
        source: String,
        value: Value,
        assets: Value,
        options: services::Options,
        stopped: &impl Fn() -> bool,
        spawn: impl FnOnce(std::os::windows::io::OwnedHandle, Instant) -> io::Result<Child>,
    ) -> io::Result<(Self, Frame)> {
        let services::Options {
            viewport,
            dispatcher,
            allow_results,
            images,
        } = options;
        let (authority, revision) = snapshot(&value)?;
        resources(&assets)?;
        if let Some(view) = viewport {
            view.validate().map_err(|_| wire::invalid())?;
        }
        if source.is_empty() || source.len() > wire::MAX_SOURCE || source.contains('\0') {
            return Err(wire::invalid());
        }
        let headers = if images.is_empty() {
            None
        } else {
            Some(wire::images::headers(&images)?)
        };
        let deadline = Instant::now() + TIMEOUT;
        let (pipe, endpoint) = Pipe::pair()?;
        let child = spawn(endpoint.into_handle(), deadline)?;
        let mut session = Self {
            child: Some(child),
            pipe,
            sequence: 0,
            binding,
            authority,
            revision,
            suspended: false,
            viewport,
            dispatcher,
            allow_results,
        };
        if let Some(headers) = headers {
            // Installation and mount share the original absolute deadline; binary data bypasses MAX_FRAME/inline-string gates.
            let child = session.child.as_ref().ok_or_else(wire::invalid)?;
            session.sequence = 1;
            let mut channel = session
                .pipe
                .cancellable_io(deadline, child.handle(), stopped);
            wire::write(
                &mut channel,
                &wire::Request {
                    sequence: 1,
                    viewport: None,
                    command: Command::InstallImages { headers },
                },
            )?;
            wire::images::write(&mut channel, &images)?;
            let installed: wire::images::Installed = wire::read(&mut channel)?;
            if installed.sequence != 1 || stopped() {
                return Err(wire::invalid());
            }
        }
        drop(images);
        let frame = session.exchange(
            Command::Mount {
                source,
                snapshot: value,
                resources: assets,
            },
            deadline,
            stopped,
        )?;
        Ok((session, frame))
    }
}

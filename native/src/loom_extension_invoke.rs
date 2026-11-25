//! One-shot command transaction: obtain remote one-time permission before reading host resources; hosts still check targets/commit complete results.
use super::*;
use serde_json::{json, Value};
struct Policy {
    access: Access,
    scope: String,
    command: String,
    gesture: bool,
    timeout: Duration,
}
impl Policy {
    fn new(command: catalog::Command<'_>, ocr_text: bool) -> Result<Self, Error> {
        let permissions = command.permissions().ok_or(Error::Denied)?;
        let payload = command.payload().ok_or(Error::Wire)?;
        let context = match payload.get("inputContext") {
            None => false,
            Some(Value::String(value)) if value == "ocr-text.v1" && ocr_text => true,
            _ => return Err(Error::Unsupported),
        };
        let attachments = permissions.contains(&"hook.unit.attachments.read");
        if context && !attachments {
            return Err(Error::Denied);
        }
        let timeout = payload
            .get("timeoutMs")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v > 0.0)
            .map_or(20_000, |v| {
                if v >= 170_000.0 {
                    180_000
                } else {
                    v.trunc() as u64 + 10_000
                }
            });
        Ok(Self {
            access: Access {
                plugin: command.plugin().into(),
                image: permissions.contains(&"hook.unit.image.read"),
                attachments,
                ocr_text: context,
            },
            scope: command.scope().into(),
            command: command.id().into(),
            gesture: payload.get("requiresUserGesture") != Some(&Value::Bool(false)),
            timeout: Duration::from_millis(timeout.clamp(5_000, 180_000)),
        })
    }
}
impl Session {
    /// For declared resource reads, call prepare only after valid remote permission; callbacks must recheck original target/source identity.
    /// Background owners only; this layer never directly accesses HWND/files or executes side effects, and failure closes sessions/unconsumed permits.
    pub fn invoke(
        &mut self,
        command: &str,
        context: &Context<'_>,
        target: &Target,
        prepare: impl FnOnce(&Access) -> Result<Prepared, Error>,
        cancel: &AtomicBool,
    ) -> Result<Outcome, Error> {
        if cancel.load(Ordering::Acquire) {
            self.stop();
            return Err(Error::Cancelled);
        }
        let snapshot = self.snapshot().ok_or(Error::Closed)?;
        let generation = snapshot.generation();
        let policy = Policy::new(
            snapshot
                .command(command, context)
                .ok_or(Error::Unavailable)?,
            self.ocr_text,
        )?;
        let result = self.invoke_once(generation, policy, target, prepare, cancel);
        self.settle(generation, result)
    }
    fn invoke_once(
        &mut self,
        generation: u64,
        policy: Policy,
        target: &Target,
        prepare: impl FnOnce(&Access) -> Result<Prepared, Error>,
        cancel: &AtomicBool,
    ) -> Result<Outcome, Error> {
        let authorization = if policy.access.image || policy.access.attachments {
            let id = request_id()?;
            let request = json!({"method":"loom.extension.command.authorize", "params":{
                "requestId":id, "sessionId":self.id, "pluginId":policy.access.plugin, "commandId":policy.command,
                "snapshotGeneration":generation, "target":target, "checkOnly":false }});
            let data = wire::exchange(
                self.socket.as_mut().ok_or(Error::Closed)?,
                &mut self.snapshot,
                &id,
                request,
                Duration::from_secs(5),
                cancel,
            )?;
            let lease = data["authorizationId"]
                .as_str()
                .filter(|v| valid_lease(v))
                .ok_or(Error::Wire)?;
            Some(lease.to_owned())
        } else {
            None
        };
        self.check_generation(generation, cancel)?;
        let prepared = prepare(&policy.access)?;
        self.check_generation(generation, cancel)?;
        let (input, images, attachments) = prepared.wire(&policy.access)?;
        let id = request_id()?;
        let mut request = json!({"method":"loom.extension.command.invoke", "params":{
            "sessionId":self.id, "invocation":{"protocol":catalog::PROTOCOL, "apiVersion":catalog::API,
                "requestId":id, "pluginId":policy.access.plugin, "commandId":policy.command,
                "snapshotGeneration":generation, "target":target, "input":input, "resourceRefs":[] }}});
        if let Some(images) = images {
            request["params"]["resourceUploads"] = images;
        }
        if let Some(attachments) = attachments {
            request["params"]["invocation"]["unitAttachments"] = attachments;
        }
        if let Some(authorization) = authorization {
            request["params"]["authorizationId"] = json!(authorization);
        }
        if policy.gesture {
            request["params"]["invocation"]["userGestureToken"] = json!(gesture()?);
        }
        let deadline = Instant::now() + policy.timeout;
        let request = request.to_string();
        if request.len() > resources::REQUEST_BYTES {
            return Err(Error::Limit);
        }
        let response = wire::exchange_text(
            self.socket.as_mut().ok_or(Error::Closed)?,
            &mut self.snapshot,
            &id,
            request,
            deadline,
            cancel,
        )?;
        Outcome::parse(
            response,
            &id,
            policy.access.plugin,
            policy.scope,
            generation,
        )
    }
    fn check_generation(&self, generation: u64, cancel: &AtomicBool) -> Result<(), Error> {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        match self.snapshot() {
            Some(snapshot) if snapshot.generation() == generation => Ok(()),
            Some(_) => Err(Error::AuthorityChanged),
            None => Err(Error::Closed),
        }
    }
}
fn valid_lease(value: &str) -> bool {
    value
        .strip_prefix("extension-auth:")
        .and_then(|id| {
            uuid::Uuid::parse_str(id)
                .ok()
                .map(|uuid| uuid.to_string() == id)
        })
        .unwrap_or(false)
}
fn gesture() -> Result<String, Error> {
    let mut bytes = [0; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| Error::Random)?;
    bytes[6] = bytes[6] & 0x0f | 0x40;
    bytes[8] = bytes[8] & 0x3f | 0x80;
    Ok(format!("otter-gesture:{}", uuid::Uuid::from_bytes(bytes)))
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "loom_extension_invoke_tests.rs"]
mod tests;

//! Current-user login startup; writes require explicit menu actions, without automatically taking over other Otter installations' startup entries.
use std::{io, path::Path};
#[path = "autostart_registry.rs"]
pub mod registry;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Disabled,
    Enabled,
    OtherInstallation,
}
trait Store {
    fn command(&self) -> io::Result<Option<Vec<u8>>>;
    fn approval(&self) -> io::Result<Option<Vec<u8>>>;
    fn set_command(&self, command: Option<&[u8]>) -> io::Result<()>;
    fn approve(&self) -> io::Result<()>;
}
fn command(executable: &Path) -> io::Result<Vec<u8>> {
    let path = executable.to_str().ok_or_else(invalid)?;
    if !executable.is_absolute() || path.contains(['"', '\0', '\r', '\n']) {
        return Err(invalid());
    }
    let command = format!("\"{path}\"");
    // Run command lines are limited to 260 characters, without cmd/PowerShell expansion or interpretation.
    if command.encode_utf16().count() > 260 {
        return Err(invalid());
    }
    Ok(command
        .encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect())
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "Invalid startup executable path",
    )
}
fn status(store: &impl Store, expected: &[u8]) -> io::Result<Status> {
    let Some(actual) = store.command()? else {
        return Ok(Status::Disabled);
    };
    if actual != expected {
        return Ok(Status::OtherInstallation);
    }
    let approval = store.approval()?;
    Ok(
        if approval
            .as_deref()
            .and_then(|bytes| bytes.first())
            .is_some_and(|value| matches!(value, 3 | 7))
        {
            Status::Disabled
        } else {
            Status::Enabled
        },
    )
}
fn set(store: &impl Store, expected: &[u8], enabled: bool) -> io::Result<Status> {
    if status(store, expected)? == Status::OtherInstallation {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Startup belongs to another Otter installation",
        ));
    }
    if enabled {
        // Read approval entries first so read errors leave no partial writes; report subsequent write failure without fabricated rollback success.
        let approval = store.approval()?;
        store.set_command(Some(expected))?;
        if approval.is_some() {
            store.approve()?;
        }
    } else {
        store.set_command(None)?;
    }
    let actual = status(store, expected)?;
    if actual
        != if enabled {
            Status::Enabled
        } else {
            Status::Disabled
        }
    {
        return Err(io::Error::other("Startup state changed during update"));
    }
    Ok(actual)
}
pub fn query() -> io::Result<Status> {
    status(&registry::Registry, &command(&std::env::current_exe()?)?)
}
pub fn set_enabled(enabled: bool) -> io::Result<Status> {
    set(
        &registry::Registry,
        &command(&std::env::current_exe()?)?,
        enabled,
    )
}
#[cfg(test)]
#[path = "autostart_tests.rs"]
mod tests;

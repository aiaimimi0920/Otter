//! Bounded registration state machine for the recovery child; transport must be authenticated, and bind must pin the original source-process identity and keep its handle alive.
//! Use a single-request-in-flight protocol: without an ACK, only resend the unchanged request; do not pipeline the next sequence number.
use crate::live_recovery_wire::{Ack, Command, Record, Request, Status, MAX_RECORDS, SCHEMA};
use std::io;

struct Entry<T> {
    id: u64,
    record: Record,
    owner: T,
}
pub struct Service<T> {
    session: [u8; 16],
    entries: Vec<Entry<T>>,
    last: Option<(Request, Ack)>,
    highest_id: u64,
    shutdown: bool,
    recovering: bool,
}
fn protocol(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
impl<T> Service<T> {
    pub fn new(session: [u8; 16]) -> io::Result<Self> {
        if session == [0; 16] {
            return Err(protocol("Invalid Live recovery session"));
        }
        Ok(Self {
            session,
            entries: Vec::new(),
            last: None,
            highest_id: 0,
            shutdown: false,
            recovering: false,
        })
    }
    pub fn pending(&self) -> usize {
        self.entries.len()
    }
    pub fn shutdown_accepted(&self) -> bool {
        self.shutdown
    }
    pub fn apply(
        &mut self,
        request: Request,
        bind: impl FnOnce(&Record) -> io::Result<T>,
    ) -> io::Result<Ack> {
        request.validate()?;
        if request.session != self.session || self.recovering {
            return Err(protocol("Inactive Live recovery session"));
        }
        if let Some((previous, ack)) = &self.last {
            if request.sequence == previous.sequence {
                return if &request == previous {
                    Ok(ack.clone())
                } else {
                    Err(protocol("Changed Live recovery retry"))
                };
            }
        }
        let expected = self
            .last
            .as_ref()
            .map_or(Some(1), |(request, _)| request.sequence.checked_add(1));
        if self.shutdown || Some(request.sequence) != expected {
            return Err(protocol("Out-of-order Live recovery request"));
        }
        let accepted = match &request.command {
            Command::Register { id, record } => {
                if *id <= self.highest_id {
                    false
                } else {
                    // Consume the id even on rejection; after retirement, an old id cannot register another generation of window state.
                    self.highest_id = *id;
                    if self.entries.len() >= MAX_RECORDS
                        || self
                            .entries
                            .iter()
                            .any(|entry| entry.record.hwnd == record.hwnd)
                    {
                        false
                    } else {
                        self.entries.try_reserve(1).map_err(io::Error::other)?;
                        match bind(record) {
                            Ok(owner) => {
                                self.entries.push(Entry {
                                    id: *id,
                                    record: record.clone(),
                                    owner,
                                });
                                true
                            }
                            Err(_) => false,
                        }
                    }
                }
            }
            Command::Retire { id } => {
                if *id > self.highest_id {
                    false
                } else {
                    self.entries.retain(|entry| entry.id != *id);
                    true
                }
            }
            Command::Shutdown => {
                self.shutdown = self.entries.is_empty();
                self.shutdown
            }
        };
        let ack = Ack {
            schema: SCHEMA,
            session: self.session,
            sequence: request.sequence,
            status: if accepted {
                Status::Accepted
            } else {
                Status::Rejected
            },
        };
        self.last = Some((request, ack.clone()));
        Ok(ack)
    }
    /// Call only after a trusted owner confirms that terminal recovery should begin; do not infer parent death from EOF alone.
    /// Release successful records; retain failed records and the original source owner for retries. Accept no new commands after entering recovery.
    pub fn recover(
        &mut self,
        mut restore: impl FnMut(&Record, &mut T) -> io::Result<()>,
    ) -> io::Result<()> {
        self.recovering = true;
        let mut first: Option<io::Error> = None;
        let mut index = 0;
        while index < self.entries.len() {
            let entry = &mut self.entries[index];
            match restore(&entry.record, &mut entry.owner) {
                Ok(()) => {
                    self.entries.remove(index);
                }
                Err(error) => {
                    // Asynchronous waiting must not hide actual restoration failures for other sources in the same round; retain both kinds of responsibility.
                    if first.as_ref().is_none_or(|previous| {
                        previous.kind() == io::ErrorKind::WouldBlock
                            && error.kind() != io::ErrorKind::WouldBlock
                    }) {
                        first = Some(error);
                    }
                    index += 1;
                }
            }
        }
        first.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
#[path = "live_recovery_service_tests.rs"]
mod tests;

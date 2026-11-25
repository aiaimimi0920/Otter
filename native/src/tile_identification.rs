//! Temporary identification owner for one presenter; monotonic deadlines, no reopening old requests, and reports consumed once by the original serial control thread.
use crate::tile_wire::{Endpoint, Identification as Command};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    pub endpoint: String,
    pub lease: String,
    pub request: String,
}
pub(crate) struct Marker {
    pub identity: Identity,
    pub endpoint: Endpoint,
    pub until: Instant,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Applied,
    Dismissed,
}
impl Outcome {
    pub fn wire(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Dismissed => "dismissed",
        }
    }
}
pub(crate) struct Report {
    pub identity: Identity,
    pub outcome: Outcome,
}
#[derive(Default)]
pub(crate) struct Identification {
    last: Option<Identity>,
    active: Option<Arc<Marker>>,
    pending: Option<Report>,
    applied_attempted: bool,
    // Local hiding does not permit immediate old-scene reuse; wait for a fresh normal control cycle before reopening presentation.
    blocked: bool,
    ordinary_observed: bool,
}
impl Identification {
    pub fn blocked(&self) -> bool {
        self.blocked
    }
    pub fn clear(&mut self) {
        self.active = None;
        self.pending = None;
        self.blocked = false;
        self.ordinary_observed = false;
        // Retain the last identity so even late old GETs cannot reopen the same canceled/expired request.
    }
    pub fn expire(&mut self, now: Instant) {
        if self.active.as_ref().is_some_and(|m| now >= m.until) {
            self.active = None;
            if self
                .pending
                .as_ref()
                .is_some_and(|p| p.outcome == Outcome::Applied)
            {
                self.pending = None;
            }
        }
    }
    pub fn observe(
        &mut self,
        command: Option<&Command>,
        endpoint: &Endpoint,
        lease: &str,
        sent: Instant,
        now: Instant,
    ) {
        self.expire(now);
        self.ordinary_observed = false;
        let Some(command) = command else {
            self.active = None;
            self.pending = None;
            self.ordinary_observed = true;
            return;
        };
        let identity = Identity {
            endpoint: endpoint.endpoint_id.clone(),
            lease: lease.into(),
            request: command.request_id.clone(),
        };
        let until = sent + Duration::from_millis(command.remaining_ms);
        if self.last.as_ref() == Some(&identity) {
            if let Some(marker) = &mut self.active {
                // Preserve stable Arc identity so old paints cannot confirm markers retired by shortened deadlines.
                if until < marker.until {
                    *marker = Arc::new(Marker {
                        identity,
                        endpoint: endpoint.clone(),
                        until,
                    });
                }
            }
            self.expire(now);
            self.ordinary_observed = self.active.is_none();
            return;
        }
        self.clear();
        self.last = Some(identity.clone());
        self.applied_attempted = false;
        if until > now && endpoint.display.as_ref().is_some_and(|d| d.can_identify) {
            self.active = Some(Arc::new(Marker {
                identity,
                endpoint: endpoint.clone(),
                until,
            }));
            self.blocked = true;
        }
    }
    pub fn marker(&self, now: Instant) -> Option<Arc<Marker>> {
        self.active.as_ref().filter(|m| now < m.until).cloned()
    }
    pub fn finish_cycle(&mut self) {
        if self.ordinary_observed && self.active.is_none() {
            self.blocked = false;
        }
    }
    pub fn confirm(&mut self, marker: &Arc<Marker>, now: Instant) {
        self.expire(now);
        if !self.applied_attempted && self.active.as_ref().is_some_and(|m| Arc::ptr_eq(m, marker)) {
            self.applied_attempted = true;
            self.pending = Some(Report {
                identity: marker.identity.clone(),
                outcome: Outcome::Applied,
            });
        }
    }
    pub fn dismiss(&mut self, displayed: &Arc<Marker>, now: Instant) -> bool {
        self.expire(now);
        if !self
            .active
            .as_ref()
            .is_some_and(|m| Arc::ptr_eq(m, displayed))
        {
            return false;
        }
        let Some(marker) = self.active.take() else {
            return false;
        };
        // Reduce authority first in the same slot; if the control thread already took applied, dismissed naturally follows it, never in reverse order.
        self.pending = Some(Report {
            identity: marker.identity.clone(),
            outcome: Outcome::Dismissed,
        });
        true
    }
    pub fn take_report(&mut self, endpoint: &str, lease: &str, now: Instant) -> Option<Report> {
        self.expire(now);
        self.pending
            .take()
            .filter(|p| p.identity.endpoint == endpoint && p.identity.lease == lease)
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "tile_identification_tests.rs"]
mod tests;

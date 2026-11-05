//! 一个presenter的临时识别owner；单调期限、不重开旧请求，报告由原串行控制线程消费一次。
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
    // 本地隐藏并不允许立即复用旧场景；必须等新鲜普通控制周期重开呈现。
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
        // 保留最后身份，即使旧GET迟到也不能重开已取消/过期的同一请求。
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
                // 保稳定Arc身份，让旧paint不能确认缩短期限后已退休的marker。
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
        // 同槽优先减权；若applied已被控制线程取走，dismissed自然排在它后面，绝不反序。
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
#[cfg(test)]
#[path = "tile_identification_tests.rs"]
mod tests;

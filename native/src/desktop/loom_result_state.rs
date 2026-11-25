//! Single-node result owner: previews never propagate to official ports; final commits preserve this round's preview and merge ports atomically.
use super::{
    image::Image,
    loom_output_batch::{Batch, Output, Port},
};
use crate::{graph_requests::Request, workspace_id::UnitId};
use std::sync::Arc;
#[path = "art_local_result_state.rs"]
mod local_result;
#[path = "loom_result_restore_values.rs"]
mod restore_values;
#[path = "loom_result_save_values.rs"]
mod save_values;

/// Opaque official-output evidence; no pixels or identity reuse from counter overflow/recreated same-named nodes.
#[derive(Clone)]
pub struct OutputStamp(Arc<()>);
impl OutputStamp {
    pub fn matches(&self, results: &Results) -> bool {
        Arc::ptr_eq(&self.0, &results.output_stamp.0)
    }
}
pub struct ImageSource {
    pub image: super::loom_image_value::ImageValue,
    pub stamp: OutputStamp,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    InvalidOutput,
    Stale,
    Capacity,
    Allocation,
    Budget,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,
    Running,
    Completed,
    Failed,
    Cancelled,
}
pub struct Results {
    surface: Option<surface::Version>,
    surface_revoked: bool,
    output_stamp: OutputStamp,
    status: Status,
    progress: Option<f64>,
    error: Option<String>,
    unit: UnitId,
    active: Option<Request>,
    preview_revision: u64,
    workflow_preview: Option<Image>,
    display: Option<Image>,
    restored_preview_locked: bool,
    ports: Vec<Port>,
    // Immutable committed port costs, in port order; Surface replacement invalidates them.
    port_costs: Option<Vec<usize>>,
    formal_outputs_committed: bool,
    candidates: Option<serde_json::Value>,
    candidate_view: Option<Arc<super::loom_output_batch::candidates::Set>>,
    candidate_configuration: Option<super::configuration_stamp::Stamp>,
}
impl Results {
    pub fn new(unit: UnitId) -> Self {
        Self {
            surface: None,
            surface_revoked: false,
            output_stamp: OutputStamp(Arc::new(())),
            status: Status::Idle,
            progress: None,
            error: None,
            unit,
            active: None,
            preview_revision: 0,
            workflow_preview: None,
            display: None,
            restored_preview_locked: false,
            ports: Vec::new(),
            port_costs: None,
            formal_outputs_committed: false,
            candidates: None,
            candidate_view: None,
            candidate_configuration: None,
        }
    }
    pub fn unit(&self) -> &UnitId {
        &self.unit
    }
    /// Session images serve only initial display, without fabricated completed requests, official ports, or workflow previews.
    pub fn from_restored_display(unit: UnitId, display: Option<Image>, locked: bool) -> Self {
        Self {
            restored_preview_locked: locked && display.is_some(),
            display,
            ..Self::new(unit)
        }
    }
    pub fn holds_restored_preview(&self) -> bool {
        self.restored_preview_locked
    }
    /// Clear retention flags only, without erasing old images or fabricating official outputs. Execution toggles and this flag are independent.
    pub fn invalidate_restored_preview(&mut self) {
        self.restored_preview_locked = false;
    }
    pub fn begin(&mut self, request: Request) {
        self.surface = None;
        self.surface_revoked = true;
        self.invalidate_restored_preview();
        self.status = Status::Running;
        self.progress = None;
        self.error = None;
        self.active = Some(request);
        self.preview_revision = 0;
        self.workflow_preview = None;
        self.clear_candidates();
        // As in the baseline, starting new execution does not clear existing official output/display.
    }
    pub fn display(&self) -> Option<&Image> {
        self.display.as_ref()
    }
    pub fn ports(&self) -> &[Port] {
        &self.ports
    }
    pub fn output_stamp(&self) -> OutputStamp {
        self.output_stamp.clone()
    }
    /// Use official ports only; previews/display do not select execution sources. Share image pixels without copying.
    pub fn image_source(&self, name: &str) -> Option<ImageSource> {
        let port = self.ports.iter().find(|port| port.name == name)?;
        Some(ImageSource {
            image: port.value.image_value()?,
            stamp: self.output_stamp(),
        })
    }
    pub fn candidates(&self) -> Option<&serde_json::Value> {
        self.candidates.as_ref()
    }
    pub(crate) fn receive_candidates(&mut self, candidates: Option<serde_json::Value>) {
        self.candidates = candidates;
    }
    pub(crate) fn bind_candidate_configuration(
        &mut self,
        stamp: super::configuration_stamp::Stamp,
    ) {
        self.candidate_configuration = self.candidate_view.as_ref().map(|_| stamp);
    }
    pub(crate) fn candidate_view(
        &self,
        stamp: &super::configuration_stamp::Stamp,
    ) -> Option<&Arc<super::loom_output_batch::candidates::Set>> {
        (self.status == Status::Completed && self.candidate_configuration.as_ref() == Some(stamp))
            .then_some(self.candidate_view.as_ref())
            .flatten()
    }
    fn clear_candidates(&mut self) {
        self.candidates = None;
        self.candidate_view = None;
        self.candidate_configuration = None;
    }
    pub fn active(&self) -> Option<&Request> {
        self.active.as_ref()
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn progress(&self) -> Option<f64> {
        self.progress
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    /// Accept only finite progress values for current requests; never guess whether remote units are percentages or normalized.
    pub fn set_progress(&mut self, request: &Request, progress: f64) -> bool {
        if self.active.as_ref() != Some(request) || !progress.is_finite() {
            return false;
        }
        self.progress = Some(progress);
        true
    }
    /// Bound user-visible errors and preserve old displays/official ports on failure; no logging or raw-protocol-response copies.
    pub fn fail(&mut self, request: &Request, message: &str) -> bool {
        if self.active.as_ref() != Some(request) {
            return false;
        }
        self.error = Some(message.chars().take(1024).collect());
        self.status = Status::Failed;
        self.active = None;
        self.workflow_preview = None;
        true
    }
    pub fn preview(
        &mut self,
        request: &Request,
        revision: u64,
        batch: Batch,
    ) -> Result<bool, Error> {
        if self.active.as_ref() != Some(request) {
            return Err(Error::Stale);
        }
        if revision == 0 || revision <= self.preview_revision {
            return Ok(false);
        }
        self.preview_revision = revision;
        let Some(image) = batch.primary().value.image() else {
            return Ok(false);
        };
        self.workflow_preview = Some(image.clone());
        self.display = Some(image.clone());
        Ok(true)
    }
    /// Callers confirm document/graph freshness before committing in the same UI scheduling segment; release remote-result leases first.
    /// Local results neither need nor may fabricate remote leases/deliveries; this layer does not replace application provenance/configuration validation.
    /// Failure preserves old results; inputs are purely local owners and can be reclaimed safely on failure.
    pub fn finish(&mut self, request: &Request, batch: Batch) -> Result<(), Error> {
        if self.active.as_ref() != Some(request) {
            return Err(Error::Stale);
        }
        let image = batch.primary().value.image_value();
        let candidates = batch.candidates.clone();
        let (mut incoming, _) = batch.into_ports();
        // Baseline final image output replaces both output and output_image, sharing pixels without copying.
        if let Some(image) = &image {
            incoming.try_reserve(2).map_err(|_| Error::Allocation)?;
            for name in ["output", "output_image"] {
                if let Some(port) = incoming.iter_mut().find(|port| port.name == name) {
                    port.value = Output::from_image(image.clone());
                } else {
                    incoming.push(Port {
                        name: name.into(),
                        value: Output::from_image(image.clone()),
                    });
                }
            }
        }
        let retained = self
            .ports
            .iter()
            .enumerate()
            .filter(|(_, old)| !incoming.iter().any(|new| new.name == old.name));
        let count = incoming.len() + retained.clone().count();
        if count > 128 {
            return Err(Error::Capacity);
        }
        let mut bytes = Meter(0);
        let mut costs = Vec::new();
        costs
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        for (index, port) in retained {
            if let Some(cached) = &self.port_costs {
                let cost = cached[index];
                bytes.add(cost).map_err(|_| Error::Budget)?;
                costs.push(cost);
            } else {
                costs.push(bytes.charge(&port.value)?);
            }
        }
        for port in &incoming {
            costs.push(bytes.charge(&port.value)?);
        }
        // Handle every failure point before moving old ports; conservatively count aliases repeatedly toward total official-output budget.
        let mut merged = Vec::new();
        merged.try_reserve(count).map_err(|_| Error::Allocation)?;
        let output_stamp = OutputStamp(Arc::new(()));
        for old in self.ports.drain(..) {
            if !incoming.iter().any(|new| new.name == old.name) {
                merged.push(old);
            }
        }
        merged.append(&mut incoming);
        self.ports = merged;
        self.port_costs = Some(costs);
        self.formal_outputs_committed = true;
        self.output_stamp = output_stamp;
        self.clear_candidates();
        self.candidate_view = candidates;
        self.display = self
            .workflow_preview
            .take()
            .or_else(|| image.map(|value| value.image().clone()))
            .or_else(|| self.display.take());
        self.active = None;
        self.status = Status::Completed;
        self.progress = Some(1.0);
        self.error = None;
        Ok(())
    }
    pub fn cancel(&mut self, request: &Request) -> bool {
        if self.active.as_ref() != Some(request) {
            return false;
        }
        self.active = None;
        self.workflow_preview = None;
        self.status = Status::Cancelled;
        self.error = None;
        true
    }
}
#[path = "surface_node_results.rs"]
mod surface;
// Avoid serializing an equally large JSON string merely to check budgets.
struct Meter(usize);
impl Meter {
    fn charge(&mut self, output: &Output) -> Result<usize, Error> {
        let before = self.0;
        match output {
            Output::Image(image) => self.add(image.pixels.len()),
            Output::PngImage(image) => self.add(image.bytes()),
            Output::Value(value) => {
                return serde_json::to_writer(&mut *self, value)
                    .map(|()| self.0 - before)
                    .map_err(|_| Error::Budget);
            }
        }
        .map_err(|_| Error::Budget)?;
        Ok(self.0 - before)
    }
    fn add(&mut self, count: usize) -> std::io::Result<()> {
        self.0 = self
            .0
            .checked_add(count)
            .filter(|value| *value <= 256 * 1024 * 1024)
            .ok_or_else(|| std::io::Error::other("result budget exceeded"))?;
        Ok(())
    }
}
impl std::io::Write for Meter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.add(bytes.len())?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "loom_result_state_tests.rs"]
mod tests;

//! 单向 PNG 持续同步的后台阶段；许可仍归原 Job/Attempt，永不 reaccept/displayed/unlink。
use super::*;
use crate::projection_delivery::{
    accept::Read,
    binding::{checkpoint::Index, Binding},
    journal::Ticket,
};
use crate::{image_decode::Image, pixels::Pixels, session_source::Source};
#[path = "projection_sync_patch.rs"]
mod patch;

pub(crate) struct Request {
    pub ticket: Ticket,
    pub expected: Binding,
    pub source: Source,
    pub size: (u32, u32),
    pub frame: crate::crop_geometry::Rect,
    pub original: Option<Image>,
    pub replaceable: bool,
    pub editing: Option<crate::projection_edit::Document>,
}
pub struct Update {
    pub source: Source,
    pub index: Index,
    pub binding: Binding,
    pub image: Option<ImageUpdate>,
    pub editing: Option<crate::projection_edit::Document>,
    pub editing_frame: Option<crate::crop_geometry::Rect>,
}
pub struct ImageUpdate {
    pub original: Image,
    pub base: Pixels,
    pub mask: crate::alpha_region::AlphaRegion,
}
pub enum Outcome {
    Unchanged,
    Updated(Box<Update>),
}
impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if matches!(self, Self::Unchanged) {
            "Unchanged"
        } else {
            "Updated"
        })
    }
}
impl Request {
    pub(super) fn run(
        self,
        control: &crate::live_relay_http::Control<'_>,
        client: &Client,
        lease: &Lease,
        attempt: &attempt::Attempt,
    ) -> Result<Completed, Error> {
        if self.ticket.origin() != client.origin()
            || self.ticket.device() != lease.device_id()
            || !self.expected.matches_ticket(&self.ticket)
            || self.expected.stopped()
        {
            return Err(Error::Stopped);
        }
        let ticket = attempt.recover(self.ticket)?;
        let response = control.sync_projection(&ticket, &self.expected);
        let cancel = lease.preparation_cancel();
        let (binding, image, encoded, editing) = match response {
            Ok(Read::Unchanged) => {
                if self.editing.is_some() {
                    return Err(Error::Decode(crate::projection_delivery::Error::Response));
                }
                return Ok(Completed::Metadata(Output::Synchronized(Box::new(
                    Outcome::Unchanged,
                ))));
            }
            Ok(Read::Editing(document)) => {
                Self::check_edit(self.editing.as_ref(), Some(&document))?;
                if self
                    .expected
                    .source_dimensions()
                    .or_else(|| self.editing.as_ref().map(|doc| doc.basis().size()))
                    != Some(document.basis().size())
                {
                    return Err(Error::Decode(crate::projection_delivery::Error::Response));
                }
                (self.expected.clone(), None, None, Some(document))
            }
            Ok(Read::Changed(response)) => {
                let value = received::decode(*response, lease, &ticket)?;
                let editing = value.editing().cloned();
                Self::check_edit(self.editing.as_ref(), editing.as_ref())?;
                if value.revision() == self.expected.revision() {
                    if editing.is_none() {
                        return Ok(Completed::Metadata(Output::Synchronized(Box::new(
                            Outcome::Unchanged,
                        ))));
                    }
                    (self.expected.clone(), None, None, editing)
                } else if editing.is_some() && value.digest() == self.expected.digest() {
                    (
                        Binding::new_cancellable(&value, &ticket, || lease.is_revoked())
                            .map_err(Error::Decode)?,
                        None,
                        None,
                        editing,
                    )
                } else {
                    // 破坏性本地编辑/旧未证明的持久源只暂停像素更新，read/终态仍能收敛。
                    if !self.replaceable
                        || !self.original.as_ref().is_some_and(|image| {
                            self.expected.source_matches(image, || lease.is_revoked())
                        })
                    {
                        return Err(Error::Stopped);
                    }
                    let next = Binding::new_cancellable(&value, &ticket, || lease.is_revoked())
                        .map_err(Error::Decode)?;
                    let image = value.image().clone();
                    let placement = fit(self.frame, (image.width, image.height));
                    let base = crate::raster_projection::project(
                        &image.pixels,
                        (image.width, image.height),
                        self.size,
                        placement,
                        || lease.is_revoked(),
                    )
                    .map_err(|_| Error::Decode(crate::projection_delivery::Error::Snapshot))?;
                    let encoded = crate::loom_base64::encode_png_data_url(value.png(), cancel)
                        .map_err(|_| Error::Decode(crate::projection_delivery::Error::Budget))?;
                    let mask = crate::alpha_region::build_layers_cancellable(
                        self.size.0,
                        self.size.1,
                        &base,
                        None,
                        || lease.is_revoked(),
                    )
                    .ok_or(Error::Decode(crate::projection_delivery::Error::Budget))?;
                    (
                        next,
                        Some(ImageUpdate {
                            original: image,
                            base,
                            mask,
                        }),
                        Some(encoded),
                        editing,
                    )
                }
            }
            Err(HttpError::ProjectionStopped(reason)) => {
                (self.expected.stop(reason), None, None, None)
            }
            Err(error) => return Err(Error::Http(error)),
        };
        // 首次接入即使远端附带合法PNG，也不能把未换新的本地像素误认成同一basis。
        // 已有owner由UI弱像素身份围栏保护，不在每轮metadata read重复hash整图。
        if self.editing.is_none()
            && image.is_none()
            && editing.as_ref().is_some_and(|document| {
                !self.replaceable
                    || !self.original.as_ref().is_some_and(|image| {
                        (image.width, image.height) == document.basis().size()
                            && self.expected.source_matches(image, || lease.is_revoked())
                    })
            })
        {
            return Err(Error::Stopped);
        }
        if lease.is_revoked() {
            return Err(Error::Stopped);
        }
        let source = if binding == self.expected && encoded.is_none() {
            self.source.clone()
        } else {
            patch::prepare(
                &self.source,
                &self.expected,
                &binding,
                encoded.as_ref().map(|e| e.as_str()),
                cancel,
            )
            .map_err(|_| Error::Decode(crate::projection_delivery::Error::Response))?
        };
        let index = Index::capture(&source)
            .ok_or(Error::Decode(crate::projection_delivery::Error::Response))?;
        Ok(Completed::Metadata(Output::Synchronized(Box::new(
            Outcome::Updated(Box::new(Update {
                source,
                index,
                binding,
                image,
                editing_frame: editing
                    .as_ref()
                    .map(|document| fit(self.frame, document.basis().size())),
                editing,
            })),
        ))))
    }
    fn check_edit(
        previous: Option<&crate::projection_edit::Document>,
        document: Option<&crate::projection_edit::Document>,
    ) -> Result<(), Error> {
        if let Some(previous) = previous {
            document
                .ok_or(Error::Decode(crate::projection_delivery::Error::Response))?
                .follows(previous)
                .map_err(|_| Error::Decode(crate::projection_delivery::Error::Response))?;
        }
        Ok(())
    }
}
fn fit(frame: crate::crop_geometry::Rect, size: (u32, u32)) -> crate::crop_geometry::Rect {
    let (width, height) = (size.0 as f64, size.1 as f64);
    let scale = (frame.width / width).min(frame.height / height);
    crate::crop_geometry::Rect {
        x: frame.x + (frame.width - width * scale) / 2.0,
        y: frame.y + (frame.height - height * scale) / 2.0,
        width: width * scale,
        height: height * scale,
    }
}

//! Continuous PNG sync reconciles with Read before incrementing; received updates replace only a provably rebasable base image and preserve local annotations.
use super::*;
struct ImageInput {
    size: (u32, u32),
    frame: crate::crop_geometry::Rect,
}
impl Application {
    pub(super) unsafe fn start_account_sync(&mut self, binding: Binding, stop: bool) -> Result<()> {
        if !self.account_idle()
            || !self
                .workspace_checkpoint
                .as_ref()
                .is_some_and(|o| o.includes_account(&binding, &self.workspace))
        {
            return Err(stale());
        }
        let origin = binding.origin().to_owned();
        let participation = if stop {
            None
        } else {
            Some(
                self.authorization
                    .borrow()
                    .consent
                    .participation()
                    .ok_or_else(stale)?,
            )
        };
        if self.authorization.borrow().origin.as_deref() != Some(&origin) {
            return Err(stale());
        }
        let unit = UnitId::parse(binding.unit()).ok_or_else(stale)?;
        let record = self.restored_session.as_ref().ok_or_else(stale)?;
        let source = record.original.clone();
        let (guard, input, image_input) = if binding.is_source() && !stop {
            let (scope, snapshot) = projection_sender::scope::Scope::capture(self, &unit)?;
            let input = work::SourceInput {
                source: scope.binding_source.clone(),
                unit: unit.clone(),
                scope: scope.scope,
                snapshot,
            };
            (Guard::Source(Box::new(scope)), Some(input), None)
        } else {
            let doc = self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id == unit)
                .ok_or_else(stale)?;
            let guard = restore_batch::guard::Document::capture(doc)?;
            let canvas = doc.state.canvas.borrow();
            let image = if !binding.is_source()
                && !stop
                && canvas.editor.source_replaceable()
                && canvas.minified.is_none()
                && canvas
                    .minified_source
                    .as_ref()
                    .is_some_and(|source| source.matches(&canvas.editor))
            {
                canvas.editor.layers().map(|layers| ImageInput {
                    size: layers.size,
                    frame: canvas.editor.full_logical_frame().unwrap_or(
                        crate::crop_geometry::Rect {
                            x: 0.0,
                            y: 0.0,
                            width: layers.size.0 as f64,
                            height: layers.size.1 as f64,
                        },
                    ),
                })
            } else {
                None
            };
            (
                Guard::Existing {
                    unit,
                    document: Box::new(guard),
                },
                None,
                image,
            )
        };
        let lease = self
            .authorization_owner
            .lease(&origin, crate::desktop::surface_windows::now_ms()?)
            .map_err(|_| stale())?;
        let discovery = crate::loom_discovery::Config::from_env();
        let original = source.clone();
        let job = job::Job::start(lease, self.execution_wake.callback(), move |lease| {
            let account = lease
                .account_projections(&discovery)
                .map_err(job::failure)?;
            if binding.device() != account.context().device_id {
                return Err(stale());
            }
            let cancel = lease.preparation_cancel();
            let response = if stop {
                account.execute(account::Operation::Unlink {
                    envelope: binding.envelope(),
                })
            } else {
                account.execute(account::Operation::Read {
                    envelope: binding.envelope(),
                    known_revision: binding.revision(),
                    known_digest: binding.digest(),
                    receiver_unit: (!binding.is_source()).then_some(binding.unit()),
                })
            };
            let terminal = matches!(&response, Ok(account::State::Unlinked))
                || matches!(&response, Err(account::Error::Remote { status: 410, code: Some(code) }) if matches!(code.as_str(), "projection_unlinked" | "projection_invitation_expired"));
            if terminal {
                let next = binding.stop();
                let source = next
                    .replace(&original, Some(&binding), cancel)
                    .map_err(job::failure)?;
                return Ok(Output::Patch {
                    source,
                    binding: Box::new(next),
                    image: None,
                });
            }
            let mut content = job::content(response.map_err(job::failure)?)?;
            let mut next = binding
                .follows(&content, account.context())
                .map_err(job::failure)?;
            let mut image = None;
            let mut encoded = None;
            if let Some(input) = input {
                if content.linked {
                    let pixels = work::image(&input, lease)?;
                    if pixels.digest() != content.digest {
                        content = job::content(
                            account
                                .execute(account::Operation::Update {
                                    envelope: binding.envelope(),
                                    prior_revision: content.revision,
                                    revision: content.revision.checked_add(1).ok_or_else(stale)?,
                                    image: &pixels,
                                })
                                .map_err(job::failure)?,
                        )?;
                        next = next
                            .follows(&content, account.context())
                            .map_err(job::failure)?;
                    }
                }
            } else if !binding.is_source() && next.digest() != binding.digest() {
                let layout = image_input.ok_or_else(stale)?;
                let value = content.image.take().ok_or_else(stale)?;
                let png = value.png(cancel).map_err(job::failure)?;
                let original = crate::image_decode::Image::from_projection_png(
                    &png,
                    value.width(),
                    value.height(),
                    &|| cancel.load(Ordering::Acquire),
                )?;
                let scale = (layout.frame.width / original.width as f64)
                    .min(layout.frame.height / original.height as f64);
                let width = original.width as f64 * scale;
                let height = original.height as f64 * scale;
                let placement = crate::crop_geometry::Rect {
                    x: layout.frame.x + (layout.frame.width - width) / 2.0,
                    y: layout.frame.y + (layout.frame.height - height) / 2.0,
                    width,
                    height,
                };
                let base = crate::raster_projection::project(
                    &original.pixels,
                    (original.width, original.height),
                    layout.size,
                    placement,
                    || cancel.load(Ordering::Acquire),
                )
                .map_err(|_| stale())?;
                let mask = crate::alpha_region::build_layers_cancellable(
                    layout.size.0,
                    layout.size.1,
                    &base,
                    None,
                    || cancel.load(Ordering::Acquire),
                )
                .ok_or_else(stale)?;
                encoded = Some(
                    crate::loom_base64::encode_png_data_url(&png, cancel).map_err(|_| stale())?,
                );
                image = Some(crate::projection_delivery_runtime::sync::ImageUpdate {
                    original,
                    base,
                    mask,
                });
            }
            if next == binding {
                return Ok(Output::Unchanged);
            }
            let source = if let Some(encoded) = encoded {
                next.replace_image(&original, &binding, encoded.as_str(), cancel)
            } else {
                next.replace(&original, Some(&binding), cancel)
            }
            .map_err(job::failure)?;
            Ok(Output::Patch {
                source,
                binding: Box::new(next),
                image,
            })
        })?;
        self.account_projection.pending = Some(Pending {
            job,
            origin,
            source,
            workspace: self.workspace.identity_stamp(),
            guard,
            cancel: None,
            participation,
        });
        self.account_status(if stop {
            "正在解除账号关联…"
        } else {
            "正在同步账号投射…"
        });
        Ok(())
    }
}

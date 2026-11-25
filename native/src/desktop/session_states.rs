//! 会话候选转为未发布宿主状态；不建 HWND、不执行 Art、不触及现有工作区。
use super::{Candidate, Parts};
use crate::desktop::{
    capability_selection::Selection, document_state, loom_result_state::Results, State,
};
use crate::session_art_batch::Binding;
use std::{
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG},
};
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_states_source_tests.rs"]
mod source_tests;

pub struct Prepared {
    // 完整会话和缺失能力身份保留在候选原文中；此 owner 不是持久化提交凭证。
    pub candidate: Candidate,
    pub catalog: Option<Rc<crate::desktop::capability_job::Loaded>>,
    pub(in crate::desktop) states: Vec<State>,
    pub annotation_ids: Vec<(crate::workspace_id::UnitId, Vec<String>)>,
}
impl Prepared {
    pub fn state_count(&self) -> usize {
        self.states.len()
    }
}

impl Parts {
    pub fn prepare_states(self, cancel: &AtomicBool) -> Result<Prepared> {
        let Parts {
            candidate,
            catalog,
            arts,
            results,
            vectors,
            regions,
            projection_edits,
        } = self;
        let units = candidate.workspace.units();
        let count = units.len();
        if [
            candidate.placements.len(),
            candidate.images.len(),
            vectors.len(),
            regions.len(),
        ]
        .iter()
        .any(|&len| len != count)
        {
            return Err(invalid("Session state count mismatch"));
        }
        // 密集资源与工作区同序；稀疏资源按同一顺序过滤生成，不能跳过不匹配的记录。
        let mut arts = arts.into_iter();
        let mut results = results.into_iter();
        let mut projection_edits = projection_edits.into_iter().peekable();
        let mut states = Vec::<State>::with_capacity(vectors.len());
        let mut annotation_ids = Vec::with_capacity(vectors.len());
        for ((((unit, restored), (region_unit, region)), placement), (images, expected)) in vectors
            .into_iter()
            .zip(regions)
            .zip(&candidate.placements)
            .zip(candidate.images.iter().zip(units))
        {
            check(cancel)?;
            if unit != *expected
                || unit != placement.unit
                || unit != images.unit
                || unit != region_unit
            {
                return Err(invalid("Invalid restored state identity"));
            }
            // UI 线程接管后台 alpha 缓存，不重新扫描图片或创建图形设备。
            let mut state = State::new(None, true)?;
            state.unit_id = unit.clone();
            *state.extension_document.borrow_mut() =
                crate::desktop::extension_document::Owner::restore(
                    placement.extension_state.clone(),
                    placement.extension_propagation.as_deref(),
                );
            state.extension_document.borrow_mut().legacy_text = placement.legacy_text.clone();
            state.sticker_input.restore(images.sticker.clone());
            state.accept_upstream.set(images.accept_upstream);
            state.node_settings.set(placement.node_settings);
            if projection_edits.peek().is_some_and(|(id, _)| *id == unit) {
                *state.projection_edit.borrow_mut() =
                    projection_edits.next().map(|(_, owner)| owner);
            }
            state.restored_art = placement.art;
            state.local_art_id = placement.local_art_id.clone().map(|id| {
                let config = document_state::local_art::Config::from(id);
                *config.ports.borrow_mut() =
                    placement.local_art_ports.clone().map(std::sync::Arc::new);
                config
            });
            state.restored_naming_art =
                crate::desktop::naming_context::RestoredArtId::from_id(placement.art_id.as_deref());
            *state.hit_region.borrow_mut() = region;
            *state.execution_results.borrow_mut() = if placement.art {
                results
                    .next()
                    .filter(|result| result.unit() == &unit)
                    .ok_or_else(|| invalid("Missing restored Art result"))?
            } else {
                Results::new(unit.clone())
            };
            {
                let mut canvas = state.canvas.borrow_mut();
                canvas.editor = restored.editor;
                // 恢复位置已对应编辑器当前的可见框；后续同步只能应用相对变化。
                canvas.applied_crop = canvas.editor.annotation_frame();
                canvas.frame_style = placement.frame_style;
                canvas.normal_opacity = placement.opacity_normal as f32;
                canvas.window_opacity = if placement.minified {
                    placement.opacity_mini
                } else {
                    placement.opacity_normal
                } as f32;
                canvas.sync_raster_source();
                let mut capture_source = placement
                    .capture_meta
                    .as_deref()
                    .map(crate::desktop::canvas::CaptureSource::from_session)
                    .transpose()?;
                if let Some(source) = capture_source.as_mut() {
                    source.original_lease = images.capture_original.clone();
                }
                canvas.attach_capture(capture_source);
                canvas.minified_opacity = placement.opacity_mini as f32;
                canvas.minified_source =
                    images
                        .source
                        .as_ref()
                        .or(images.preview.as_ref())
                        .map(|image| {
                            let source = crate::desktop::canvas::minified_source::Source::new(
                                image.clone(),
                                &canvas.editor,
                            )
                            .with_origin(crate::desktop::canvas::minified_source::Origin {
                                record: candidate.retained_source_record(),
                                unit: unit.clone(),
                            })
                            .with_rasterized_crop(
                                placement.rasterized_full
                                    || placement.crop.is_some_and(|crop| crop.rasterized_source),
                            );
                            // 保存非等比恢复之前的 sourceSize，不能用当前 Editor 框替代。
                            if let Some(crop) = placement.crop {
                                source.with_source_logical(crop.crop.source)
                            } else {
                                source
                            }
                        });
                // 预览只供显示；绝不能写入可编辑底图或生成正式输出端口。
                if canvas.image.is_none() {
                    canvas.image = images.display().cloned();
                }
                if let Some(views) = placement.minified_views {
                    canvas.minified =
                        Some(crate::desktop::canvas::minified::Presentation::restored(
                            images.source.as_ref().or(images.preview.as_ref()).cloned(),
                            views,
                            crate::crop_geometry::Size {
                                width: placement.rect.w,
                                height: placement.rect.h,
                            },
                            &canvas.editor,
                        )?);
                }
            }
            if placement.art {
                let entry = arts
                    .next()
                    .filter(|entry| entry.unit == unit)
                    .ok_or_else(|| invalid("Missing restored Art binding"))?;
                if let Binding::Ready { restored, .. } = entry.binding {
                    *state.selected_art.borrow_mut() = Some(Selection {
                        identity: document_state::new_unit_id()?,
                        catalog: catalog
                            .as_ref()
                            .ok_or_else(|| invalid("Executable Art requires a real catalog"))?
                            .clone(),
                        parameters: restored.parameters,
                        execution: restored.execution,
                    });
                }
            }
            if let Some(group) = candidate.workspace.group_of(&unit) {
                *state.group_id.borrow_mut() = Some(group.id.clone());
                state.group_hidden.set(group.hidden);
                state.group_locked.set(group.locked);
            }
            annotation_ids.push((unit, restored.ids));
            states.push(state);
        }
        if arts.next().is_some() || results.next().is_some() || projection_edits.next().is_some() {
            return Err(invalid("Unmatched restored Art or projection state"));
        }
        check(cancel)?;
        Ok(Prepared {
            candidate,
            catalog,
            states,
            annotation_ids,
        })
    }
}
fn invalid(message: &'static str) -> Error {
    Error::new(E_INVALIDARG, message)
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Session state preparation cancelled"))
    } else {
        Ok(())
    }
}

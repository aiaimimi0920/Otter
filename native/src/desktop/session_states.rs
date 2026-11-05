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
#[cfg(test)]
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
            mut arts,
            mut results,
            vectors,
            mut regions,
            mut projection_edits,
        } = self;
        if vectors.len() != candidate.placements.len()
            || candidate.images.len() != candidate.placements.len()
            || regions.len() != vectors.len()
        {
            return Err(invalid("Session state count mismatch"));
        }
        let mut states = Vec::<State>::with_capacity(vectors.len());
        let mut annotation_ids = Vec::with_capacity(vectors.len());
        for (unit, restored) in vectors {
            check(cancel)?;
            if states.iter().any(|state| state.unit_id == unit)
                || !candidate.workspace.contains(&unit)
            {
                return Err(invalid("Invalid restored state identity"));
            }
            let placement = candidate
                .placements
                .iter()
                .find(|item| item.unit == unit)
                .ok_or_else(|| invalid("Missing restored state placement"))?;
            let images = candidate
                .images
                .iter()
                .find(|item| item.unit == unit)
                .ok_or_else(|| invalid("Missing restored state image"))?;
            // UI 线程接管后台 alpha 缓存，不重新扫描图片或创建图形设备。
            let mut state = State::new(None, true)?;
            state.unit_id = unit.clone();
            state.sticker_input.restore(images.sticker.clone());
            state.accept_upstream.set(images.accept_upstream);
            if let Some(index) = projection_edits.iter().position(|(id, _)| *id == unit) {
                *state.projection_edit.borrow_mut() = Some(projection_edits.swap_remove(index).1);
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
            let region = regions
                .iter()
                .position(|(id, _)| id == &unit)
                .ok_or_else(|| invalid("Missing restored hit region"))?;
            *state.hit_region.borrow_mut() = regions.swap_remove(region).1;
            *state.execution_results.borrow_mut() = if placement.art {
                let index = results
                    .iter()
                    .position(|result| result.unit() == &unit)
                    .ok_or_else(|| invalid("Missing restored Art result"))?;
                results.swap_remove(index)
            } else {
                Results::new(unit.clone())
            };
            {
                let mut canvas = state.canvas.borrow_mut();
                canvas.editor = restored.editor;
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
                            .with_origin(
                                crate::desktop::canvas::minified_source::Origin {
                                    record: candidate.retained_source_record(),
                                    unit: unit.clone(),
                                },
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
                let index = arts
                    .iter()
                    .position(|entry| entry.unit == unit)
                    .ok_or_else(|| invalid("Missing restored Art binding"))?;
                if let Binding::Ready { restored, .. } = arts.swap_remove(index).binding {
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
        if !arts.is_empty() || !results.is_empty() || !projection_edits.is_empty() {
            return Err(invalid("Unmatched restored Art state"));
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

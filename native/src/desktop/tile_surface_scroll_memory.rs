//! prepare空隙仅保留至多四个同租约Art的COW偏移；不拥有scene、像素、COM或呈现资格。
use super::*;
use crate::tile_shared::{Key, State};
use crate::tile_surface_wire::View;

struct Entry {
    view: View,
    generation: u64,
    size: [u32; 2],
    scroll: crate::surface_scroll::Scroll,
}
#[derive(Default)]
pub(in crate::desktop::tile_window::render) struct Memory {
    key: Option<(Key, crate::tile_wire::Size)>,
    entries: BTreeMap<String, Entry>,
}
impl Memory {
    pub(in crate::desktop::tile_window::render) fn synchronize(
        &mut self,
        frame: Option<&Frame>,
        cache: &BTreeMap<String, Prepared>,
        state: &State,
    ) {
        if let Some(frame) = frame {
            self.key = Some((frame.key.clone(), frame.size));
            // 协议原本最多四个Art来源；防御性上限不允许本地状态扩展这份预算。
            self.entries = frame
                .arts
                .iter()
                .take(4)
                .filter_map(|(id, art)| {
                    let p = cache.get(id).filter(|p| Arc::ptr_eq(&p.owner, art))?;
                    Some((
                        id.clone(),
                        Entry {
                            view: art.view.clone(),
                            generation: art.generation,
                            size: art.size,
                            scroll: p.presentation.scroll_state(),
                        },
                    ))
                })
                .collect();
        }
        let now = std::time::Instant::now();
        if state.stopped
            || state.expired
            || state.frozen
            || state.black
            || state.identification.blocked()
            || state.until.is_none_or(|until| now >= until)
            || !state.clock.permits(state.activate_at, now)
            || self
                .key
                .as_ref()
                .is_none_or(|(key, _)| state.desired.as_ref() != Some(key))
        {
            self.key = None;
            self.entries.clear();
            return;
        }
        self.entries.retain(|id, saved| {
            state.actions.states.get(id).is_some_and(|s| {
                s.view == saved.view && s.generation == saved.generation && s.size == saved.size
            })
        });
    }
    pub(in crate::desktop::tile_window::render) fn restore(
        &self,
        frame: Option<&Frame>,
        cache: &mut BTreeMap<String, Prepared>,
    ) -> Result<()> {
        let Some(frame) = frame.filter(|f| {
            self.key
                .as_ref()
                .is_some_and(|(key, size)| *key == f.key && *size == f.size)
        }) else {
            return Ok(());
        };
        for (id, saved) in &self.entries {
            let Some(p) = cache.get_mut(id).filter(|p| {
                frame.arts.get(id).is_some_and(|a| Arc::ptr_eq(a, &p.owner))
                    && p.owner.view == saved.view
                    && p.owner.generation == saved.generation
                    && p.owner.size == saved.size
            }) else {
                continue;
            };
            p.presentation.restore_scroll(&saved.scroll)?;
            reembed(p)?;
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "tile_surface_scroll_memory_tests.rs"]
mod tests;

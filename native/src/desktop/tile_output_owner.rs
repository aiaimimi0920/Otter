//! 一个稳定物理输出的单代际owner；先销毁旧窗口/排空旧线程，才允许同Grant的新绑定。
use super::*;
use crate::desktop::tile_window::Window;
use crate::tile_outputs::TileOutput;

struct Active {
    window: Window,
    runtime: Runtime,
    output: TileOutput,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Status {
    Running,
    Waiting,
    Draining,
    Stopped,
}
pub(super) struct Output {
    id: String,
    observed: Option<TileOutput>,
    active: Option<Active>,
    retiring: Option<Runtime>,
    stopped: bool,
}
impl Output {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            observed: None,
            active: None,
            retiring: None,
            stopped: false,
        }
    }
    /// 成功枚举也必须唯一匹配原ID；不追随枚举顺序、不自动接管另一屏。
    pub fn observe(&mut self, inventory: std::result::Result<Vec<TileOutput>, String>) {
        if self.stopped {
            return;
        }
        self.observed = inventory.ok().and_then(|outputs| {
            let mut matches = outputs.into_iter().filter(|v| v.output_id == self.id);
            let value = matches.next()?;
            (matches.next().is_none() && admissible(&value)).then_some(value)
        });
    }
    pub fn stop(&mut self) {
        self.stopped = true;
        self.observed = None;
        self.retire();
    }
    fn retire(&mut self) {
        if let Some(active) = self.active.take() {
            active.window.hide();
            active.runtime.invalidate_output();
            drop(active.window);
            debug_assert!(self.retiring.is_none());
            self.retiring = Some(active.runtime);
        }
    }
    pub fn drain(&mut self) -> bool {
        if self.retiring.as_ref().is_some_and(Runtime::finished) {
            self.retiring.take(); // finished之后Drop仍执行原join/凭据清理。
        }
        self.active.is_none() && self.retiring.is_none()
    }
    pub fn tick(
        &mut self,
        owner: &mut Owner,
        origin: &str,
        profile: Option<&Path>,
    ) -> Result<Status> {
        let result = self.advance(owner, origin, profile);
        if result.is_err() {
            self.stop();
        }
        result
    }
    fn advance(
        &mut self,
        owner: &mut Owner,
        origin: &str,
        profile: Option<&Path>,
    ) -> Result<Status> {
        if self.stopped {
            return Ok(if self.drain() {
                Status::Stopped
            } else {
                Status::Draining
            });
        }
        if self.active.as_ref().is_some_and(|a| !a.window.alive()) {
            self.stop();
            return self.advance(owner, origin, profile);
        }
        if self
            .active
            .as_ref()
            .is_some_and(|a| a.window.output_changed())
        {
            // DPI消息不保证物理矩形改变；必须等下一次新鲜枚举，不能立即复用旧探测值。
            self.observed = None;
            self.retire();
        } else if self
            .active
            .as_ref()
            .is_some_and(|a| Some(&a.output) != self.observed.as_ref())
        {
            self.retire();
        }
        if let Some(retiring) = &self.retiring {
            if !retiring.finished() {
                return Ok(Status::Draining);
            }
            let valid = retiring.authorization_valid(origin);
            self.retiring.take();
            if !valid {
                return Err(fail("Tile authorization ended during output recovery"));
            }
        }
        if owner.token(origin, now_ms()).is_none() {
            return Err(fail(
                "Tile authorization expired; approve/renew explicitly in manager",
            ));
        }
        if let Some(active) = &self.active {
            if active.runtime.finished() {
                return Err(fail(format!("Tile stopped: {}", active.runtime.status())));
            }
            active.window.tick()?;
            if !active.window.alive() {
                self.stop();
                return self.advance(owner, origin, profile);
            }
            if active.window.output_changed() {
                self.observed = None;
                self.retire();
                return self.advance(owner, origin, profile);
            }
            return Ok(Status::Running);
        }
        let Some(output) = self.observed.clone() else {
            return Ok(Status::Waiting);
        };
        let lease = owner
            .lease(origin, now_ms())
            .map_err(|_| fail("Tile authorization unavailable"))?;
        let images = owner
            .lease(origin, now_ms())
            .map_err(|_| fail("Tile authorization unavailable"))?;
        let art = owner
            .lease(origin, now_ms())
            .map_err(|_| fail("Tile authorization unavailable"))?;
        let runtime = Runtime::start(config(origin, profile)?, lease, images, art, output.clone())
            .map_err(|_| fail("Tile runtime start"))?;
        let window = match Window::new(runtime.shared.clone(), &output) {
            Ok(window) => window,
            Err(error) => {
                runtime.request_stop();
                self.retiring = Some(runtime);
                return Err(error);
            }
        };
        self.active = Some(Active {
            window,
            runtime,
            output,
        });
        Ok(Status::Running)
    }
    #[cfg(test)]
    pub(super) fn runtime_for_test(&self) -> Option<&Runtime> {
        self.active.as_ref().map(|a| &a.runtime)
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        self.stop();
        // 正常路径由UI泵排空；仅异常早退才使用Runtime的有界请求/join兜底。
        self.retiring.take();
    }
}
fn admissible(value: &TileOutput) -> bool {
    (1..=16384).contains(&value.width)
        && (1..=16384).contains(&value.height)
        && u64::from(value.width) * u64::from(value.height) <= crate::tile_wire::MAX_PIXELS
        && value.x.checked_add_unsigned(value.width).is_some()
        && value.y.checked_add_unsigned(value.height).is_some()
}
#[cfg(test)]
#[path = "tile_output_owner_tests.rs"]
mod tests;

//! 同 revision 的本地呈现更新：只交付 viewId，不替换权威场景、资源或 child。
use super::*;

impl Session {
    /// 上层先复核本次目录/宿主声明；此接缝不赋予 Art 执行或远端更新权限。
    pub fn select_view(&mut self, view_id: &str, stopped: &impl Fn() -> bool) -> io::Result<Frame> {
        let view_id = crate::art_surface_views::Id::parse(view_id).ok_or_else(wire::invalid)?;
        self.exchange(
            Command::SelectView { view_id },
            Instant::now() + TIMEOUT,
            stopped,
        )
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_javascript_view_tests.rs"]
mod tests;

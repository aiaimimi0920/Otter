//! 当前Live owner的发布接缝；只移交同捕获Consumer与有限身份，不复制帧或创建第二WGC。
use super::*;
use crate::{live_frames::Consumer, live_relay_http::source::Publication};
impl Session {
    pub(in crate::desktop) fn observation_target(
        &self,
    ) -> Result<crate::live_window_target::Target, String> {
        if !self.is_alive() {
            return Err("Live 源已关闭".into());
        }
        // 裁切窗口只共享可见图像，不默认泄露未包含在所选区域中的其他控件。
        if self.input_crop.is_some() {
            return Err("窗口选区不共享整窗观察".into());
        }
        let target = self.target.as_ref().ok_or("此来源不是窗口")?;
        target.validate().map_err(|e| e.to_string())?;
        crate::live_window_target::input_security::preflight(target)
            .map_err(|e| format!("窗口观察不可用：{e:?}"))?;
        Ok(target.clone())
    }
    pub(in crate::desktop) fn take_publication_request(
        &self,
    ) -> Option<super::super::window::publication::Request> {
        self.window.as_ref()?.take_publication_request()
    }
    pub(in crate::desktop) fn publication_phase(
        &self,
        phase: super::super::window::publication::Phase,
    ) {
        if let Some(window) = &self.window {
            window.publication_phase(phase);
        }
    }
    pub(in crate::desktop) fn source_publication(
        &self,
    ) -> Result<(Publication, Consumer<Texture>), String> {
        if !self.is_alive() {
            return Err("Live 源已关闭".into());
        }
        let frame = self
            .snapshot_frame()
            .map_err(|e| e.to_string())?
            .ok_or("Live 尚无可发布帧")?;
        let window = self
            .target
            .as_ref()
            .map(|target| {
                target.validate().map_err(|e| e.to_string())?;
                Ok::<_, String>((
                    format!("{:x}", target.hwnd().0 as usize),
                    target.process_id(),
                ))
            })
            .transpose()?;
        let publication = Publication::new(
            &self.unit_id.to_string(),
            [frame.resource.width, frame.resource.height],
            window,
        )
        .map_err(|e| format!("发布来源无效：{e:?}"))?;
        let consumer = self
            .job
            .subscribe()
            .map_err(|e| format!("Live 订阅失败：{e:?}"))?;
        Ok((publication, consumer))
    }
}

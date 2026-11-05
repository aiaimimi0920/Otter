//! 结算真实 Application JS 挂载；与图片共用原 Grant/任务容量，不在 UI 上准备 Client。
use super::*;
impl Application {
    pub(super) fn poll_surface_javascript_with(
        &mut self,
        visible: bool,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) -> Result<()> {
        let now = crate::desktop::surface_windows::now_ms()?;
        // 同一应用输入策略先收紧再交付回复；模态/撤权后的旧请求不能趁 refresh 抢焦点。
        let origin = self.surface_input_origin(visible);
        for window in self.surface_windows.iter_mut() {
            if origin.as_deref() != Some(window.binding().origin()) {
                window.disable_local_input()?;
            }
        }
        self.surface_windows.poll(now)?;
        let mut running = self
            .surface_windows
            .iter_mut()
            .filter(|w| !w.javascript_finished())
            .count();
        for window in self.surface_windows.iter_mut() {
            if !window.javascript_needs_start() {
                continue;
            }
            if running >= crate::surface_javascript::MAX_SESSIONS {
                break;
            }
            let origin = window.binding().origin();
            let lease = match self.authorization_owner.lease(origin, now) {
                Ok(lease) => lease,
                Err(crate::device_authorization_owner::Error::Capacity) => continue,
                Err(error) => {
                    window.fail_javascript(format!("JS 授权不可用：{error:?}"))?;
                    continue;
                }
            };
            let config = match config(origin) {
                Ok(config) => config,
                Err(error) => {
                    window.fail_javascript(format!("JS 配置无效：{error:?}"))?;
                    continue;
                }
            };
            if let Err(error) = window.start_javascript(config, lease) {
                // 未启动的容量竞争可等现有任务完成再试；一旦创建 Runtime 则不重放挂载。
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    continue;
                }
                window.fail_javascript(format!("JS 挂载未启动：{error}"))?;
            } else {
                running += 1;
            }
        }
        Ok(())
    }
}

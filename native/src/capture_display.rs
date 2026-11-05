//! 显示器 HDR 元数据只读快照；每次探测新建 DXGI factory，不缓存易变化的系统颜色状态。
use windows::{
    core::Interface,
    Win32::Graphics::{
        Dxgi::{Common::*, *},
        Gdi::HMONITOR,
    },
};
#[path = "capture_display_config.rs"]
mod configuration;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayInfo {
    pub hdr_enabled: bool,
    pub sdr_white_nits: f32,
    pub minimum_nits: f32,
    pub maximum_nits: f32,
    pub configuration_known: bool,
    pub white_level_known: bool,
}
#[derive(Debug)]
pub enum Error {
    InvalidMonitor,
    NotFound,
    EnumerationLimit,
    Api(windows::core::Error),
}
/// 只枚举元数据；不打开捕获会话、切换 HDR、设置显示模式或创建窗口。
pub fn query(monitor: HMONITOR) -> Result<DisplayInfo, Error> {
    if monitor.0.is_null() {
        return Err(Error::InvalidMonitor);
    }
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1().map_err(Error::Api)? };
    for adapter_index in 0..32 {
        let adapter = match unsafe { factory.EnumAdapters1(adapter_index) } {
            Ok(adapter) => adapter,
            Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => return Err(Error::NotFound),
            Err(error) => return Err(Error::Api(error)),
        };
        for output_index in 0..64 {
            let output = match unsafe { adapter.EnumOutputs(output_index) } {
                Ok(output) => output,
                Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(error) => return Err(Error::Api(error)),
            };
            if let Ok(output) = output.cast::<IDXGIOutput6>() {
                if let Ok(desc) = unsafe { output.GetDesc1() } {
                    if desc.Monitor == monitor {
                        let config = configuration::query(&desc.DeviceName);
                        return Ok(combine(&desc, config));
                    }
                }
            }
            if output_index == 63 {
                return Err(Error::EnumerationLimit);
            }
        }
    }
    Err(Error::EnumerationLimit)
}
fn combine(desc: &DXGI_OUTPUT_DESC1, config: Option<(bool, Option<u32>)>) -> DisplayInfo {
    let pq = matches!(
        desc.ColorSpace,
        DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020 | DXGI_COLOR_SPACE_RGB_STUDIO_G2084_NONE_P2020
    );
    let white = config
        .and_then(|(_, raw)| raw)
        .map(|raw| (80.0 * raw as f32 / 1000.0).max(80.0));
    let maximum = if desc.MaxLuminance.is_finite() {
        desc.MaxLuminance.clamp(1.0, 10_000.0)
    } else {
        1000.0
    };
    DisplayInfo {
        hdr_enabled: pq && config.is_none_or(|(enabled, _)| enabled),
        sdr_white_nits: white.unwrap_or(203.0),
        minimum_nits: if desc.MinLuminance.is_finite() {
            desc.MinLuminance.clamp(0.0, maximum)
        } else {
            0.0
        },
        maximum_nits: maximum,
        configuration_known: config.is_some(),
        white_level_known: white.is_some(),
    }
}
#[cfg(test)]
#[path = "capture_display_tests.rs"]
mod tests;

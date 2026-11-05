//! HWND 与隔离 JS 共用的客户区/DPI/字体合同；不使用外框尺寸或建立第二套布局默认值。
use crate::{
    surface_length::Context,
    surface_text::{Align, Style, Whitespace},
};
use serde::{Deserialize, Serialize};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::GetClientRect},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    pixels: [u32; 2],
    dpi: u32,
}
impl Viewport {
    pub fn new(pixels: [u32; 2], dpi: u32) -> Result<Self> {
        let value = Self { pixels, dpi };
        value.validate()?;
        Ok(value)
    }
    pub(crate) fn validate(self) -> Result<()> {
        if self.dpi == 0 || self.pixels.iter().any(|v| *v > 32768) {
            return Err(Error::new(E_INVALIDARG, "Invalid Surface viewport"));
        }
        Ok(())
    }
    pub(crate) fn from_window(hwnd: HWND) -> Result<Self> {
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(hwnd, &mut rect)?;
        }
        if rect.right < 0 || rect.bottom < 0 {
            return Err(Error::new(E_INVALIDARG, "Invalid Surface viewport"));
        }
        Self::new([rect.right as u32, rect.bottom as u32], unsafe {
            GetDpiForWindow(hwnd)
        })
    }
    pub fn pixels(self) -> [u32; 2] {
        self.pixels
    }
    pub fn dpi(self) -> u32 {
        self.dpi
    }
    pub fn ratio(self) -> f64 {
        f64::from(self.dpi) / 96.
    }
    pub fn logical(self) -> [f32; 2] {
        self.pixels.map(|v| v as f32 * 96. / self.dpi as f32)
    }
    pub(crate) fn context(self) -> Context {
        context(self.logical())
    }
}
pub(crate) fn style() -> Style<'static> {
    Style {
        family: "Segoe UI",
        size: 16.,
        weight: 400,
        whitespace: Whitespace::Normal,
        align: Align::Start,
        line_height: None,
    }
}
pub(crate) fn context(size: [f32; 2]) -> Context {
    Context {
        axis: size[0],
        viewport: size,
        container: size,
        root_font: 16.,
        font: 16.,
        fraction: None,
    }
}

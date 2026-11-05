//! 系统 trackbar 与有界位置映射；子窗口由对话框销毁，不创建线程或计时器。
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::*,
        UI::{Controls::*, WindowsAndMessaging::*},
    },
};
const TICKS: i32 = 10000;
// Windows SDK 的 TBM_GETPOS = WM_USER；windows 0.60 未生成这个别名。
const TBM_GETPOS: u32 = WM_USER;
pub(super) const ID: i32 = 105;
#[derive(Clone, Copy)]
pub(super) struct Range {
    min: f64,
    max: f64,
    step: f64,
    current: f64,
}
impl Range {
    pub fn new(current: f64, min: Option<f64>, max: Option<f64>, step: f64) -> Option<Self> {
        if !current.is_finite() || !step.is_finite() || step <= 0.0 {
            return None;
        }
        let min = min.unwrap_or(current.min(0.0));
        let max = max.unwrap_or((min + step).max(current).max(100.0)).max(min);
        if !min.is_finite() || !max.is_finite() || !(max - min).is_finite() {
            return None;
        }
        Some(Self {
            min,
            max,
            step,
            current,
        })
    }
    fn position(self) -> i32 {
        if self.max <= self.min {
            return 0;
        }
        (((self.current - self.min) / (self.max - self.min)).clamp(0.0, 1.0) * TICKS as f64).round()
            as i32
    }
    fn value(self, position: i32) -> Option<f64> {
        let raw = self.min + (self.max - self.min) * position.clamp(0, TICKS) as f64 / TICKS as f64;
        let stepped = self.min + ((raw - self.min) / self.step).round() * self.step;
        super::parameter_number::normalize(stepped, Some(self.min), Some(self.max))
    }
}
pub(super) struct Control {
    hwnd: HWND,
    range: Range,
}
impl Control {
    pub(super) fn handle(&self) -> HWND {
        self.hwnd
    }
    pub unsafe fn create(parent: HWND, rect: RECT, range: Range) -> Result<Self> {
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES,
        })
        .ok()?;
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("msctls_trackbar32"),
            None,
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(TBS_AUTOTICKS),
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            Some(parent),
            Some(HMENU(ID as usize as *mut _)),
            None,
            None,
        )?;
        SendMessageW(hwnd, TBM_SETRANGEMIN, Some(WPARAM(0)), Some(LPARAM(0)));
        SendMessageW(
            hwnd,
            TBM_SETRANGEMAX,
            Some(WPARAM(1)),
            Some(LPARAM(TICKS as isize)),
        );
        let mut control = Self { hwnd, range };
        control.sync(range);
        Ok(control)
    }
    pub unsafe fn sync(&mut self, range: Range) {
        self.range = range;
        SendMessageW(
            self.hwnd,
            TBM_SETPOS,
            Some(WPARAM(1)),
            Some(LPARAM(range.position() as isize)),
        );
    }
    pub unsafe fn changed(&self, child: LPARAM) -> Option<String> {
        if child.0 != self.hwnd.0 as isize {
            return None;
        }
        let position = SendMessageW(self.hwnd, TBM_GETPOS, None, None).0 as i32;
        Some(self.range.value(position)?.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn range_quantizes_from_minimum_and_handles_missing_bounds() {
        let range = Range::new(0.3, Some(0.1), Some(1.1), 0.2).unwrap();
        assert_eq!(range.position(), 2000);
        assert_eq!(range.value(5000), Some(0.7));
        assert_eq!(range.value(-1), Some(0.1));
        assert_eq!(range.value(20000), Some(1.1));
        let fallback = Range::new(2.0, None, None, 1.0).unwrap();
        assert_eq!(fallback.value(TICKS), Some(100.0));
        assert_eq!(
            Range::new(2.0, Some(4.0), Some(1.0), 1.0)
                .unwrap()
                .value(TICKS),
            Some(4.0)
        );
        assert!(Range::new(0.0, Some(-f64::MAX), Some(f64::MAX), 1.0).is_none());
    }
    #[test]
    fn hidden_native_control_roundtrips_position_and_rejects_other_sender() {
        unsafe {
            let parent = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                None,
                WINDOW_STYLE::default(),
                0,
                0,
                300,
                100,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let range = Range::new(0.0, Some(0.0), Some(1.0), 0.1).unwrap();
            let control = Control::create(
                parent,
                RECT {
                    left: 0,
                    top: 0,
                    right: 250,
                    bottom: 30,
                },
                range,
            )
            .unwrap();
            SendMessageW(
                control.hwnd,
                TBM_SETPOS,
                Some(WPARAM(0)),
                Some(LPARAM(3000)),
            );
            assert_eq!(
                control.changed(LPARAM(control.hwnd.0 as isize)).as_deref(),
                Some("0.3")
            );
            assert!(control.changed(LPARAM(parent.0 as isize)).is_none());
            DestroyWindow(parent).unwrap();
            assert!(!IsWindow(Some(control.hwnd)).as_bool());
        }
    }
}

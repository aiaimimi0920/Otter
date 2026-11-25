//! Local Chromium wheel delivery: lazy child ownership, fallback packet and stream retirement.
use super::{Backend, Destination, Error, Event, Message};
use crate::live_browser_scroll::{self as scroll, Worker};

impl Backend {
    /// A libtest owner explicitly opts in; unifying test-support never changes normal startup.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn new_local_in_test_harness(
        root: super::Target,
        region: Option<super::FrameRegion>,
    ) -> Result<Self, Error> {
        let mut backend = Self::new_local(root, region)?;
        backend.scroll_in_test_harness = true;
        Ok(backend)
    }

    fn start_browser_scroll(&self) -> std::io::Result<Worker> {
        #[cfg(any(test, feature = "test-support"))]
        if self.scroll_in_test_harness {
            return Worker::start_in_test_harness(self.root.clone());
        }
        Worker::start(self.root.clone())
    }

    pub(super) fn scroll_packet(
        &mut self,
        target: &Destination,
        event: Event,
        buttons: u8,
        packet: Message,
    ) -> Result<Option<Message>, Error> {
        let Event::Wheel {
            delta, horizontal, ..
        } = event
        else {
            return Ok(Some(packet));
        };
        // 点击/键盘不占滚动 child 名额。启动失败传播，不回退或重放结果未知的滚动。
        if self.local && self.browser_scroll.is_none() && scroll::eligible(&self.root) {
            self.browser_scroll = Some(self.start_browser_scroll().map_err(scroll_error)?);
        }
        // 任意传输失败均退休此流，迟到回执不能被下次用户输入误认。
        let Some(mut worker) = self.browser_scroll.take() else {
            return Ok(Some(packet));
        };
        let mapped = self.mouse.as_ref().ok_or(Error::InvalidDelivery)?.mapped;
        let physical = scroll::physical_point(target.hwnd(), mapped.screen)
            .map_err(|error| Error::BrowserScroll(error.to_string()))?;
        let result = worker
            .scroll(physical, delta, horizontal)
            .map_err(scroll_error)?;
        self.browser_scroll = Some(worker);
        result
            .map(|delta| {
                Message::wheel(mapped.screen, buttons, delta, horizontal).map_err(Error::Message)
            })
            .transpose()
    }
}

fn scroll_error(error: std::io::Error) -> Error {
    if error.kind() == std::io::ErrorKind::WouldBlock {
        Error::BrowserTransition(error.to_string())
    } else {
        Error::BrowserScroll(error.to_string())
    }
}

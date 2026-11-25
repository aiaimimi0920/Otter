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
        // Clicks/keyboard input consume no scroll-child slots. Startup failures propagate, without fallback or replay of scrolls with unknown outcomes.
        if self.local && self.browser_scroll.is_none() && scroll::eligible(&self.root) {
            self.browser_scroll = Some(self.start_browser_scroll().map_err(scroll_error)?);
        }
        // Retire this stream on any transport failure so late receipts cannot be mistaken for the next user input.
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

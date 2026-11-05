//! Controller 的 Win32 后端：路由、坐标、受限消息组合；必须由独立输入 worker 拥有。
use super::{
    input_coordinates::{self as coordinates, FrameRegion, Mapped},
    input_message::{self as message, Message},
    input_routing as routing, input_security,
    input_target::{self, Destination},
    Target,
};
use crate::live_input::{self, Event};
#[path = "live_input_button.rs"]
mod button;
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Security(input_security::Denied),
    Target(input_target::Error),
    Coordinates(coordinates::Error),
    Routing(routing::Error),
    Message(message::Error),
    InvalidDelivery,
}
struct MousePosition {
    target: Destination,
    mapped: Mapped,
}
pub struct Backend {
    root: Target,
    root_destination: Destination,
    region: Option<FrameRegion>,
    mouse: Option<MousePosition>,
    button: button::Tracker,
}
impl Backend {
    /// 创建不启用 Controller；region 必须是用户确认的窗口内物理裁剪，不能用于桌面区域。
    pub fn new(root: Target, region: Option<FrameRegion>) -> Result<Self, Error> {
        let root_destination = Destination::bind(&root, root.hwnd()).map_err(Error::Target)?;
        Ok(Self {
            root,
            root_destination,
            region,
            mouse: None,
            button: button::Tracker::default(),
        })
    }
    fn prepare(
        &mut self,
        target: &Destination,
        event: Event,
        buttons: u8,
    ) -> Result<Message, Error> {
        if let Event::Key { code, down } = event {
            return Message::key(code, down).map_err(Error::Message);
        }
        let point = match event {
            Event::Move { point } | Event::Button { point, .. } | Event::Wheel { point, .. } => {
                point
            }
            _ => unreachable!(),
        };
        let mapped =
            coordinates::map(&self.root, target, point, self.region).map_err(Error::Coordinates)?;
        let packet = match event {
            Event::Move { .. } => Message::motion(mapped.destination_client, buttons),
            Event::Button {
                button,
                down,
                double,
                ..
            } => Message::button(mapped.destination_client, buttons, button, down, double),
            Event::Wheel {
                delta, horizontal, ..
            } => Message::wheel(mapped.screen, buttons, delta, horizontal),
            _ => unreachable!(),
        }
        .map_err(Error::Message)?;
        // 发送前保留最后有效位置，超时后的释放不能重新计算或重新命中。
        self.mouse = Some(MousePosition {
            target: target.clone(),
            mapped,
        });
        Ok(packet)
    }
}
impl live_input::Backend for Backend {
    type Target = Destination;
    type Error = Error;
    fn preflight(&mut self) -> Result<(), Error> {
        input_security::preflight(&self.root).map_err(Error::Security)
    }
    fn resolve(
        &mut self,
        event: Event,
        preferred: Option<Destination>,
    ) -> Result<Destination, Error> {
        if let Some(target) = preferred {
            return routing::retained(&self.root, &target).map_err(Error::Routing);
        }
        match event {
            Event::Key { .. } => routing::keyboard(&self.root).map_err(Error::Routing),
            Event::Move { point } | Event::Button { point, .. } | Event::Wheel { point, .. } => {
                let mapped =
                    coordinates::map(&self.root, &self.root_destination, point, self.region)
                        .map_err(Error::Coordinates)?;
                routing::pointer(&self.root, mapped.root_client).map_err(Error::Routing)
            }
        }
    }
    fn send(
        &mut self,
        target: Destination,
        event: Event,
        buttons: u8,
        timeout_ms: u32,
    ) -> Result<(), Error> {
        let expected = if matches!(event, Event::Move { .. }) {
            50
        } else {
            250
        };
        if timeout_ms != expected {
            return Err(Error::InvalidDelivery);
        }
        let packet = self.prepare(&target, event, buttons)?;
        let point = self
            .mouse
            .as_ref()
            .map_or([0, 0], |mouse| mouse.mapped.destination_client);
        let candidate = self.button.track(&target, event, point, buttons);
        if let Err(error) = message::send_to(&target, packet) {
            self.button.clear();
            return Err(Error::Message(error));
        }
        if let Some(candidate) = candidate {
            button::complete(candidate, point)?;
        }
        Ok(())
    }
    fn cleanup_target(&mut self, target: Destination) -> Option<Destination> {
        self.button.clear();
        routing::retained(&self.root, &target).ok()
    }
    fn release(&mut self, target: Destination, event: Event, buttons: u8) -> Result<(), Error> {
        self.button.clear();
        let packet = match event {
            Event::Key { code, down: false } => Message::key(code, false),
            Event::Button {
                button,
                down: false,
                ..
            } => {
                let mouse = self.mouse.as_ref().ok_or(Error::InvalidDelivery)?;
                let point = if mouse.target.same_target(&target) {
                    mouse.mapped.destination_client
                } else if target.same_target(&self.root_destination) {
                    mouse.mapped.root_client
                } else {
                    return Err(Error::InvalidDelivery);
                };
                Message::button(point, buttons, button, false, false)
            }
            _ => return Err(Error::InvalidDelivery),
        }
        .map_err(Error::Message)?;
        // 独立 cleanup 路径：不再次命中/变换、不进行任何按钮点击补偿。
        message::release_to(&target, packet).map_err(Error::Message)
    }
}
#[cfg(test)]
#[path = "live_input_backend_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "live_input_winforms_tests.rs"]
mod winforms_tests;

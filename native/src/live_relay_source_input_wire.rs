//! Source只接受既有window-message能力；完整批次预检后才投递，无远程文本/焦点或全局输入。
use super::input::Action;
use crate::{
    live_input::{Button, Event, MAX_SEQUENCE},
    live_relay_viewer_identity::{identifier, ControlEvent},
};
use serde::Deserialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Protocol,
    Authority,
    Input,
    Closed,
}
#[derive(Clone)]
pub(crate) struct State {
    revision: u64,
    initialized: bool,
    controller: Option<String>,
    input: u64,
    enabled: bool,
    closed: bool,
    suspended: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Session {
    revision: u64,
    visibility: super::super::live_relay_control_owner::Visibility,
    viewers: Vec<String>,
    controller_device_id: Option<String>,
    reason: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    input_sequence: u64,
    issued_at_ms: u64,
    source_device_id: String,
    kind: Kind,
}
#[derive(Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum Kind {
    MouseMove(Point),
    MouseButton(Mouse),
    Wheel(Wheel),
    Key(Key),
    Cancel,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Point {
    x: f64,
    y: f64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Mouse {
    button: String,
    state: String,
    x: f64,
    y: f64,
    click_count: u8,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Wheel {
    delta_x: i32,
    delta_y: i32,
    x: f64,
    y: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Key {
    code: String,
    state: String,
    modifiers: u8,
}
fn pressed(value: &str) -> Result<bool, Error> {
    match value {
        "pressed" => Ok(true),
        "released" => Ok(false),
        _ => Err(Error::Input),
    }
}
impl Kind {
    fn event(self) -> Result<Option<Event>, Error> {
        let event = match self {
            Self::Cancel => return Ok(None),
            Self::MouseMove(p) => Event::Move { point: [p.x, p.y] },
            Self::MouseButton(m) => {
                let button = match m.button.as_str() {
                    "left" => Button::Left,
                    "right" => Button::Right,
                    "middle" => Button::Middle,
                    _ => return Err(Error::Input),
                };
                if !(1..=2).contains(&m.click_count) {
                    return Err(Error::Input);
                }
                let down = pressed(&m.state)?;
                Event::Button {
                    point: [m.x, m.y],
                    button,
                    down,
                    double: down && m.click_count == 2,
                }
            }
            Self::Wheel(w) => {
                let (delta, horizontal) = match (w.delta_x, w.delta_y) {
                    (0, d) if d != 0 => (d, false),
                    (d, 0) if d != 0 => (d, true),
                    _ => return Err(Error::Input),
                };
                Event::Wheel {
                    point: [w.x, w.y],
                    delta,
                    horizontal,
                }
            }
            Self::Key(k) => {
                if k.modifiers != 0 {
                    return Err(Error::Input);
                }
                let code = k
                    .code
                    .strip_prefix("vk:")
                    .and_then(|n| n.parse().ok())
                    .ok_or(Error::Input)?;
                Event::Key {
                    code,
                    down: pressed(&k.state)?,
                }
            }
        };
        if !event.valid() {
            return Err(Error::Input);
        }
        Ok(Some(event))
    }
}
impl State {
    pub fn new(revision: u64) -> Self {
        Self {
            revision,
            initialized: false,
            controller: None,
            input: 0,
            enabled: false,
            closed: false,
            suspended: false,
        }
    }
    pub fn closed(&self) -> bool {
        self.closed
    }
    pub fn suspend(&mut self) {
        self.enabled = false;
        self.suspended = true;
    }
    pub fn prepare(&self, events: &[ControlEvent]) -> Result<(Self, Vec<Action>), Error> {
        if events.len() > 256 {
            return Err(Error::Protocol);
        }
        let mut next = self.clone();
        let mut actions = Vec::new();
        for event in events {
            if next.closed {
                return Err(Error::Closed);
            }
            match event.kind() {
                "session_state" => {
                    let s: Session =
                        serde_json::from_str(event.payload()).map_err(|_| Error::Protocol)?;
                    if s.revision == 0
                        || s.revision < next.revision
                        || s.viewers.len() > 512
                        || s.viewers.iter().any(|v| !identifier(v))
                        || s.controller_device_id
                            .as_ref()
                            .is_some_and(|v| !identifier(v))
                        || s.reason.as_ref().is_some_and(|r| r.len() > 1024)
                    {
                        return Err(Error::Protocol);
                    }
                    if next.controller != s.controller_device_id {
                        if next.enabled {
                            actions.push(Action::Release);
                        }
                        next.enabled = false;
                        next.input = 0;
                        next.controller = s.controller_device_id;
                    }
                    next.closed =
                        s.visibility == crate::live_relay_control_owner::Visibility::Closed;
                    if next.controller.is_none() {
                        next.suspended = false;
                    }
                    if next.closed && next.enabled {
                        actions.push(Action::Release);
                        next.enabled = false;
                    }
                    next.revision = s.revision;
                    next.initialized = true;
                }
                "input_event" => {
                    if !next.initialized {
                        return Err(Error::Protocol);
                    }
                    let i: Input =
                        serde_json::from_str(event.payload()).map_err(|_| Error::Input)?;
                    if next.controller.as_deref() != Some(&i.source_device_id) {
                        return Err(Error::Authority);
                    }
                    if i.input_sequence <= next.input
                        || i.input_sequence > MAX_SEQUENCE
                        || i.issued_at_ms == 0
                        || i.issued_at_ms > MAX_SEQUENCE
                    {
                        return Err(Error::Input);
                    }
                    next.input = i.input_sequence;
                    let event = i.kind.event()?;
                    if next.suspended {
                        continue;
                    }
                    if let Some(event) = event {
                        if !next.enabled {
                            actions.push(Action::Enable);
                            next.enabled = true;
                        }
                        actions.push(Action::Input(event));
                    } else if next.enabled {
                        actions.push(Action::Release);
                        next.enabled = false;
                    }
                }
                _ => return Err(Error::Protocol),
            }
        }
        Ok((next, actions))
    }
}
#[cfg(test)]
#[path = "live_relay_source_input_wire_tests.rs"]
mod tests;

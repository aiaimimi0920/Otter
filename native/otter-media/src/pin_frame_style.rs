//! Pin-content appearance is stored in window logical pixels; not baked into the base image and does not own HWND or selection boxes.
use crate::surface_color::Color;
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Style {
    width: f32,
    color: Option<Color>,
    radius: f32,
}
impl Style {
    pub fn read_raw(raw: Option<&serde_json::value::RawValue>) -> Result<Self, &'static str> {
        raw.map(|raw| {
            // Borrow only the three original entries; do not copy the whole Value, including unknown erase/extension arrays, just to read appearance.
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Fields<'a> {
                #[serde(default, borrow)]
                border_width: Option<&'a serde_json::value::RawValue>,
                #[serde(default, borrow)]
                border_color: Option<&'a serde_json::value::RawValue>,
                #[serde(default, borrow)]
                corner_radius: Option<&'a serde_json::value::RawValue>,
            }
            let fields: Option<Fields<'_>> =
                serde_json::from_str(raw.get()).map_err(|_| "Invalid frame edit state")?;
            let Some(fields) = fields else {
                return Ok(Self::default());
            };
            let mut values = Map::new();
            for (key, raw) in [
                ("borderWidth", fields.border_width),
                ("borderColor", fields.border_color),
                ("cornerRadius", fields.corner_radius),
            ] {
                if let Some(raw) = raw {
                    if raw.get().len() > 386 {
                        return Err("Frame style field exceeds budget");
                    }
                    values.insert(
                        key.into(),
                        serde_json::from_str(raw.get()).map_err(|_| "Invalid frame style field")?,
                    );
                }
            }
            Self::read(&values.into())
        })
        .transpose()
        .map(Option::unwrap_or_default)
    }
    pub fn read(value: &Value) -> Result<Self, &'static str> {
        if !value.is_null() && !value.is_object() {
            return Err("Invalid frame edit state");
        }
        let width = number(value, "borderWidth")?;
        let text = match value.get("borderColor") {
            None | Some(Value::Null) => "",
            Some(value) => value.as_str().ok_or("Invalid frame border color")?,
        };
        let color = if width == 0.0 || text.is_empty() {
            None
        } else {
            Some(Color::parse(text, Color::BLACK).ok_or("Unsupported frame border color")?)
        };
        Ok(Self {
            width,
            color,
            radius: number(value, "cornerRadius")?,
        })
    }
    pub fn border(self) -> Option<(f32, Color)> {
        self.color
            .filter(|_| self.width > 0.0)
            .map(|color| (self.width, color))
    }
    pub fn radius(self) -> f32 {
        self.radius
    }
    pub fn with_radius(self, radius: u8) -> Result<Self, &'static str> {
        if radius > 128 {
            return Err("Invalid editable frame corner radius");
        }
        Ok(Self {
            radius: f32::from(radius),
            ..self
        })
    }
    /// Pressing the same color again turns it off; a different color replaces it and preserves positive width, using the mainline default of 4 DIP.
    pub fn toggle(self, color: Color) -> Self {
        if self.border().is_some_and(|(_, current)| current == color) {
            Self {
                width: 0.0,
                color: None,
                ..self
            }
        } else {
            Self {
                width: if self.width > 0.0 { self.width } else { 4.0 },
                color: Some(color),
                ..self
            }
        }
    }
    pub fn merge(self, original: Value) -> Result<Value, &'static str> {
        let mut fields = match original {
            Value::Null => Map::new(),
            Value::Object(fields) => fields,
            _ => return Err("Invalid frame edit state"),
        };
        fields.insert("borderWidth".into(), self.width.into());
        fields.insert("cornerRadius".into(), self.radius.into());
        if let Some(Color([r, g, b, a])) = self.color {
            fields.insert(
                "borderColor".into(),
                format!("rgba({},{},{},{})", r * 255.0, g * 255.0, b * 255.0, a).into(),
            );
        } else {
            fields.remove("borderColor");
        }
        Ok(fields.into())
    }
}
fn number(value: &Value, key: &str) -> Result<f32, &'static str> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(0.0),
        Some(value) => value
            .as_f64()
            .filter(|v| v.is_finite() && (0.0..=8192.0).contains(v))
            .map(|v| v as f32)
            .ok_or("Invalid frame style dimension"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn border_toggle_preserves_width_on_recolor_and_roundtrips_without_losing_extensions() {
        let red = Color([1., 0., 0., 0.5]);
        let s =
            Style::read(&json!({"borderWidth":7,"borderColor":"blue","cornerRadius":12})).unwrap();
        let s = s.toggle(red);
        assert_eq!(s.border(), Some((7., red)));
        let value = s
            .merge(json!({"future":{"keep":true}, "cropRect":{"x":2}}))
            .unwrap();
        assert_eq!(value["future"]["keep"], true);
        assert_eq!(value["cropRect"]["x"], 2);
        assert_eq!(Style::read(&value).unwrap(), s);
        let off = s.toggle(red);
        assert_eq!(off.border(), None);
        assert_eq!(off.radius(), 12.);
        assert_eq!(off.toggle(red).border(), Some((4., red)));
    }
    #[test]
    fn legacy_large_radius_is_preserved_but_new_ui_edits_use_main_range() {
        let s = Style::read(&json!({"cornerRadius":256.5})).unwrap();
        assert_eq!(s.radius(), 256.5);
        assert!(s.with_radius(255).is_err());
        assert_eq!(s.with_radius(128).unwrap().radius(), 128.);
        for v in [
            json!(3),
            json!({"borderWidth":-1}),
            json!({"cornerRadius":8193}),
            json!({"cornerRadius":"5"}),
            json!({"borderWidth":4,"borderColor":"url(secret)"}),
        ] {
            assert!(Style::read(&v).is_err());
        }
    }
}

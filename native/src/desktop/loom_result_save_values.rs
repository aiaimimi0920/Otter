//! Session snapshots of official JSON ports; no descriptor downloads or disguising local pixels as JSON resources.
use super::{Output, Results};
use serde_json::{Map, Value};

impl Results {
    /// None preserves imported outputs; Some(empty) means a completed empty result and must clear historical ports.
    /// Save with save_output_images; background encoding adds image ports to the same output object.
    pub fn save_value_outputs(&self) -> Option<Map<String, Value>> {
        if !self.formal_outputs_committed {
            return None;
        }
        Some(
            self.ports
                .iter()
                .filter_map(|port| {
                    let Output::Value(value) = &port.value else {
                        return None;
                    };
                    Some((port.name.clone(), value.clone()))
                })
                .collect(),
        )
    }
    /// Share only pixels/original PNG owners; no buffer copies or UI-thread encoding.
    pub fn save_output_images(
        &self,
    ) -> Vec<(String, crate::desktop::loom_image_value::ImageValue)> {
        self.ports
            .iter()
            .filter_map(|port| {
                port.value
                    .image_value()
                    .map(|image| (port.name.clone(), image))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{desktop::loom_output_batch::Port, workspace_id::UnitId};
    #[test]
    fn mixed_output_snapshot_shares_pixels_and_keeps_json_separate() {
        let mut results = Results::new(UnitId::parse("node:test").unwrap());
        results.formal_outputs_committed = true;
        let image = crate::desktop::image::Image {
            width: 1,
            height: 1,
            pixels: vec![10, 20, 30, 255].into(),
        };
        results.ports.push(Port {
            name: "output_image".into(),
            value: Output::Image(image.clone()),
        });
        results.ports.push(Port {
            name: "text".into(),
            value: Output::Value("hello".into()),
        });
        let images = results.save_output_images();
        assert_eq!(images.len(), 1);
        assert!(images[0].1.pixels.same_storage(&image.pixels));
        assert_eq!(results.save_value_outputs().unwrap().len(), 1);
        results.ports.clear();
        assert_eq!(images[0].1.pixels.as_ref(), &[10, 20, 30, 255]);
    }
    #[test]
    fn untouched_and_explicit_empty_outputs_have_distinct_save_semantics() {
        let mut results = Results::new(UnitId::parse("node:test").unwrap());
        assert!(results.save_value_outputs().is_none());
        results.formal_outputs_committed = true;
        assert_eq!(results.save_value_outputs(), Some(Map::new()));
        let request = crate::graph_requests::Request {
            id: UnitId::parse("request:next").unwrap(),
            generation: 1,
        };
        results.begin(request.clone());
        assert!(results.cancel(&request));
        assert_eq!(results.save_value_outputs(), Some(Map::new()));
    }
    #[test]
    fn snapshot_keeps_exact_values_and_survives_later_result_mutation() {
        let mut results = Results::new(UnitId::parse("node:test").unwrap());
        results.formal_outputs_committed = true;
        for (name, value) in [
            ("null", Value::Null),
            ("false", Value::Bool(false)),
            (
                "resource",
                serde_json::json!({"uri":"opaque:test","extension":{"keep":0}}),
            ),
            (
                "stream",
                serde_json::json!({"streamId":"stream:test","sequence":0}),
            ),
        ] {
            results.ports.push(Port {
                name: name.into(),
                value: Output::Value(value),
            });
        }
        let saved = results.save_value_outputs().unwrap();
        results.ports.clear();
        assert_eq!(saved.len(), 4);
        assert_eq!(saved["null"], Value::Null);
        assert_eq!(saved["false"], false);
        assert_eq!(saved["resource"]["extension"]["keep"], 0);
        assert_eq!(saved["stream"]["sequence"], 0);
    }
}

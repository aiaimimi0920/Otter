//! 本地 Art 的正式端口替换；输入已由签名执行 owner 复核，不借用远端 completion。
use super::*;
use crate::surface_javascript_wire::result::{Resolved, ResolvedOutput};
impl Results {
    #[cfg(test)]
    pub(crate) fn apply_local_result(
        &mut self,
        expected: &OutputStamp,
        result: Resolved,
    ) -> Result<(), Error> {
        self.apply_local_result_if(expected, result, &|| true)
    }
    pub(crate) fn apply_local_result_if(
        &mut self,
        expected: &OutputStamp,
        result: Resolved,
        allowed: &dyn Fn() -> bool,
    ) -> Result<(), Error> {
        if !expected.matches(self) || self.active.is_some() || self.surface.is_some() {
            return Err(Error::Stale);
        }
        if result.outputs.len() > 128 {
            return Err(Error::Capacity);
        }
        let mut ports = Vec::new();
        ports
            .try_reserve_exact(result.outputs.len())
            .map_err(|_| Error::Allocation)?;
        let mut bytes = Meter(0);
        for (name, value) in result.outputs {
            let value = match value {
                ResolvedOutput::Image(image) => {
                    bytes.add(image.pixels.len()).map_err(|_| Error::Budget)?;
                    Output::Image(image)
                }
                ResolvedOutput::Value(value) => {
                    serde_json::to_writer(&mut bytes, &value).map_err(|_| Error::Budget)?;
                    Output::Value(value)
                }
            };
            ports.push(Port { name, value });
        }
        let display = ["output_image", "output"]
            .iter()
            .find_map(|name| {
                ports
                    .iter()
                    .find(|port| port.name == *name)
                    .and_then(|port| port.value.image())
            })
            .or_else(|| ports.iter().find_map(|port| port.value.image()))
            .cloned();
        let stamp = OutputStamp(Arc::new(()));
        if !allowed() {
            return Err(Error::Cancelled);
        }
        // 完整替换（包括空结果）；全部可失败准备先结束，随后没有 UI 重入或外部回调。
        self.ports = ports;
        self.output_stamp = stamp;
        self.clear_candidates();
        self.formal_outputs_committed = true;
        self.display = display;
        self.workflow_preview = None;
        self.status = Status::Completed;
        self.progress = Some(1.0);
        self.error = None;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_at_commit_fence_preserves_ports_and_output_identity() {
        let mut state = Results::new(UnitId::parse("art:test").unwrap());
        let original = state.output_stamp();
        assert_eq!(
            state.apply_local_result_if(
                &original,
                Resolved {
                    outputs: Default::default()
                },
                &|| false
            ),
            Err(Error::Cancelled)
        );
        assert!(original.matches(&state));
        assert!(state.ports().is_empty());
        assert!(state.save_value_outputs().is_none());
    }
    #[test]
    fn local_result_replaces_atomically_and_enters_existing_save_owners() {
        let mut state = Results::new(UnitId::parse("art:test").unwrap());
        let old = state.output_stamp();
        let pixels: crate::pixels::Pixels = vec![1, 2, 3, 255].into();
        let result = Resolved {
            outputs: [
                (
                    "output".into(),
                    ResolvedOutput::Image(Image {
                        width: 1,
                        height: 1,
                        pixels: pixels.clone(),
                    }),
                ),
                (
                    "data".into(),
                    ResolvedOutput::Value(serde_json::Value::Null),
                ),
            ]
            .into_iter()
            .collect(),
        };
        state.apply_local_result(&old, result).unwrap();
        assert_eq!(
            state.save_value_outputs().unwrap()["data"],
            serde_json::Value::Null
        );
        assert!(state.save_output_images()[0].1.pixels.same_storage(&pixels));
        assert!(!old.matches(&state));
        assert_eq!(
            state.apply_local_result(
                &old,
                Resolved {
                    outputs: Default::default()
                }
            ),
            Err(Error::Stale)
        );
        assert_eq!(state.ports().len(), 2);
        state
            .apply_local_result(
                &state.output_stamp(),
                Resolved {
                    outputs: Default::default(),
                },
            )
            .unwrap();
        assert!(state.ports().is_empty());
        assert!(state.save_value_outputs().unwrap().is_empty());
        assert!(state.display().is_none());
    }
}

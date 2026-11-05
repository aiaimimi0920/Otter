//! Surface 宿主能力报告的有界 wire 合同；没有默认全能力，调用方只能列出实际实现。
use serde::Serialize;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Runtime {
    #[default]
    Declarative,
    Javascript,
    Shader,
    LoomRemote,
}
#[derive(Clone, Copy, Default, Serialize)]
pub struct Input {
    pub pointer: bool,
    pub hover: bool,
    pub touch: bool,
    pub keyboard: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    api_version: &'static str,
    runtimes: Vec<Runtime>,
    nodes: Vec<String>,
    transports: Vec<String>,
    capabilities: Vec<String>,
    input: Input,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Duplicate,
    Identifier,
}
fn list(values: &[String], limit: usize) -> Result<(), Error> {
    if values.len() > limit {
        return Err(Error::Limit);
    }
    for (index, value) in values.iter().enumerate() {
        if value.is_empty()
            || value.len() > 160
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b':'))
        {
            return Err(Error::Identifier);
        }
        if values[..index].contains(value) {
            return Err(Error::Duplicate);
        }
    }
    Ok(())
}
impl Report {
    pub fn supports(&self, runtime: Runtime) -> bool {
        self.runtimes.contains(&runtime)
    }
    pub fn new(
        runtimes: Vec<Runtime>,
        nodes: Vec<String>,
        transports: Vec<String>,
        capabilities: Vec<String>,
        input: Input,
    ) -> Result<Self, Error> {
        if runtimes.len() > 4 {
            return Err(Error::Limit);
        }
        for (index, runtime) in runtimes.iter().enumerate() {
            if runtimes[..index].contains(runtime) {
                return Err(Error::Duplicate);
            }
        }
        list(&nodes, 64)?;
        list(&transports, 16)?;
        list(&capabilities, 64)?;
        Ok(Self {
            api_version: "1.0",
            runtimes,
            nodes,
            transports,
            capabilities,
            input,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_never_implicitly_claims_webview_or_input_capabilities() {
        let report = Report::new(
            vec![Runtime::Declarative],
            vec!["text".into()],
            vec![],
            vec![],
            Input::default(),
        )
        .unwrap();
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value["runtimes"], serde_json::json!(["declarative"]));
        assert_eq!(value["apiVersion"], "1.0");
        assert_eq!(
            value["input"],
            serde_json::json!({"pointer":false,"hover":false,"touch":false,"keyboard":false})
        );
        assert_eq!(value["capabilities"], serde_json::json!([]));
    }
    #[test]
    fn duplicate_oversized_or_unsafe_reports_are_rejected() {
        assert!(Report::new(
            vec![Runtime::Declarative; 2],
            vec![],
            vec![],
            vec![],
            Input::default()
        )
        .is_err());
        for nodes in [
            vec!["text".into(); 65],
            vec!["bad value".into()],
            vec!["x".repeat(161)],
            vec!["text".into(); 2],
        ] {
            assert!(Report::new(vec![], nodes, vec![], vec![], Input::default()).is_err());
        }
    }
}

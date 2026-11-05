//! 编辑节点合同摘要：只忽略根节点权威 value，其余属性及子树变化使旧意图失效。
use crate::surface_scene::Node;
use ring::digest::{Context, SHA256};
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

struct HashWriter(Context);
impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub fn fingerprint(node: &Node) -> Result<[u8; 32], serde_json::Error> {
    calculate(node, true)
}
pub(crate) fn full_fingerprint(node: &Node) -> Result<[u8; 32], serde_json::Error> {
    calculate(node, false)
}
fn calculate(node: &Node, ignore_value: bool) -> Result<[u8; 32], serde_json::Error> {
    let mut output = HashWriter(Context::new(&SHA256));
    encode(&mut output, node, ignore_value)?;
    Ok(output
        .0
        .finish()
        .as_ref()
        .try_into()
        .expect("SHA256 length"))
}
fn encode(output: &mut HashWriter, node: &Node, root: bool) -> Result<(), serde_json::Error> {
    // 固定字段顺序、结构长度及 JSON 编码避免拼接歧义；流式哈希不复制整个场景。
    let props: BTreeMap<_, _> = node
        .props()
        .iter()
        .filter(|(key, _)| !root || key.as_str() != "value")
        .collect();
    serde_json::to_writer(
        &mut *output,
        &(
            node.id(),
            node.kind() as u8,
            props,
            node.layout(),
            node.style(),
            node.accessibility(),
            node.events(),
            node.children().len(),
        ),
    )?;
    for child in node.children() {
        encode(output, child, false)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_scene::Scene;
    fn hash(value: serde_json::Value) -> [u8; 32] {
        fingerprint(Scene::parse(value.to_string().as_bytes()).unwrap().root()).unwrap()
    }
    #[test]
    fn surface_edit_contract_ignores_only_root_value() {
        let source = serde_json::json!({"id":"a","type":"textarea","props":{"value":"first","maxLength":8},"events":{"input":"edit"},"children":[{"id":"child","type":"text","props":{"value":"child value"}}]});
        let mut next = source.clone();
        next["props"]["value"] = "authoritative update".into();
        assert_eq!(hash(source.clone()), hash(next.clone()));
        next["children"][0]["props"]["value"] = "changed".into();
        assert_ne!(hash(source.clone()), hash(next));
        for (key, value) in [
            ("props", serde_json::json!({"maxLength":9})),
            ("style", serde_json::json!({"color":"red"})),
            ("layout", serde_json::json!({"width":200})),
            ("accessibility", serde_json::json!({"label":"new"})),
            ("events", serde_json::json!({"input":"other"})),
        ] {
            let mut next = source.clone();
            next[key] = value;
            assert_ne!(hash(source.clone()), hash(next));
        }
    }
}

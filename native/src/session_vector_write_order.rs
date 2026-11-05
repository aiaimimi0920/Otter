//! 删除不重编号未移动对象的原层级；真正重排才重新分配顺序，新增对象接在原最大值后。
use super::*;
pub(crate) struct Plan {
    values: std::collections::HashMap<String, Value>,
}
impl Plan {
    pub(crate) fn new<'a>(editor: &Editor, old: impl Iterator<Item = &'a Value>) -> Self {
        let mut ordered = old
            .filter_map(|v| {
                Some((
                    v["id"].as_str()?,
                    v["zIndex"]
                        .as_f64()
                        .filter(|v| v.is_finite() && v.abs() <= 1_000_000.0)?,
                    &v["zIndex"],
                ))
            })
            .collect::<Vec<_>>();
        ordered.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut ranks = ordered
            .iter()
            .enumerate()
            .map(|(i, (id, _, z))| (*id, (i, (*z).clone())))
            .collect::<std::collections::HashMap<_, _>>();
        for stroke in editor.elements() {
            if let Some((z, rank)) = stroke.annotation_layer() {
                ranks
                    .entry(stroke.annotation_id())
                    .or_insert((rank, json!(z)));
            }
        }
        let mut last = None;
        let mut added = false;
        let moved = editor.elements().iter().any(|s| {
            let Some((rank, _)) = ranks.get(s.annotation_id()) else {
                added = true;
                return false;
            };
            let rank = *rank;
            let moved = added || last.is_some_and(|last| rank < last);
            last = Some(rank);
            moved
        });
        let mut next = ranks
            .values()
            .filter_map(|v| v.1.as_f64())
            .fold(-1.0, f64::max)
            + 1.0;
        let mut values = std::collections::HashMap::new();
        let added = editor
            .elements()
            .iter()
            .filter(|s| !ranks.contains_key(s.annotation_id()))
            .count();
        // 没有新增时，即使原最高层级已达上限也不重写；仅真实新增溢出才回退。
        if !moved && (added == 0 || next + added as f64 - 1.0 <= 1_000_000.0) {
            for stroke in editor.elements() {
                let z = match ranks.get(stroke.annotation_id()) {
                    Some((_, z)) => (*z).clone(),
                    None => {
                        let z = json!(next);
                        next += 1.0;
                        z
                    }
                };
                values.insert(stroke.annotation_id().to_owned(), z);
            }
        }
        Self { values }
    }
    pub(crate) fn value(&self, id: &str, index: usize) -> Value {
        self.values.get(id).cloned().unwrap_or_else(|| json!(index))
    }
}

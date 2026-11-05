//! 显示来源对齐 canvasDisplayResolution；不与执行输入混用，不修改底图或编辑历史。
use super::*;
impl<'a> Graph<'a> {
    pub fn canvas_image(&self, id: &UnitId) -> Option<&'a str> {
        let index = self.index(id)?;
        let node = &self.nodes[index];
        let mut visited = [false; MAX_UNITS];
        visited[index] = true;
        let link = self.links.iter().find(|link| {
            &link.to_unit == id
                && matches!(link.to_port.as_str(), "image" | "input_image" | "input")
        });
        let enabled = node.kind != Kind::Sticker || !node.input_disabled;
        let relay = node.kind == Kind::Sticker && enabled && node.relays();
        if relay {
            if let Some(value) = link.and_then(|link| self.connected(link, &mut visited)) {
                return Some(value);
            }
        }
        if let Some(preview) = node.preview.filter(|value| !value.is_empty()) {
            return Some(preview);
        }
        if !relay && enabled {
            if let Some(value) = link.and_then(|link| self.connected(link, &mut visited)) {
                return Some(value);
            }
        }
        node.source.filter(|value| !value.is_empty())
    }
}
#[cfg(test)]
mod tests {
    use super::super::tests::{id, link, sticker};
    use super::*;
    #[test]
    fn untouched_sticker_uses_formal_upstream_before_cached_preview() {
        let ids = [id("art"), id("sticker")];
        let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
        nodes[0].kind = Kind::Art;
        nodes[0].preview = Some("visual-preview");
        nodes[0].values = &[("output_image", Output::Image("formal"))];
        nodes[1].source = Some("original");
        nodes[1].preview = Some("cached");
        let links = [link("edge", &ids[0], &ids[1], "image")];
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().canvas_image(&ids[1]),
            Some("formal")
        );
        nodes[0].values = &[];
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().canvas_image(&ids[1]),
            Some("cached")
        );
        nodes[1].preview = None;
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().canvas_image(&ids[1]),
            Some("original")
        );
    }
    #[test]
    fn edited_sticker_and_art_keep_their_own_display_priority() {
        let ids = [id("upstream"), id("target")];
        let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
        nodes[0].source = Some("upstream");
        nodes[1].source = Some("original");
        nodes[1].preview = Some("local-edit");
        nodes[1].locally_edited = true;
        let links = [link("edge", &ids[0], &ids[1], "input_image")];
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().canvas_image(&ids[1]),
            Some("local-edit")
        );
        nodes[1].kind = Kind::Art;
        nodes[1].preview = Some("art-preview");
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().canvas_image(&ids[1]),
            Some("art-preview")
        );
        nodes[1].preview = None;
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().canvas_image(&ids[1]),
            Some("upstream")
        );
    }
    #[test]
    fn disabled_input_blocks_fallback_but_stop_receiving_preserves_main_display_fallback() {
        let ids = [id("upstream"), id("target")];
        let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
        nodes[0].source = Some("upstream");
        nodes[1].source = Some("original");
        nodes[1].accept_upstream = false;
        let links = [link("edge", &ids[0], &ids[1], "input")];
        // 主干停止中继但没有本地预览时仍允许显示连接图，与执行中继语义不同。
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().canvas_image(&ids[1]),
            Some("upstream")
        );
        nodes[1].input_disabled = true;
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().canvas_image(&ids[1]),
            Some("original")
        );
    }
    #[test]
    fn unrelated_ports_are_ignored_and_cycles_keep_local_fallback() {
        let ids = [id("a"), id("b")];
        let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
        nodes[0].source = Some("a-local");
        nodes[1].source = None;
        let links = [
            link("ab", &ids[0], &ids[1], "image"),
            link("ba", &ids[1], &ids[0], "image"),
        ];
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().canvas_image(&ids[0]),
            Some("a-local")
        );
        let unrelated = [link("not-image", &ids[0], &ids[1], "amount")];
        assert_eq!(
            Graph::new(&nodes, &unrelated)
                .unwrap()
                .canvas_image(&ids[1]),
            None
        );
        assert_eq!(
            Graph::new(&nodes, &links)
                .unwrap()
                .canvas_image(&id("missing")),
            None
        );
    }
}

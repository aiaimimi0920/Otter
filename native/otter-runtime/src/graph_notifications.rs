//! Formal-result notifications pass through available image relays to the next Art; no display updates or node execution.
use super::*;
impl<'a> Graph<'a> {
    pub fn notification_targets(&self, source: &UnitId) -> Result<Vec<&'a UnitId>, Error> {
        self.notification_targets_with(source, |_| true)
    }
    /// Notification policy restricts outward propagation only; it does not change image resolution, display refresh, or port connectivity.
    pub fn notification_targets_with(
        &self,
        source: &UnitId,
        notify: impl Fn(&UnitId) -> bool,
    ) -> Result<Vec<&'a UnitId>, Error> {
        let Some(root) = self.index(source) else {
            return Ok(Vec::new());
        };
        let mut queue = Vec::new();
        let mut targets = Vec::new();
        queue
            .try_reserve(self.nodes.len())
            .map_err(|_| Error::Allocation)?;
        targets
            .try_reserve(self.nodes.len())
            .map_err(|_| Error::Allocation)?;
        let mut visited = [false; MAX_UNITS];
        queue.push(root);
        visited[root] = true;
        let mut next = 0;
        while next < queue.len() {
            let node = &self.nodes[queue[next]];
            if !notify(node.id) {
                next += 1;
                continue;
            }
            for link in self.links.iter().filter(|link| &link.from_unit == node.id) {
                let Some(index) = self.index(&link.to_unit) else {
                    continue;
                };
                if visited[index] {
                    continue;
                }
                let child = &self.nodes[index];
                if child.kind == Kind::Art {
                    visited[index] = true;
                    targets.push(child.id);
                } else if child.kind == Kind::Sticker
                    && child.relays()
                    && child.image_input(&link.to_port)
                    && self.connected(link, &mut [false; MAX_UNITS]).is_some()
                {
                    visited[index] = true;
                    queue.push(index);
                }
            }
            next += 1;
        }
        Ok(targets)
    }
}
#[cfg(test)]
#[path = "graph_notification_policy_tests.rs"]
mod policy_tests;
#[cfg(test)]
mod tests {
    use super::super::tests::{id, link, sticker};
    use super::*;
    #[test]
    fn formal_images_relay_but_preview_scalar_and_disabled_stickers_do_not() {
        let ids = [id("source"), id("relay"), id("target")];
        let mut nodes = ids.iter().map(sticker).collect::<Vec<_>>();
        nodes[0].kind = Kind::Art;
        nodes[0].preview = Some("preview");
        nodes[2].kind = Kind::Art;
        let links = [
            link("a", &ids[0], &ids[1], "image"),
            link("b", &ids[1], &ids[2], "input"),
        ];
        assert!(Graph::new(&nodes, &links)
            .unwrap()
            .notification_targets(&ids[0])
            .unwrap()
            .is_empty());
        nodes[0].values = &[("output_image", Output::Number(7.0))];
        assert!(Graph::new(&nodes, &links)
            .unwrap()
            .notification_targets(&ids[0])
            .unwrap()
            .is_empty());
        nodes[0].values = &[("output_image", Output::Image("formal"))];
        assert_eq!(
            Graph::new(&nodes, &links)
                .unwrap()
                .notification_targets(&ids[0])
                .unwrap(),
            vec![&ids[2]]
        );
        nodes[1].accept_upstream = false;
        assert!(Graph::new(&nodes, &links)
            .unwrap()
            .notification_targets(&ids[0])
            .unwrap()
            .is_empty());
        nodes[1].accept_upstream = true;
        nodes[1].locally_edited = true;
        assert!(Graph::new(&nodes, &links)
            .unwrap()
            .notification_targets(&ids[0])
            .unwrap()
            .is_empty());
    }
    #[test]
    fn repeated_edges_and_imported_cycles_visit_each_node_once() {
        let ids = [id("source"), id("relay"), id("target")];
        let mut nodes = ids.iter().map(sticker).collect::<Vec<_>>();
        nodes[0].kind = Kind::Art;
        nodes[0].values = &[("output_image", Output::Image("formal"))];
        nodes[2].kind = Kind::Art;
        let links = [
            link("a", &ids[0], &ids[1], "image"),
            link("b", &ids[1], &ids[2], "input"),
            link("c", &ids[0], &ids[2], "other"),
            link("cycle", &ids[1], &ids[0], "input"),
        ];
        assert_eq!(
            Graph::new(&nodes, &links)
                .unwrap()
                .notification_targets(&ids[0])
                .unwrap(),
            vec![&ids[2]]
        );
    }
}

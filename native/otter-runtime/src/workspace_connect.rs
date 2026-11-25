//! User-connection transaction: validate/reserve before replacing a single-input edge; imports may still use add_link to preserve legacy graph semantics.
use super::*;

impl Workspace {
    /// Match baseline useLinking self-connection/cycle rejection and single-input rules, without owning port-capability type validation.
    /// Unlike deleting the old edge before adding, any failure here preserves the original graph intact.
    pub fn connect(&mut self, link: Link) -> Result<Option<Link>, Error> {
        if !self.contains(&link.from_unit) || !self.contains(&link.to_unit) {
            return Err(Error::MissingUnit);
        }
        if !valid_port(&link.from_port) || !valid_port(&link.to_port) {
            return Err(Error::InvalidPort);
        }
        if link.from_unit == link.to_unit {
            return Err(Error::SelfConnection);
        }
        if self.links.iter().any(|existing| existing.id == link.id) {
            return Err(Error::DuplicateId);
        }
        // The baseline checks target -> source in the existing graph before replacing input. Traversal must terminate even with existing directed cycles.
        if self.reachable(&link.to_unit, &link.from_unit) {
            return Err(Error::Cycle);
        }
        let replaced = self.links.iter().position(|existing| {
            existing.to_unit == link.to_unit && existing.to_port == link.to_port
        });
        if replaced.is_none() && self.links.len() >= MAX_LINKS {
            return Err(Error::Capacity);
        }
        if replaced.is_none() {
            self.links.try_reserve(1).map_err(|_| Error::Allocation)?;
        }
        // The baseline removes the first occupying edge, then appends the new one; preserve order/content of all other edges in an existing imported graph.
        self.topology = Arc::new(());
        let previous = replaced.map(|index| self.links.remove(index));
        self.links.push(link);
        Ok(previous)
    }

    fn reachable(&self, from: &UnitId, to: &UnitId) -> bool {
        let Some(start) = self.units.iter().position(|unit| unit == from) else {
            return false;
        };
        // Object count is at most MAX_UNITS; use a fixed stack buffer instead of allocating a BFS container for every mouse drop point.
        let mut queue = [0usize; MAX_UNITS];
        let mut visited = [false; MAX_UNITS];
        let (mut head, mut tail) = (0, 1);
        queue[0] = start;
        visited[start] = true;
        while head < tail {
            let unit = &self.units[queue[head]];
            head += 1;
            if unit == to {
                return true;
            }
            for link in self.links.iter().filter(|link| link.from_unit == *unit) {
                let Some(next) = self.units.iter().position(|unit| *unit == link.to_unit) else {
                    continue;
                };
                if !visited[next] {
                    visited[next] = true;
                    queue[tail] = next;
                    tail += 1;
                }
            }
        }
        false
    }
}

#[cfg(test)]
#[path = "workspace_connect_tests.rs"]
mod tests;

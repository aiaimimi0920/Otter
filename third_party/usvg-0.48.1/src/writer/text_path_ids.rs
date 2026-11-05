//! Export-local path identities: source IDs need not identify resolved geometry.
use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(super) struct Context<'a> {
    options: &'a WriteOptions,
    pub paths: Vec<(Arc<TextPath>, String)>,
    indices: HashMap<usize, usize>,
}

impl std::ops::Deref for Context<'_> {
    type Target = WriteOptions;
    fn deref(&self) -> &Self::Target {
        self.options
    }
}

impl<'a> Context<'a> {
    pub fn new(tree: &Tree, options: &'a WriteOptions) -> Self {
        let mut context = Self {
            options,
            paths: Vec::new(),
            indices: HashMap::new(),
        };
        if !options.preserve_text {
            return context;
        }
        let mut reserved = HashSet::new();
        for id in tree
            .linear_gradients()
            .iter()
            .map(|v| v.id())
            .chain(tree.radial_gradients().iter().map(|v| v.id()))
            .chain(tree.patterns().iter().map(|v| v.id()))
            .chain(tree.clip_paths().iter().map(|v| v.id()))
            .chain(tree.masks().iter().map(|v| v.id()))
            .chain(tree.filters().iter().map(|v| v.id()))
        {
            reserved.insert(id.to_owned());
        }
        context.collect(&tree.root, &mut reserved);
        let mut serial = 0usize;
        for (_, id) in &mut context.paths {
            loop {
                let candidate = format!("otter_text_path_{serial}");
                serial += 1;
                if reserved.insert(candidate.clone()) {
                    *id = candidate;
                    break;
                }
            }
        }
        context
    }

    fn collect(&mut self, group: &Group, reserved: &mut HashSet<String>) {
        reserved.insert(group.id().to_owned());
        for node in &group.children {
            reserved.insert(node.id().to_owned());
            match node {
                Node::Group(group) => self.collect(group, reserved),
                Node::Text(text) => {
                    for chunk in &text.chunks {
                        if let TextFlow::Path(path) = &chunk.text_flow {
                            reserved.insert(path.id().to_owned());
                            let key = Arc::as_ptr(path) as usize;
                            if !self.indices.contains_key(&key) {
                                self.indices.insert(key, self.paths.len());
                                self.paths.push((path.clone(), String::new()));
                            }
                        }
                    }
                }
                _ => {}
            }
            node.subroots(|root| self.collect(root, reserved));
        }
    }

    pub fn path_id(&self, path: &Arc<TextPath>) -> &str {
        // Collection and emission use the same immutable tree and Arc identity.
        &self.paths[self.indices[&(Arc::as_ptr(path) as usize)]].1
    }
}

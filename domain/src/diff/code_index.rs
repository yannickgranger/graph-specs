use crate::ConceptNode;
use std::collections::HashMap;

pub(super) struct CodeIndex {
    by_name: HashMap<String, Vec<ConceptNode>>,
    contexts_declared: bool,
}

impl CodeIndex {
    pub(super) fn new(nodes: Vec<ConceptNode>, contexts_declared: bool) -> Self {
        let mut by_name: HashMap<String, Vec<ConceptNode>> = HashMap::new();
        for node in nodes {
            by_name.entry(node.name.clone()).or_default().push(node);
        }
        for items in by_name.values_mut() {
            items.sort_by(|a, b| a.unit().cmp(&b.unit()));
        }
        Self {
            by_name,
            contexts_declared,
        }
    }

    pub(super) fn contains(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    pub(super) fn names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }

    pub(super) fn take_for(&mut self, spec_node: &ConceptNode) -> Option<ConceptNode> {
        let items = self.by_name.get_mut(spec_node.name.as_str())?;
        if items.is_empty() {
            return None;
        }
        let index = match spec_node.context().filter(|_| self.contexts_declared) {
            Some(ctx) => items.iter().position(|item| item.context() == Some(ctx))?,
            None => 0,
        };
        let taken = items.remove(index);
        if items.is_empty() {
            self.by_name.remove(spec_node.name.as_str());
        }
        Some(taken)
    }

    pub(super) fn into_remaining(self) -> Vec<ConceptNode> {
        let mut out: Vec<ConceptNode> = self.by_name.into_values().flatten().collect();
        out.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.unit().cmp(&b.unit()))
                .then_with(|| a.source.path().cmp(b.source.path()))
        });
        out
    }
}

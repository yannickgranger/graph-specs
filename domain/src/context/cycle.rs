use crate::{ContextDecl, ContextPattern};
use std::collections::HashMap;

#[must_use]
pub fn detect_import_cycle(contexts: &[ContextDecl]) -> Option<Vec<String>> {
    use std::collections::HashSet;

    let adj: HashMap<&str, Vec<&str>> = contexts
        .iter()
        .map(|c| {
            let edges: Vec<&str> = c
                .imports
                .iter()
                .filter(|i| i.pattern != ContextPattern::SharedKernel)
                .map(|i| i.from_context.as_str())
                .collect();
            (c.name.as_str(), edges)
        })
        .collect();

    let mut visited: HashSet<&str> = HashSet::new();
    let mut stack: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();

    for start in adj.keys() {
        if visited.contains(start) {
            continue;
        }
        if let Some(cycle) = dfs_cycle(start, &adj, &mut visited, &mut stack, &mut path) {
            return Some(cycle.into_iter().map(String::from).collect());
        }
    }
    None
}

fn dfs_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
    stack: &mut std::collections::HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<&'a str>> {
    visited.insert(node);
    stack.insert(node);
    path.push(node);
    if let Some(neighbours) = adj.get(node) {
        for &next in neighbours {
            if let Some(cycle) = visit_neighbour(next, adj, visited, stack, path) {
                return Some(cycle);
            }
        }
    }
    stack.remove(node);
    path.pop();
    None
}

fn visit_neighbour<'a>(
    next: &'a str,
    adj: &HashMap<&'a str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
    stack: &mut std::collections::HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<&'a str>> {
    if !adj.contains_key(next) {
        return None;
    }
    if stack.contains(next) {
        let start = path.iter().position(|&n| n == next).unwrap_or(0);
        return Some(path[start..].to_vec());
    }
    if visited.contains(next) {
        return None;
    }
    dfs_cycle(next, adj, visited, stack, path)
}

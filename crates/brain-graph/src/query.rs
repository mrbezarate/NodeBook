//! Запросы к графу знаний.
use crate::edge::Edge;
use std::collections::{HashMap, HashSet, VecDeque};

pub fn find_neighbors(edges: &[Edge], id: &str) -> Vec<String> {
    let mut neighbors = HashSet::new();
    for e in edges {
        if e.from == id { neighbors.insert(e.to.clone()); }
        if e.to == id { neighbors.insert(e.from.clone()); }
    }
    neighbors.into_iter().collect()
}

pub fn bfs_path(edges: &[Edge], from: &str, to: &str) -> Option<Vec<String>> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in edges {
        adj.entry(e.from.as_str()).or_default().push(e.to.as_str());
        adj.entry(e.to.as_str()).or_default().push(e.from.as_str());
    }
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut parent: HashMap<&str, &str> = HashMap::new();
    queue.push_back(from);
    visited.insert(from);
    while let Some(node) = queue.pop_front() {
        if node == to {
            let mut path = vec![to.to_string()];
            let mut cur = to;
            while let Some(&p) = parent.get(cur) { path.push(p.to_string()); cur = p; }
            path.reverse();
            return Some(path);
        }
        if let Some(neighbors) = adj.get(node) {
            for &n in neighbors {
                if visited.insert(n) { parent.insert(n, node); queue.push_back(n); }
            }
        }
    }
    None
}

pub fn most_connected(edges: &[Edge], limit: usize) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for e in edges { *counts.entry(e.from.clone()).or_default() += 1; *counts.entry(e.to.clone()).or_default() += 1; }
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.truncate(limit);
    sorted
}

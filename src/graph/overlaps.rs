use std::collections::{HashMap, HashSet};

use crate::graph::model::{GraphData, NodeData, NodeId};

struct Aabb {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

fn aabb_from_node_data(node: &NodeData) -> Aabb {
    let cx = node.position.x;
    let cy = node.position.y;
    let w = node.size.x;
    let h = node.size.y;
    let hw = w * 0.5;
    let hh = h * 0.5;
    Aabb {
        min_x: cx - hw,
        max_x: cx + hw,
        min_y: cy - hh,
        max_y: cy + hh,
    }
}

/// Positive-area intersection (edge-only touching does not count).
fn aabbs_overlap(a: &Aabb, b: &Aabb) -> bool {
    a.min_x < b.max_x
        && a.max_x > b.min_x
        && a.min_y < b.max_y
        && a.max_y > b.min_y
}

/// Other nodes whose axis-aligned bounds intersect this node's bounds with positive area.
pub fn overlapping_node_ids(graph: &GraphData, node_id: NodeId) -> Vec<NodeId> {
    let Some(self_data) = graph.nodes.get(&node_id) else {
        return Vec::new();
    };
    let self_aabb = aabb_from_node_data(self_data);
    let mut out = Vec::new();
    for (&other_id, other_data) in graph.nodes.iter() {
        if other_id == node_id {
            continue;
        }
        let other_aabb = aabb_from_node_data(other_data);
        if aabbs_overlap(&self_aabb, &other_aabb) {
            out.push(other_id);
        }
    }
    out.sort_by_key(|id| id.0);
    out
}

/// Number of nodes that overlap at least one other node (positive-area bounds intersection).
pub fn overlapping_node_count(graph: &GraphData) -> usize {
    let ids: Vec<NodeId> = graph.nodes.keys().copied().collect();
    let mut with_overlap: HashSet<NodeId> = HashSet::new();
    for i in 0..ids.len() {
        let a_id = ids[i];
        let Some(a_data) = graph.nodes.get(&a_id) else {
            continue;
        };
        let aabb_a = aabb_from_node_data(a_data);
        for j in (i + 1)..ids.len() {
            let b_id = ids[j];
            let Some(b_data) = graph.nodes.get(&b_id) else {
                continue;
            };
            let aabb_b = aabb_from_node_data(b_data);
            if aabbs_overlap(&aabb_a, &aabb_b) {
                with_overlap.insert(a_id);
                with_overlap.insert(b_id);
            }
        }
    }
    with_overlap.len()
}

/// Symmetric overlap adjacency: each key lists all other `NodeId`s that overlap it.
pub fn overlapping_pairs_by_node(graph: &GraphData) -> HashMap<NodeId, Vec<NodeId>> {
    let ids: Vec<NodeId> = graph.nodes.keys().copied().collect();
    let mut map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for i in 0..ids.len() {
        let a_id = ids[i];
        let Some(a_data) = graph.nodes.get(&a_id) else {
            continue;
        };
        let aabb_a = aabb_from_node_data(a_data);
        for j in (i + 1)..ids.len() {
            let b_id = ids[j];
            let Some(b_data) = graph.nodes.get(&b_id) else {
                continue;
            };
            let aabb_b = aabb_from_node_data(b_data);
            if aabbs_overlap(&aabb_a, &aabb_b) {
                map.entry(a_id).or_default().push(b_id);
                map.entry(b_id).or_default().push(a_id);
            }
        }
    }
    for list in map.values_mut() {
        list.sort_by_key(|id| id.0);
    }
    map
}

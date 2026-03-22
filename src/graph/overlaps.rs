use std::collections::{HashMap, HashSet};

use bevy::prelude::Vec2;

use crate::graph::model::{GraphData, NodeData, NodeId};

struct Aabb {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

fn aabb_at(position: Vec2, size: Vec2) -> Aabb {
    let cx = position.x;
    let cy = position.y;
    let w = size.x;
    let h = size.y;
    let hw = w * 0.5;
    let hh = h * 0.5;
    Aabb {
        min_x: cx - hw,
        max_x: cx + hw,
        min_y: cy - hh,
        max_y: cy + hh,
    }
}

fn aabb_from_node_data(node: &NodeData) -> Aabb {
    aabb_at(node.position, node.size)
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

const RESOLVE_MAX_ITERS: usize = 64;
const SEP_EPS: f32 = 0.5;

fn has_temp_overlap(graph: &GraphData, positions: &HashMap<NodeId, Vec2>) -> bool {
    let ids: Vec<NodeId> = graph.nodes.keys().copied().collect();
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let a_id = ids[i];
            let b_id = ids[j];
            let Some(a_data) = graph.nodes.get(&a_id) else {
                continue;
            };
            let Some(b_data) = graph.nodes.get(&b_id) else {
                continue;
            };
            let pos_a = positions[&a_id];
            let pos_b = positions[&b_id];
            let aabb_a = aabb_at(pos_a, a_data.size);
            let aabb_b = aabb_at(pos_b, b_data.size);
            if aabbs_overlap(&aabb_a, &aabb_b) {
                return true;
            }
        }
    }
    false
}

/// Iteratively separates overlapping node bounds (same rules as [`overlapping_node_count`]).
/// Returns only nodes whose center position changed.
pub fn resolve_overlapping_positions(graph: &GraphData) -> HashMap<NodeId, Vec2> {
    let ids: Vec<NodeId> = graph.nodes.keys().copied().collect();
    let mut positions: HashMap<NodeId, Vec2> = HashMap::new();
    for &id in &ids {
        if let Some(n) = graph.nodes.get(&id) {
            positions.insert(id, n.position);
        }
    }

    for _ in 0..RESOLVE_MAX_ITERS {
        if !has_temp_overlap(graph, &positions) {
            break;
        }
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let a_id = ids[i];
                let b_id = ids[j];
                let Some(a_data) = graph.nodes.get(&a_id) else {
                    continue;
                };
                let Some(b_data) = graph.nodes.get(&b_id) else {
                    continue;
                };
                let pos_a = positions[&a_id];
                let pos_b = positions[&b_id];
                let aabb_a = aabb_at(pos_a, a_data.size);
                let aabb_b = aabb_at(pos_b, b_data.size);
                if !aabbs_overlap(&aabb_a, &aabb_b) {
                    continue;
                }

                let pen_x = aabb_a.max_x.min(aabb_b.max_x) - aabb_a.min_x.max(aabb_b.min_x);
                let pen_y = aabb_a.max_y.min(aabb_b.max_y) - aabb_a.min_y.max(aabb_b.min_y);
                if pen_x <= 0.0 || pen_y <= 0.0 {
                    continue;
                }

                if pen_x < pen_y {
                    let total = pen_x + SEP_EPS;
                    let half = total * 0.5;
                    let dir = if pos_a.x >= pos_b.x { 1.0 } else { -1.0 };
                    let delta = Vec2::new(dir * half, 0.0);
                    let new_a = pos_a + delta;
                    let new_b = pos_b - delta;
                    positions.insert(a_id, new_a);
                    positions.insert(b_id, new_b);
                } else {
                    let total = pen_y + SEP_EPS;
                    let half = total * 0.5;
                    let dir = if pos_a.y >= pos_b.y { 1.0 } else { -1.0 };
                    let delta = Vec2::new(0.0, dir * half);
                    let new_a = pos_a + delta;
                    let new_b = pos_b - delta;
                    positions.insert(a_id, new_a);
                    positions.insert(b_id, new_b);
                }
            }
        }
    }

    let mut out = HashMap::new();
    for &id in &ids {
        let Some(orig) = graph.nodes.get(&id) else {
            continue;
        };
        let new_pos = positions[&id];
        if (orig.position - new_pos).length_squared() > 1e-6 {
            out.insert(id, new_pos);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Color;

    fn node_at(pos: Vec2, size: Vec2) -> NodeData {
        NodeData {
            position: pos,
            name: "n".into(),
            data: String::new(),
            color: Color::WHITE,
            size,
            entity: None,
        }
    }

    #[test]
    fn resolve_separates_two_overlapping_nodes() {
        let mut graph = GraphData::default();
        let a = NodeId(0);
        let b = NodeId(1);
        let size = Vec2::new(100.0, 40.0);
        graph.nodes.insert(a, node_at(Vec2::ZERO, size));
        graph.nodes.insert(b, node_at(Vec2::new(10.0, 0.0), size));
        graph.adjacency.entry(a).or_default();
        graph.adjacency.entry(b).or_default();

        assert!(overlapping_node_count(&graph) > 0);
        let resolved = resolve_overlapping_positions(&graph);
        assert!(!resolved.is_empty());

        let mut positions: HashMap<NodeId, Vec2> = graph
            .nodes
            .iter()
            .map(|(&id, n)| (id, n.position))
            .collect();
        for (id, p) in &resolved {
            positions.insert(*id, *p);
        }
        assert!(!super::has_temp_overlap(&graph, &positions));
    }
}

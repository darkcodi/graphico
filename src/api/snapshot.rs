use std::collections::HashMap;

use uuid::Uuid;

use crate::graph::model::GraphData;

use super::state::{ApiNode, Coordinates, NodeUuidRegistry, color_to_hex};

struct Aabb {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

fn aabb_from_center_size(cx: f32, cy: f32, w: f32, h: f32) -> Aabb {
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

/// Build the API-facing snapshot from the live graph (single source of truth for persistence).
pub fn build_api_snapshot(
    graph: &GraphData,
    registry: &NodeUuidRegistry,
) -> HashMap<Uuid, ApiNode> {
    let mut entries: Vec<(Uuid, Aabb)> = Vec::new();
    for (uuid, &node_id) in &registry.uuid_to_node {
        let Some(node_data) = graph.nodes.get(&node_id) else {
            continue;
        };
        let aabb = aabb_from_center_size(
            node_data.position.x,
            node_data.position.y,
            node_data.size.x,
            node_data.size.y,
        );
        entries.push((*uuid, aabb));
    }

    let mut overlap_lists: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            if aabbs_overlap(&entries[i].1, &entries[j].1) {
                let u_i = entries[i].0;
                let u_j = entries[j].0;
                overlap_lists.entry(u_i).or_default().push(u_j);
                overlap_lists.entry(u_j).or_default().push(u_i);
            }
        }
    }
    for list in overlap_lists.values_mut() {
        list.sort();
    }

    let mut state = HashMap::new();

    for (uuid, &node_id) in &registry.uuid_to_node {
        let Some(node_data) = graph.nodes.get(&node_id) else {
            continue;
        };

        let mut neighbor_uuids = Vec::new();
        if let Some(edge_ids) = graph.adjacency.get(&node_id) {
            for edge_id in edge_ids {
                if let Some(edge) = graph.edges.get(edge_id) {
                    let other_id = if edge.source == node_id {
                        edge.target
                    } else {
                        edge.source
                    };
                    if let Some(other_uuid) = registry.node_to_uuid.get(&other_id)
                        && !neighbor_uuids.contains(other_uuid)
                    {
                        neighbor_uuids.push(*other_uuid);
                    }
                }
            }
        }

        let overlaps = overlap_lists.get(uuid).cloned().unwrap_or_default();

        state.insert(
            *uuid,
            ApiNode {
                id: *uuid,
                name: node_data.name.clone(),
                data: node_data.data.clone(),
                color: color_to_hex(&node_data.color),
                edges: neighbor_uuids,
                position: Coordinates {
                    x: node_data.position.x,
                    y: node_data.position.y,
                },
                size: Coordinates {
                    x: node_data.size.x,
                    y: node_data.size.y,
                },
                overlaps,
            },
        );
    }

    state
}

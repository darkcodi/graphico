use std::collections::HashMap;

use uuid::Uuid;

use crate::graph::model::GraphData;
use crate::graph::overlaps::overlapping_pairs_by_node;

use super::state::{ApiNode, Coordinates, NodeUuidRegistry, color_to_hex};

/// Build the API-facing snapshot from the live graph (single source of truth for persistence).
pub fn build_api_snapshot(
    graph: &GraphData,
    registry: &NodeUuidRegistry,
) -> HashMap<Uuid, ApiNode> {
    let overlap_by_node = overlapping_pairs_by_node(graph);

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

        let mut overlaps: Vec<Uuid> = overlap_by_node
            .get(&node_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|nid| registry.node_to_uuid.get(nid).copied())
                    .collect()
            })
            .unwrap_or_default();
        overlaps.sort();

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

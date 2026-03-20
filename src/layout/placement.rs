use bevy::prelude::*;
use rand::Rng;

use crate::graph::model::GraphData;

use super::engine::LayoutEngine;
use super::params::LayoutParams;

/// Detect newly added nodes (not yet tracked by the engine) and place them
/// near the centroid of their already-positioned neighbors. Isolated nodes
/// go to a fallback ring around the origin.
///
/// Also detects edge-count changes. Any topology change triggers a
/// synchronous layout solve that runs to completion within this frame.
pub fn place_new_nodes(
    mut graph: ResMut<GraphData>,
    mut engine: ResMut<LayoutEngine>,
    params: Res<LayoutParams>,
) {
    if !params.enabled {
        return;
    }

    let mut topology_changed = false;

    // Detect edge-only topology changes
    let current_edge_count = graph.edge_count();
    if current_edge_count != engine.last_edge_count {
        engine.last_edge_count = current_edge_count;
        topology_changed = true;
    }

    // Detect new nodes
    let new_ids: Vec<_> = graph
        .nodes
        .keys()
        .filter(|id| !engine.knows_node(id))
        .copied()
        .collect();

    if !new_ids.is_empty() {
        topology_changed = true;

        let mut rng = rand::rng();
        let fallback_ring_radius = 500.0;
        let fallback_count = new_ids.len();

        for (i, id) in new_ids.iter().enumerate() {
            let neighbor_positions = collect_neighbor_positions(id, &graph);

            let position = if !neighbor_positions.is_empty() {
                let centroid: Vec2 = neighbor_positions.iter().copied().sum::<Vec2>()
                    / neighbor_positions.len() as f32;
                let jitter = Vec2::new(
                    rng.random_range(-params.jitter_radius..params.jitter_radius),
                    rng.random_range(-params.jitter_radius..params.jitter_radius),
                );
                centroid + jitter
            } else {
                let angle =
                    std::f32::consts::TAU * (i as f32 / fallback_count.max(1) as f32);
                let jitter = rng.random_range(0.0..params.jitter_radius);
                Vec2::new(
                    (fallback_ring_radius + jitter) * angle.cos(),
                    (fallback_ring_radius + jitter) * angle.sin(),
                )
            };

            if let Some(node) = graph.nodes.get_mut(id) {
                node.position = position;
            }

            engine.register_node(*id);
        }
    }

    if topology_changed {
        engine.solve(&mut graph, &params);
    }
}

fn collect_neighbor_positions(id: &crate::graph::model::NodeId, graph: &GraphData) -> Vec<Vec2> {
    let Some(edge_ids) = graph.adjacency.get(id) else {
        return Vec::new();
    };
    let mut positions = Vec::new();
    for eid in edge_ids {
        let Some(edge) = graph.edges.get(eid) else {
            continue;
        };
        let neighbor_id = if edge.source == *id {
            edge.target
        } else {
            edge.source
        };
        if let Some(neighbor) = graph.nodes.get(&neighbor_id) {
            positions.push(neighbor.position);
        }
    }
    positions
}

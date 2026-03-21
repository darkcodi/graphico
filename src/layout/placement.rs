use bevy::prelude::*;
use rand::Rng;

use crate::graph::model::GraphData;

use super::engine::LayoutEngine;
use super::params::LayoutParams;

/// Detect newly added nodes (not yet tracked by the engine) and place them
/// near the centroid of their already-positioned neighbors. Isolated nodes
/// go to a fallback ring around the origin.
pub fn place_new_nodes(
    mut graph: ResMut<GraphData>,
    mut engine: ResMut<LayoutEngine>,
    params: Res<LayoutParams>,
) {
    if !params.enabled {
        return;
    }

    let mut rng = rand::rng();

    let new_ids: Vec<_> = graph
        .nodes
        .keys()
        .filter(|id| !engine.knows_node(id))
        .copied()
        .collect();

    if new_ids.is_empty() {
        return;
    }

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

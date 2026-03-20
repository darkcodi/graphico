use bevy::prelude::*;

use crate::graph::components::{ChunkCoord, GraphNode};
use crate::graph::model::{GraphData, NodeId};
use crate::spatial::grid::{SpatialGrid, CHUNK_SIZE};

use super::engine::LayoutEngine;
use super::params::LayoutParams;
use super::quadtree::QuadTree;

/// Build the Barnes-Hut tree and accumulate repulsion, attraction, and gravity
/// forces for up to `work_budget` nodes per frame.
pub fn compute_layout_forces(
    graph: Res<GraphData>,
    mut engine: ResMut<LayoutEngine>,
    params: Res<LayoutParams>,
) {
    if !params.enabled || engine.is_converged || graph.nodes.is_empty() {
        return;
    }

    // -- Build flat body list for the quadtree (all nodes, always) --
    let bodies: Vec<(Vec2, f32)> = graph
        .nodes
        .values()
        .map(|n| (n.position, 1.0))
        .collect();

    let tree = QuadTree::build(&bodies);

    // -- Prepare force accumulators --
    engine.clear_forces();
    engine.ensure_force_entries();

    // -- Collect node IDs for round-robin iteration --
    let all_ids: Vec<NodeId> = engine.velocities.keys().copied().collect();
    let total = all_ids.len();
    if total == 0 {
        return;
    }

    let budget = params.work_budget.min(total);
    let cursor = engine.round_robin_cursor % total;

    // -- Compute graph centroid for gravity --
    let centroid: Vec2 = graph.nodes.values().map(|n| n.position).sum::<Vec2>()
        / graph.nodes.len() as f32;

    // -- Scaling factor: ForceAtlas2 scales repulsion by log(1+node_count) --
    let repulsion_k = params.repulsion_strength * (1.0 + total as f32).ln();

    for i in 0..budget {
        let idx = (cursor + i) % total;
        let id = all_ids[idx];

        let Some(node) = graph.nodes.get(&id) else {
            continue;
        };
        let pos = node.position;
        let mass = 1.0 + graph.adjacency.get(&id).map_or(0, |a| a.len()) as f32;

        let mut force = Vec2::ZERO;

        // Repulsion via Barnes-Hut
        force += tree.compute_repulsion(pos, mass, params.theta, repulsion_k);

        // Attraction along edges
        if let Some(edge_ids) = graph.adjacency.get(&id) {
            for eid in edge_ids {
                let Some(edge) = graph.edges.get(eid) else {
                    continue;
                };
                let neighbor_id = if edge.source == id {
                    edge.target
                } else {
                    edge.source
                };
                let Some(neighbor) = graph.nodes.get(&neighbor_id) else {
                    continue;
                };
                let diff = neighbor.position - pos;
                let dist = diff.length();
                if dist > 0.001 {
                    force += diff.normalize()
                        * params.attraction_strength
                        * (dist - params.ideal_edge_length).max(0.0);
                }
            }
        }

        // Gravity toward centroid
        let to_center = centroid - pos;
        let center_dist = to_center.length();
        if center_dist > 0.001 {
            force += to_center.normalize() * params.gravity_strength * mass * center_dist.ln().max(0.0);
        }

        if let Some(f) = engine.forces.get_mut(&id) {
            *f = force;
        }
    }

    engine.round_robin_cursor = (cursor + budget) % total;
}

/// Apply accumulated forces: update velocities, compute displacement, write
/// new positions into `GraphData`, and adapt the global speed.
pub fn apply_displacement(
    mut graph: ResMut<GraphData>,
    mut engine: ResMut<LayoutEngine>,
    params: Res<LayoutParams>,
) {
    if !params.enabled || engine.is_converged {
        return;
    }

    let mut total_swing = 0.0_f32;
    let mut total_traction = 0.0_f32;
    let mut total_displacement = 0.0_f32;
    let mut count = 0u32;

    let ids_with_forces: Vec<(NodeId, Vec2)> = engine
        .forces
        .iter()
        .filter(|(_, f)| f.length_squared() > 0.0)
        .map(|(&id, &f)| (id, f))
        .collect();

    for (id, force) in &ids_with_forces {
        let prev = engine.prev_forces.get(id).copied().unwrap_or(Vec2::ZERO);
        let swing = (*force - prev).length();
        let traction = (*force + prev).length() * 0.5;
        total_swing += swing;
        total_traction += traction;
    }

    engine.global_swing = total_swing;
    engine.global_traction = total_traction;
    engine.adapt_speed();

    let speed = engine.global_speed;

    for (id, force) in ids_with_forces {
        let vel = engine.velocities.entry(id).or_insert(Vec2::ZERO);
        *vel = *vel * params.damping + force * speed;

        let disp_len = vel.length();
        if disp_len > params.max_displacement {
            *vel = vel.normalize() * params.max_displacement;
        }

        let displacement = *vel;
        total_displacement += displacement.length();
        count += 1;

        if let Some(node) = graph.nodes.get_mut(&id) {
            node.position += displacement;
        }

        engine.prev_forces.insert(id, force);
    }

    let avg = if count > 0 {
        total_displacement / count as f32
    } else {
        0.0
    };
    engine.is_converged = avg < params.convergence_threshold && count > 0;
}

/// Sync `GraphData` positions to ECS `Transform` components and update
/// chunk assignments when a node crosses a chunk boundary.
pub fn sync_layout_positions(
    graph: Res<GraphData>,
    mut grid: ResMut<SpatialGrid>,
    mut query: Query<(&GraphNode, &mut Transform, &mut ChunkCoord)>,
) {
    for (gn, mut xform, mut chunk) in query.iter_mut() {
        let Some(node) = graph.nodes.get(&gn.id) else {
            continue;
        };

        xform.translation.x = node.position.x;
        xform.translation.y = node.position.y;

        let new_cx = (node.position.x / CHUNK_SIZE).floor() as i32;
        let new_cy = (node.position.y / CHUNK_SIZE).floor() as i32;

        if new_cx != chunk.x || new_cy != chunk.y {
            let old = IVec2::new(chunk.x, chunk.y);
            let new_chunk = IVec2::new(new_cx, new_cy);
            grid.move_node(gn.id, old, new_chunk);

            // Move edges that belong to this node between chunks
            if let Some(edge_ids) = graph.adjacency.get(&gn.id) {
                for eid in edge_ids {
                    let Some(edge) = graph.edges.get(eid) else {
                        continue;
                    };
                    let neighbor_id = if edge.source == gn.id {
                        edge.target
                    } else {
                        edge.source
                    };
                    let Some(neighbor) = graph.nodes.get(&neighbor_id) else {
                        continue;
                    };
                    let neighbor_chunk = IVec2::new(
                        (neighbor.position.x / CHUNK_SIZE).floor() as i32,
                        (neighbor.position.y / CHUNK_SIZE).floor() as i32,
                    );

                    let was_same_chunk = old == IVec2::new(
                        (neighbor.position.x / CHUNK_SIZE).floor() as i32,
                        (neighbor.position.y / CHUNK_SIZE).floor() as i32,
                    );
                    let now_same_chunk = new_chunk == neighbor_chunk;

                    match (was_same_chunk, now_same_chunk) {
                        (true, false) => {
                            // Was intra-chunk, now cross-chunk
                            grid.remove_edge(*eid, old);
                            grid.insert_cross_edge(*eid, new_chunk);
                            grid.insert_cross_edge(*eid, neighbor_chunk);
                        }
                        (false, true) => {
                            // Was cross-chunk, now intra-chunk
                            grid.remove_cross_edge(*eid, old);
                            grid.remove_cross_edge(*eid, neighbor_chunk);
                            grid.insert_edge(*eid, new_chunk);
                        }
                        (false, false) => {
                            // Still cross-chunk, but this endpoint moved chunks
                            grid.remove_cross_edge(*eid, old);
                            grid.insert_cross_edge(*eid, new_chunk);
                        }
                        (true, true) => {
                            // Stayed in same chunk (shouldn't happen since we
                            // know this node changed chunks, but just in case)
                        }
                    }

                    // Mark both chunks dirty so meshes rebuild
                    if let Some(c) = grid.cells.get_mut(&old) {
                        c.dirty = true;
                    }
                    if let Some(c) = grid.cells.get_mut(&new_chunk) {
                        c.dirty = true;
                    }
                    if let Some(c) = grid.cells.get_mut(&neighbor_chunk) {
                        c.dirty = true;
                    }
                }
            }

            chunk.x = new_cx;
            chunk.y = new_cy;
        }
    }
}

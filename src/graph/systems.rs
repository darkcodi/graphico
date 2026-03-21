use std::collections::HashSet;

use bevy::prelude::*;

use super::components::{ChunkCoord, GraphNode};
use super::events::{AddEdgeEvent, AddNodeEvent, DeleteNodeEvent, UpdateNodeEvent};
use super::model::{EdgeData, GraphData, NodeData, EdgeId, NodeId};
use crate::render::nodes::{NodeRectTexture, estimate_text_size};
use crate::spatial::grid::{SpatialGrid, CHUNK_SIZE};

pub fn process_add_node_events(
    mut commands: Commands,
    mut events: MessageReader<AddNodeEvent>,
    mut graph: ResMut<GraphData>,
    mut grid: ResMut<SpatialGrid>,
    rect_tex: Res<NodeRectTexture>,
) {
    for event in events.read() {
        let id = event
            .pre_allocated_id
            .unwrap_or_else(|| graph.next_node_id());
        let position = event.position;
        let size = estimate_text_size(&event.name);

        let chunk_x = (position.x / CHUNK_SIZE).floor() as i32;
        let chunk_y = (position.y / CHUNK_SIZE).floor() as i32;

        let entity = commands
            .spawn((
                GraphNode { id },
                ChunkCoord {
                    x: chunk_x,
                    y: chunk_y,
                },
                Sprite {
                    image: rect_tex.0.clone(),
                    color: event.color,
                    custom_size: Some(size),
                    ..default()
                },
                Transform::from_translation(position.extend(1.0)),
            ))
            .id();

        graph.nodes.insert(
            id,
            NodeData {
                position,
                name: event.name.clone(),
                data: event.data.clone(),
                color: event.color,
                size,
                entity: Some(entity),
            },
        );
        graph.adjacency.entry(id).or_default();

        grid.insert_node(id, IVec2::new(chunk_x, chunk_y));
    }
}

pub fn process_add_edge_events(
    mut events: MessageReader<AddEdgeEvent>,
    mut graph: ResMut<GraphData>,
    mut grid: ResMut<SpatialGrid>,
) {
    for event in events.read() {
        let source_pos = graph.nodes.get(&event.source).map(|n| n.position);
        let target_pos = graph.nodes.get(&event.target).map(|n| n.position);

        let (source_pos, target_pos) = match (source_pos, target_pos) {
            (Some(s), Some(t)) => (s, t),
            _ => continue,
        };

        let id = graph.next_edge_id();

        graph.edges.insert(
            id,
            EdgeData {
                source: event.source,
                target: event.target,
                color: event.color,
            },
        );
        graph
            .adjacency
            .entry(event.source)
            .or_default()
            .push(id);
        graph
            .adjacency
            .entry(event.target)
            .or_default()
            .push(id);

        let src_chunk = IVec2::new(
            (source_pos.x / CHUNK_SIZE).floor() as i32,
            (source_pos.y / CHUNK_SIZE).floor() as i32,
        );
        let tgt_chunk = IVec2::new(
            (target_pos.x / CHUNK_SIZE).floor() as i32,
            (target_pos.y / CHUNK_SIZE).floor() as i32,
        );

        if src_chunk == tgt_chunk {
            grid.insert_edge(id, src_chunk);
        } else {
            grid.insert_cross_edge(id, src_chunk);
            grid.insert_cross_edge(id, tgt_chunk);
        }
    }
}

fn remove_single_edge(graph: &mut GraphData, grid: &mut SpatialGrid, edge_id: EdgeId) {
    let Some(edge) = graph.edges.remove(&edge_id) else {
        return;
    };

    let source = edge.source;
    let target = edge.target;

    if let Some(adj) = graph.adjacency.get_mut(&source) {
        adj.retain(|eid| *eid != edge_id);
    }
    if let Some(adj) = graph.adjacency.get_mut(&target) {
        adj.retain(|eid| *eid != edge_id);
    }

    let src_pos = graph.nodes.get(&source).map(|n| n.position);
    let tgt_pos = graph.nodes.get(&target).map(|n| n.position);
    let (src_pos, tgt_pos) = match (src_pos, tgt_pos) {
        (Some(s), Some(t)) => (s, t),
        _ => return,
    };

    let src_chunk = IVec2::new(
        (src_pos.x / CHUNK_SIZE).floor() as i32,
        (src_pos.y / CHUNK_SIZE).floor() as i32,
    );
    let tgt_chunk = IVec2::new(
        (tgt_pos.x / CHUNK_SIZE).floor() as i32,
        (tgt_pos.y / CHUNK_SIZE).floor() as i32,
    );

    if src_chunk == tgt_chunk {
        grid.remove_edge(edge_id, src_chunk);
    } else {
        grid.remove_cross_edge(edge_id, src_chunk);
        grid.remove_cross_edge(edge_id, tgt_chunk);
    }
}

pub fn process_update_node_events(
    mut events: MessageReader<UpdateNodeEvent>,
    mut graph: ResMut<GraphData>,
    mut grid: ResMut<SpatialGrid>,
    mut edge_events: MessageWriter<AddEdgeEvent>,
    mut node_query: Query<(&GraphNode, &mut ChunkCoord, &mut Transform, &mut Sprite)>,
) {
    for event in events.read() {
        let node_id = event.node_id;
        if !graph.nodes.contains_key(&node_id) {
            continue;
        }

        let desired: HashSet<NodeId> = event.desired_neighbor_ids.iter().copied().collect();
        let incident: Vec<EdgeId> = graph.adjacency.get(&node_id).cloned().unwrap_or_default();
        let mut to_remove = Vec::new();
        for &eid in &incident {
            if let Some(edge) = graph.edges.get(&eid) {
                let other = if edge.source == node_id {
                    edge.target
                } else {
                    edge.source
                };
                if !desired.contains(&other) {
                    to_remove.push(eid);
                }
            }
        }
        for eid in to_remove {
            remove_single_edge(&mut graph, &mut grid, eid);
        }

        let Some(node_data) = graph.nodes.get_mut(&node_id) else {
            continue;
        };

        let old_pos = node_data.position;
        let old_chunk = IVec2::new(
            (old_pos.x / CHUNK_SIZE).floor() as i32,
            (old_pos.y / CHUNK_SIZE).floor() as i32,
        );

        let position = event.position;
        let size = estimate_text_size(&event.name);

        node_data.position = position;
        node_data.name = event.name.clone();
        node_data.data = event.data.clone();
        node_data.color = event.color;
        node_data.size = size;

        let new_chunk = IVec2::new(
            (position.x / CHUNK_SIZE).floor() as i32,
            (position.y / CHUNK_SIZE).floor() as i32,
        );

        if old_chunk != new_chunk {
            grid.remove_node(node_id, old_chunk);
            grid.insert_node(node_id, new_chunk);
        }

        if let Some(ent) = node_data.entity
            && let Ok((gn, mut chunk_coord, mut transform, mut sprite)) = node_query.get_mut(ent)
            && gn.id == node_id
        {
            chunk_coord.x = new_chunk.x;
            chunk_coord.y = new_chunk.y;
            transform.translation.x = position.x;
            transform.translation.y = position.y;
            sprite.color = event.color;
            sprite.custom_size = Some(size);
        }

        let neighbors_now: HashSet<NodeId> = graph
            .adjacency
            .get(&node_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|eid| {
                        graph.edges.get(eid).map(|edge| {
                            if edge.source == node_id {
                                edge.target
                            } else {
                                edge.source
                            }
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        for &target_id in &event.desired_neighbor_ids {
            if !neighbors_now.contains(&target_id) {
                edge_events.write(AddEdgeEvent {
                    source: node_id,
                    target: target_id,
                    color: Color::srgba(0.5, 0.5, 0.5, 0.4),
                });
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct LayoutState {
    pub pending_nodes: Vec<NodeId>,
}

pub fn incremental_layout(
    mut graph: ResMut<GraphData>,
    mut layout: ResMut<LayoutState>,
    mut node_query: Query<(&GraphNode, &mut Transform)>,
) {
    if layout.pending_nodes.is_empty() {
        return;
    }

    let iterations = layout.pending_nodes.len().min(100);
    let nodes_to_process: Vec<NodeId> = layout.pending_nodes.drain(..iterations).collect();

    for node_id in &nodes_to_process {
        let Some(node_data) = graph.nodes.get(node_id) else {
            continue;
        };
        let pos = node_data.position;

        let mut force = Vec2::ZERO;
        let mut neighbor_count = 0;

        for (other_id, other_data) in graph.nodes.iter() {
            if other_id == node_id {
                continue;
            }
            let diff = pos - other_data.position;
            let dist_sq = diff.length_squared();
            if dist_sq < 10000.0 && dist_sq > 0.01 {
                force += diff.normalize() * (500.0 / dist_sq.max(1.0));
                neighbor_count += 1;
            }
            if neighbor_count > 20 {
                break;
            }
        }

        if let Some(edge_ids) = graph.adjacency.get(node_id) {
            for edge_id in edge_ids.iter() {
                if let Some(edge) = graph.edges.get(edge_id) {
                    let other_id = if edge.source == *node_id {
                        edge.target
                    } else {
                        edge.source
                    };
                    if let Some(other) = graph.nodes.get(&other_id) {
                        let diff = other.position - pos;
                        let dist = diff.length();
                        if dist > 50.0 {
                            force += diff.normalize() * (dist - 50.0) * 0.01;
                        }
                    }
                }
            }
        }

        let displacement = force * 0.5;
        if displacement.length() > 0.1 {
            let new_pos = pos + displacement;
            if let Some(node_data) = graph.nodes.get_mut(node_id) {
                node_data.position = new_pos;
            }
        }
    }

    for (graph_node, mut transform) in node_query.iter_mut() {
        if let Some(node_data) = graph.nodes.get(&graph_node.id) {
            transform.translation.x = node_data.position.x;
            transform.translation.y = node_data.position.y;
        }
    }
}

pub fn process_delete_node_events(
    mut commands: Commands,
    mut events: MessageReader<DeleteNodeEvent>,
    mut graph: ResMut<GraphData>,
    mut grid: ResMut<SpatialGrid>,
) {
    for event in events.read() {
        let node_id = event.id;

        let Some(node_data) = graph.nodes.remove(&node_id) else {
            continue;
        };

        let chunk = IVec2::new(
            (node_data.position.x / CHUNK_SIZE).floor() as i32,
            (node_data.position.y / CHUNK_SIZE).floor() as i32,
        );
        grid.remove_node(node_id, chunk);

        if let Some(entity) = node_data.entity {
            commands.entity(entity).despawn();
        }

        if let Some(edge_ids) = graph.adjacency.remove(&node_id) {
            for edge_id in edge_ids {
                if let Some(edge) = graph.edges.remove(&edge_id) {
                    let other_id = if edge.source == node_id {
                        edge.target
                    } else {
                        edge.source
                    };
                    if let Some(adj) = graph.adjacency.get_mut(&other_id) {
                        adj.retain(|eid| *eid != edge_id);
                    }

                    let src_pos = if edge.source == node_id {
                        node_data.position
                    } else {
                        graph
                            .nodes
                            .get(&edge.source)
                            .map(|n| n.position)
                            .unwrap_or(node_data.position)
                    };
                    let tgt_pos = if edge.target == node_id {
                        node_data.position
                    } else {
                        graph
                            .nodes
                            .get(&edge.target)
                            .map(|n| n.position)
                            .unwrap_or(node_data.position)
                    };

                    let src_chunk = IVec2::new(
                        (src_pos.x / CHUNK_SIZE).floor() as i32,
                        (src_pos.y / CHUNK_SIZE).floor() as i32,
                    );
                    let tgt_chunk = IVec2::new(
                        (tgt_pos.x / CHUNK_SIZE).floor() as i32,
                        (tgt_pos.y / CHUNK_SIZE).floor() as i32,
                    );

                    if src_chunk == tgt_chunk {
                        grid.remove_edge(edge_id, src_chunk);
                    } else {
                        grid.remove_cross_edge(edge_id, src_chunk);
                        grid.remove_cross_edge(edge_id, tgt_chunk);
                    }
                }
            }
        }
    }
}

use bevy::prelude::*;

use super::components::{ChunkCoord, GraphNode};
use super::events::{AddEdgeEvent, AddNodeEvent, DeleteNodeEvent};
use super::model::{EdgeData, GraphData, NodeData};
use crate::render::nodes::NodeCircleTexture;
use crate::spatial::grid::{SpatialGrid, CHUNK_SIZE};

pub fn process_add_node_events(
    mut commands: Commands,
    mut events: MessageReader<AddNodeEvent>,
    mut graph: ResMut<GraphData>,
    mut grid: ResMut<SpatialGrid>,
    circle_tex: Res<NodeCircleTexture>,
) {
    for event in events.read() {
        let id = event
            .pre_allocated_id
            .unwrap_or_else(|| graph.next_node_id());
        let position = event.position;

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
                    image: circle_tex.0.clone(),
                    color: event.color,
                    custom_size: Some(Vec2::splat(event.radius * 2.0)),
                    ..default()
                },
                Transform::from_translation(position.extend(1.0)),
            ))
            .id();

        graph.nodes.insert(
            id,
            NodeData {
                position,
                label: event.label.clone(),
                color: event.color,
                radius: event.radius,
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

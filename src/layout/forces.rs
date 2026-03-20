use bevy::prelude::*;

use crate::graph::components::{ChunkCoord, GraphNode};
use crate::graph::model::GraphData;
use crate::spatial::grid::{SpatialGrid, CHUNK_SIZE};

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

                    let was_same_chunk = old == neighbor_chunk;
                    let now_same_chunk = new_chunk == neighbor_chunk;

                    match (was_same_chunk, now_same_chunk) {
                        (true, false) => {
                            grid.remove_edge(*eid, old);
                            grid.insert_cross_edge(*eid, new_chunk);
                            grid.insert_cross_edge(*eid, neighbor_chunk);
                        }
                        (false, true) => {
                            grid.remove_cross_edge(*eid, old);
                            grid.remove_cross_edge(*eid, neighbor_chunk);
                            grid.insert_edge(*eid, new_chunk);
                        }
                        (false, false) => {
                            grid.remove_cross_edge(*eid, old);
                            grid.insert_cross_edge(*eid, new_chunk);
                        }
                        (true, true) => {}
                    }

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

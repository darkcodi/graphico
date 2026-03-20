use bevy::prelude::*;
use std::collections::HashMap;

use crate::graph::model::{EdgeId, NodeId};

pub const CHUNK_SIZE: f32 = 1000.0;

pub struct ChunkData {
    pub node_ids: Vec<NodeId>,
    pub edge_ids: Vec<EdgeId>,
    pub cross_edges: Vec<EdgeId>,
    pub entity: Option<Entity>,
    pub dirty: bool,
}

impl Default for ChunkData {
    fn default() -> Self {
        Self {
            node_ids: Vec::new(),
            edge_ids: Vec::new(),
            cross_edges: Vec::new(),
            entity: None,
            dirty: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct SpatialGrid {
    pub cells: HashMap<IVec2, ChunkData>,
}

impl SpatialGrid {
    pub fn insert_node(&mut self, id: NodeId, chunk: IVec2) {
        let cell = self.cells.entry(chunk).or_default();
        cell.node_ids.push(id);
        cell.dirty = true;
    }

    pub fn insert_edge(&mut self, id: EdgeId, chunk: IVec2) {
        let cell = self.cells.entry(chunk).or_default();
        cell.edge_ids.push(id);
        cell.dirty = true;
    }

    pub fn insert_cross_edge(&mut self, id: EdgeId, chunk: IVec2) {
        let cell = self.cells.entry(chunk).or_default();
        cell.cross_edges.push(id);
        cell.dirty = true;
    }

    pub fn move_node(&mut self, id: NodeId, old_chunk: IVec2, new_chunk: IVec2) {
        self.remove_node(id, old_chunk);
        self.insert_node(id, new_chunk);
    }

    pub fn remove_node(&mut self, id: NodeId, chunk: IVec2) {
        if let Some(cell) = self.cells.get_mut(&chunk) {
            cell.node_ids.retain(|n| *n != id);
            cell.dirty = true;
        }
    }

    pub fn remove_edge(&mut self, id: EdgeId, chunk: IVec2) {
        if let Some(cell) = self.cells.get_mut(&chunk) {
            cell.edge_ids.retain(|e| *e != id);
            cell.dirty = true;
        }
    }

    pub fn remove_cross_edge(&mut self, id: EdgeId, chunk: IVec2) {
        if let Some(cell) = self.cells.get_mut(&chunk) {
            cell.cross_edges.retain(|e| *e != id);
            cell.dirty = true;
        }
    }

    /// Get the world-space AABB for a chunk coordinate.
    pub fn chunk_aabb(coord: IVec2) -> (Vec2, Vec2) {
        let min = Vec2::new(coord.x as f32 * CHUNK_SIZE, coord.y as f32 * CHUNK_SIZE);
        let max = min + Vec2::splat(CHUNK_SIZE);
        (min, max)
    }
}

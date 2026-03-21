use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(pub u64);

pub struct NodeData {
    pub position: Vec2,
    pub data: String,
    pub color: Color,
    pub size: Vec2,
    pub entity: Option<Entity>,
}

pub struct EdgeData {
    pub source: NodeId,
    pub target: NodeId,
    pub color: Color,
}

#[derive(Resource, Default)]
pub struct GraphData {
    pub nodes: HashMap<NodeId, NodeData>,
    pub edges: HashMap<EdgeId, EdgeData>,
    pub adjacency: HashMap<NodeId, Vec<EdgeId>>,
    next_node_id: u64,
    next_edge_id: u64,
}

impl GraphData {
    pub fn next_node_id(&mut self) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    pub fn next_edge_id(&mut self) -> EdgeId {
        let id = EdgeId(self.next_edge_id);
        self.next_edge_id += 1;
        id
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

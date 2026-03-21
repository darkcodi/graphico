use bevy::prelude::*;

use super::model::NodeId;

#[derive(Message)]
pub struct AddNodeEvent {
    pub position: Vec2,
    pub name: String,
    pub data: String,
    pub color: Color,
    /// Pre-allocated NodeId from the API layer. When `Some`, the event
    /// processor uses this instead of calling `next_node_id()`.
    pub pre_allocated_id: Option<NodeId>,
}

#[derive(Message)]
pub struct AddEdgeEvent {
    pub source: NodeId,
    pub target: NodeId,
    pub color: Color,
}

#[derive(Message)]
pub struct DeleteNodeEvent {
    pub id: NodeId,
}

#[derive(Message)]
pub struct UpdateNodeEvent {
    pub node_id: NodeId,
    pub name: String,
    pub data: String,
    pub color: Color,
    pub position: Vec2,
    pub desired_neighbor_ids: Vec<NodeId>,
}

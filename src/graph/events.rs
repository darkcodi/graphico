use bevy::prelude::*;

use super::model::NodeId;

#[derive(Message)]
pub struct AddNodeEvent {
    pub position: Vec2,
    pub label: String,
    pub color: Color,
    pub radius: f32,
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

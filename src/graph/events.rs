use bevy::prelude::*;

use super::model::NodeId;

#[derive(Message)]
pub struct AddNodeEvent {
    pub position: Vec2,
    pub label: String,
    pub color: Color,
    pub radius: f32,
}

#[derive(Message)]
pub struct AddEdgeEvent {
    pub source: NodeId,
    pub target: NodeId,
    pub color: Color,
}

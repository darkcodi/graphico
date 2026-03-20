use bevy::prelude::*;

use super::model::NodeId;

#[derive(Component)]
pub struct GraphNode {
    pub id: NodeId,
}

#[derive(Component)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct ChunkEntity;

#[derive(Component)]
pub struct NodeLabel;

#[derive(Component)]
pub struct Selected;

#[derive(Component)]
pub struct Hovered;

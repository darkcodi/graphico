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

/// World-space label size (`base_font_size`, `base_y`) for zoom-aware glyph rasterization.
#[derive(Component)]
pub struct NodeLabelZoom {
    pub base_font_size: f32,
    pub base_y: f32,
}

#[derive(Component)]
pub struct Selected;

#[derive(Component)]
pub struct Hovered;

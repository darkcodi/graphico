pub mod grid;
pub mod systems;

use bevy::prelude::*;

use crate::GraphSystems;
use grid::SpatialGrid;

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrid>().add_systems(
            Update,
            systems::rebuild_dirty_chunks.in_set(GraphSystems::ChunkRebuild),
        );
    }
}

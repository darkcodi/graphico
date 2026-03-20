pub mod engine;
pub mod forces;
pub mod params;
pub mod placement;
pub mod quadtree;

use bevy::prelude::*;

use crate::GraphSystems;

pub use engine::LayoutEngine;
pub use params::LayoutParams;

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LayoutParams>()
            .init_resource::<LayoutEngine>()
            .add_systems(
                Update,
                (
                    placement::place_new_nodes,
                    forces::sync_layout_positions,
                )
                    .chain()
                    .in_set(GraphSystems::Layout),
            );
    }
}

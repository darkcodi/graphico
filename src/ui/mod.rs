pub mod minimap;
pub mod selection;

use bevy::prelude::*;

use crate::GraphSystems;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, minimap::setup_minimap)
            .add_systems(
                Update,
                (
                    (selection::handle_selection, selection::handle_hover)
                        .in_set(GraphSystems::Selection),
                    selection::apply_highlights.in_set(GraphSystems::VisibilityUpdate),
                    minimap::update_minimap_camera.in_set(GraphSystems::CameraSmooth),
                ),
            );
    }
}

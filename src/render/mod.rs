pub mod edges;
pub mod labels;
pub mod lod;
pub mod nodes;

use bevy::prelude::*;

use crate::GraphSystems;
use labels::ActiveLabels;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveLabels>()
            .add_systems(Startup, nodes::create_rect_texture)
            .add_systems(
                Update,
                (
                    (labels::manage_labels, labels::sync_label_zoom)
                        .chain()
                        .in_set(GraphSystems::VisibilityUpdate),
                    lod::lod_visibility.in_set(GraphSystems::VisibilityUpdate),
                ),
            );
    }
}

pub mod fix_overlaps;
pub mod inspector;
pub mod minimap;
pub mod selection;

use bevy::prelude::*;

use crate::GraphSystems;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                minimap::setup_minimap,
                inspector::setup_inspector,
                fix_overlaps::setup_fix_overlaps,
            ),
        )
            .add_systems(
                Update,
                (
                    fix_overlaps::handle_fix_overlaps_click.before(GraphSystems::EventProcessing),
                    (
                        selection::handle_selection,
                        selection::handle_hover,
                        inspector::handle_inspector_node_link_click.after(selection::handle_selection),
                    )
                        .in_set(GraphSystems::Selection),
                    selection::apply_highlights.in_set(GraphSystems::VisibilityUpdate),
                    inspector::update_inspector.after(GraphSystems::Selection),
                    fix_overlaps::update_fix_overlaps_visibility.after(GraphSystems::Selection),
                    minimap::update_minimap_camera.in_set(GraphSystems::CameraSmooth),
                ),
            );
    }
}

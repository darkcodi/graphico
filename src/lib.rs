pub mod api;
pub mod camera;
#[cfg(feature = "demo")]
pub mod demo;
pub mod graph;
pub mod persist;
pub mod render;
pub mod spatial;
pub mod ui;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GraphSystems {
    EventProcessing,
    SpatialUpdate,
    Layout,
    CameraInput,
    CameraSmooth,
    Selection,
    VisibilityUpdate,
    ChunkRebuild,
}

pub struct GraphicoPlugin;

impl Plugin for GraphicoPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                GraphSystems::EventProcessing,
                GraphSystems::SpatialUpdate,
                GraphSystems::Layout,
                GraphSystems::CameraInput,
                GraphSystems::CameraSmooth,
                GraphSystems::Selection,
                GraphSystems::VisibilityUpdate,
                GraphSystems::ChunkRebuild,
            )
                .chain(),
        );

        app.add_plugins((
            graph::GraphPlugin,
            spatial::SpatialPlugin,
            camera::CameraPlugin,
            render::RenderPlugin,
            ui::UiPlugin,
            persist::PersistPlugin,
            api::ApiPlugin,
        ));

        #[cfg(feature = "demo")]
        app.add_plugins(demo::DemoPlugin);
    }
}

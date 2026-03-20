pub mod components;
pub mod systems;

use bevy::prelude::*;

use crate::GraphSystems;
use components::CameraState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera).add_systems(
            Update,
            (
                (systems::camera_input, systems::camera_drag_pan).in_set(GraphSystems::CameraInput),
                systems::camera_smooth.in_set(GraphSystems::CameraSmooth),
            ),
        );
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        CameraState::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

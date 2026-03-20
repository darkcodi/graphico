use bevy::prelude::*;

#[derive(Component)]
pub struct CameraState {
    pub target_position: Vec2,
    pub target_zoom: f32,
    pub smoothing: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            target_position: Vec2::ZERO,
            target_zoom: 1.0,
            smoothing: 8.0,
        }
    }
}

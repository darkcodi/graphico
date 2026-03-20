use bevy::input::mouse::{AccumulatedMouseScroll, MouseButton};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::components::CameraState;

fn ortho_scale(projection: &Projection) -> f32 {
    match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    }
}

pub fn camera_input(
    mut camera_q: Query<(&mut CameraState, &Projection, &GlobalTransform)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
) {
    let Ok((mut state, projection, global_tf)) = camera_q.single_mut() else {
        return;
    };
    let Ok(window) = window_q.single() else {
        return;
    };

    let scale = ortho_scale(projection);

    // Pan with WASD / arrows
    let mut pan = Vec2::ZERO;
    let speed = 500.0 * scale * time.delta_secs();

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        pan.y += speed;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        pan.y -= speed;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        pan.x -= speed;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        pan.x += speed;
    }
    state.target_position += pan;

    // Zoom with scroll wheel (cursor-centered)
    let scroll_y = scroll.delta.y;
    if scroll_y.abs() > 0.0 {
        let zoom_factor = 1.0 - scroll_y * 0.1;
        let new_zoom = (state.target_zoom * zoom_factor).clamp(0.01, 100.0);

        if let Some(cursor_pos) = window.cursor_position() {
            let window_size = Vec2::new(window.width(), window.height());
            let ndc = (cursor_pos / window_size) * 2.0 - Vec2::ONE;
            let ndc = Vec2::new(ndc.x, -ndc.y);

            let world_cursor =
                global_tf.translation().truncate() + ndc * window_size * 0.5 * scale;

            let zoom_ratio = new_zoom / state.target_zoom;
            state.target_position =
                world_cursor + (state.target_position - world_cursor) * zoom_ratio;
        }

        state.target_zoom = new_zoom;
    }
}

/// Middle-mouse drag panning using cursor delta.
pub fn camera_drag_pan(
    mut camera_q: Query<(&mut CameraState, &Projection)>,
    mouse: Res<ButtonInput<MouseButton>>,
    window_q: Query<&Window, With<PrimaryWindow>>,
    mut last_cursor: Local<Option<Vec2>>,
) {
    let Ok((mut state, projection)) = camera_q.single_mut() else {
        return;
    };
    let Ok(window) = window_q.single() else {
        return;
    };

    let scale = ortho_scale(projection);

    if mouse.pressed(MouseButton::Middle) {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Some(last) = *last_cursor {
                let delta = cursor_pos - last;
                state.target_position.x -= delta.x * scale;
                state.target_position.y += delta.y * scale;
            }
            *last_cursor = Some(cursor_pos);
        }
    } else {
        *last_cursor = None;
    }
}

pub fn camera_smooth(
    mut camera_q: Query<(&CameraState, &mut Transform, &mut Projection)>,
    time: Res<Time>,
) {
    let Ok((state, mut transform, mut projection)) = camera_q.single_mut() else {
        return;
    };

    let t = (state.smoothing * time.delta_secs()).min(1.0);

    transform.translation.x += (state.target_position.x - transform.translation.x) * t;
    transform.translation.y += (state.target_position.y - transform.translation.y) * t;

    if let Projection::Orthographic(ref mut ortho) = *projection {
        ortho.scale += (state.target_zoom - ortho.scale) * t;
    }
}

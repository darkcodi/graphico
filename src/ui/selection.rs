use bevy::prelude::*;

use crate::camera::components::CameraState;
use crate::graph::components::{GraphNode, Hovered, Selected};

/// Toggle selection on click.
pub fn handle_selection(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<CameraState>>,
    nodes_q: Query<(Entity, &GraphNode, &Transform), Without<Camera2d>>,
    selected_q: Query<Entity, With<Selected>>,
    graph: Res<crate::graph::model::GraphData>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = window_q.single() else {
        return;
    };
    let Ok((camera, cam_tf)) = camera_q.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(cam_tf, cursor_pos) else {
        return;
    };

    // Find closest node under cursor
    let mut closest: Option<(Entity, f32)> = None;
    for (entity, graph_node, transform) in nodes_q.iter() {
        let node_pos = transform.translation.truncate();
        let dist = (world_pos - node_pos).length();

        if let Some(node_data) = graph.nodes.get(&graph_node.id) {
            let hit_radius = node_data.radius * 1.5; // generous hit area
            if dist < hit_radius {
                if closest.is_none() || dist < closest.unwrap().1 {
                    closest = Some((entity, dist));
                }
            }
        }
    }

    // Clear previous selection
    for entity in selected_q.iter() {
        commands.entity(entity).remove::<Selected>();
    }

    // Select clicked node
    if let Some((entity, _)) = closest {
        commands.entity(entity).insert(Selected);
    }
}

/// Update hover state based on cursor proximity.
pub fn handle_hover(
    mut commands: Commands,
    window_q: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<CameraState>>,
    nodes_q: Query<(Entity, &GraphNode, &Transform), Without<Camera2d>>,
    hovered_q: Query<Entity, With<Hovered>>,
    graph: Res<crate::graph::model::GraphData>,
) {
    let Ok(window) = window_q.single() else {
        return;
    };
    let Ok((camera, cam_tf)) = camera_q.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        // Clear all hovers if cursor is outside window
        for entity in hovered_q.iter() {
            commands.entity(entity).remove::<Hovered>();
        }
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(cam_tf, cursor_pos) else {
        return;
    };

    // Clear previous hovers
    for entity in hovered_q.iter() {
        commands.entity(entity).remove::<Hovered>();
    }

    // Find closest node under cursor
    for (entity, graph_node, transform) in nodes_q.iter() {
        let node_pos = transform.translation.truncate();
        let dist = (world_pos - node_pos).length();

        if let Some(node_data) = graph.nodes.get(&graph_node.id) {
            let hit_radius = node_data.radius * 1.5;
            if dist < hit_radius {
                commands.entity(entity).insert(Hovered);
                break;
            }
        }
    }
}

/// Apply highlight colors each frame for selected/hovered nodes.
pub fn apply_highlights(
    mut nodes_q: Query<(&GraphNode, &mut Sprite, Option<&Selected>, Option<&Hovered>)>,
    graph: Res<crate::graph::model::GraphData>,
) {
    for (graph_node, mut sprite, selected, hovered) in nodes_q.iter_mut() {
        if let Some(node_data) = graph.nodes.get(&graph_node.id) {
            if selected.is_some() {
                sprite.color = Color::srgb(1.0, 1.0, 0.3); // Yellow highlight
            } else if hovered.is_some() {
                sprite.color = Color::srgb(1.0, 0.8, 0.5); // Warm highlight
            } else {
                sprite.color = node_data.color;
            }
        }
    }
}

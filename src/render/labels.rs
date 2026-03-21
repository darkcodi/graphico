use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::TextBounds;
use std::collections::HashSet;

use crate::camera::components::CameraState;
use crate::graph::components::{GraphNode, Hovered, NodeLabel, NodeLabelZoom, Selected};
use crate::graph::model::GraphData;

/// Tracks which nodes currently have label entities spawned.
#[derive(Resource, Default)]
pub struct ActiveLabels {
    pub labeled_nodes: HashSet<Entity>,
}

/// Zoom threshold below which labels become visible.
const LABEL_ZOOM_THRESHOLD: f32 = 2.0;
/// Max labels to spawn per frame to avoid spikes.
const MAX_LABEL_SPAWNS_PER_FRAME: usize = 50;
const FONT_SIZE: f32 = 12.0;
/// Match camera zoom clamp ([`camera_input`](crate::camera::systems::camera_input)).
const ORTHO_SCALE_MIN: f32 = 0.01;
const ORTHO_SCALE_MAX: f32 = 100.0;
/// Glyph rasterization size limits.
const RASTER_FONT_MIN: f32 = 4.0;
const RASTER_FONT_MAX: f32 = 256.0;

fn ortho_scale(projection: &Projection) -> f32 {
    match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    }
}

/// Keeps glyph raster resolution in sync with orthographic zoom so labels stay sharp when magnified.
pub fn sync_label_zoom(
    camera_q: Query<&Projection, With<CameraState>>,
    mut q: Query<(&NodeLabelZoom, &mut TextFont, &mut Transform), With<NodeLabel>>,
) {
    let Ok(projection) = camera_q.single() else {
        return;
    };
    let ortho = ortho_scale(projection).clamp(ORTHO_SCALE_MIN, ORTHO_SCALE_MAX);
    for (zoom, mut font, mut tf) in q.iter_mut() {
        let target_raster = (zoom.base_font_size / ortho).clamp(RASTER_FONT_MIN, RASTER_FONT_MAX);
        let display_scale = zoom.base_font_size / target_raster;
        font.font_size = target_raster;
        tf.translation = Vec3::new(0.0, 0.0, 2.0);
        tf.scale = Vec3::splat(display_scale);
    }
}

pub fn manage_labels(
    mut commands: Commands,
    camera_q: Query<&Projection, With<CameraState>>,
    nodes_q: Query<(
        Entity,
        &GraphNode,
        &ViewVisibility,
        Option<&Selected>,
        Option<&Hovered>,
    )>,
    label_q: Query<(Entity, &ChildOf), With<NodeLabel>>,
    graph: Res<GraphData>,
    mut active: ResMut<ActiveLabels>,
) {
    let Ok(projection) = camera_q.single() else {
        return;
    };

    let scale = ortho_scale(projection);
    let zoomed_in = scale < LABEL_ZOOM_THRESHOLD;
    let mut spawned = 0;

    for (entity, graph_node, visibility, selected, hovered) in nodes_q.iter() {
        let should_show =
            (zoomed_in && visibility.get()) || selected.is_some() || hovered.is_some();

        if should_show && !active.labeled_nodes.contains(&entity) {
            if spawned >= MAX_LABEL_SPAWNS_PER_FRAME {
                break;
            }

            if let Some(node_data) = graph.nodes.get(&graph_node.id) {
                if !node_data.data.is_empty() {
                    let base_font_size = FONT_SIZE;
                    let ortho = scale.clamp(ORTHO_SCALE_MIN, ORTHO_SCALE_MAX);
                    let target_raster =
                        (base_font_size / ortho).clamp(RASTER_FONT_MIN, RASTER_FONT_MAX);
                    let display_scale = base_font_size / target_raster;
                    let label_entity = commands
                        .spawn((
                            NodeLabel,
                            NodeLabelZoom {
                                base_font_size,
                                base_y: 0.0,
                            },
                            Text2d::new(&node_data.data),
                            TextBounds::from(node_data.size),
                            TextFont {
                                font_size: target_raster,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TextLayout::new_with_justify(Justify::Center),
                            Anchor::CENTER,
                            Transform::from_translation(Vec3::new(0.0, 0.0, 2.0))
                                .with_scale(Vec3::splat(display_scale)),
                        ))
                        .id();
                    commands.entity(entity).add_child(label_entity);
                    active.labeled_nodes.insert(entity);
                    spawned += 1;
                }
            }
        } else if !should_show && active.labeled_nodes.contains(&entity) {
            active.labeled_nodes.remove(&entity);
            for (label_entity, child_of) in label_q.iter() {
                if child_of.parent() == entity {
                    commands.entity(label_entity).despawn();
                }
            }
        }
    }
}

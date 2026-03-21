use bevy::prelude::*;
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
/// Label text scales with node radius; tuned so default `radius == 1` reads smaller than a fixed 14pt.
const FONT_PER_RADIUS: f32 = 6.0;
const FONT_MIN: f32 = 4.0;
const FONT_MAX: f32 = 22.0;
/// Vertical gap above the node center, as a fraction of font size.
const LABEL_OFFSET_EM: f32 = 0.4;
/// Match camera zoom clamp ([`camera_input`](crate::camera::systems::camera_input)).
const ORTHO_SCALE_MIN: f32 = 0.01;
const ORTHO_SCALE_MAX: f32 = 100.0;
/// Glyph rasterization size limits (avoids tiny atlases when zoomed out / huge when zoomed in).
const RASTER_FONT_MIN: f32 = 4.0;
const RASTER_FONT_MAX: f32 = 256.0;

fn ortho_scale(projection: &Projection) -> f32 {
    match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    }
}

/// Keeps glyph raster resolution in sync with orthographic zoom so labels stay sharp when magnified.
///
/// `TextFont::font_size` is the rasterization size; `Transform::scale` is chosen so
/// `font_size * scale ≈ base_font_size` in world units.
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
        tf.translation = Vec3::new(0.0, zoom.base_y, 2.0);
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
                if !node_data.label.is_empty() {
                    let base_font_size =
                        (node_data.radius * FONT_PER_RADIUS).clamp(FONT_MIN, FONT_MAX);
                    let label_y = node_data.radius + base_font_size * LABEL_OFFSET_EM;
                    let ortho = scale.clamp(ORTHO_SCALE_MIN, ORTHO_SCALE_MAX);
                    let target_raster =
                        (base_font_size / ortho).clamp(RASTER_FONT_MIN, RASTER_FONT_MAX);
                    let display_scale = base_font_size / target_raster;
                    let label_entity = commands
                        .spawn((
                            NodeLabel,
                            NodeLabelZoom {
                                base_font_size,
                                base_y: label_y,
                            },
                            Text2d::new(&node_data.label),
                            TextFont {
                                font_size: target_raster,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            Transform::from_translation(Vec3::new(0.0, label_y, 2.0))
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

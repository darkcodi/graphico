use bevy::prelude::*;
use std::collections::HashSet;

use crate::camera::components::CameraState;
use crate::graph::components::{GraphNode, Hovered, NodeLabel, Selected};
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

fn ortho_scale(projection: &Projection) -> f32 {
    match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
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
                    let font_size =
                        (node_data.radius * FONT_PER_RADIUS).clamp(FONT_MIN, FONT_MAX);
                    let label_y = node_data.radius + font_size * LABEL_OFFSET_EM;
                    let label_entity = commands
                        .spawn((
                            NodeLabel,
                            Text2d::new(&node_data.label),
                            TextFont {
                                font_size,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            Transform::from_translation(Vec3::new(0.0, label_y, 2.0)),
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

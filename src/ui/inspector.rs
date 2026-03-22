use bevy::prelude::*;

use crate::api::state::{color_to_hex, NodeUuidRegistry};
use crate::camera::components::CameraState;
use crate::graph::components::{GraphNode, Selected};
use crate::graph::model::{GraphData, NodeId};
use crate::graph::overlaps::overlapping_node_ids;

#[derive(Component)]
pub struct InspectorPanel;

#[derive(Component)]
pub struct InspectorText;

#[derive(Component)]
pub struct InspectorEdgeList;

#[derive(Component)]
pub struct InspectorOverlapList;

#[derive(Component)]
pub struct InspectorNodeLink {
    pub node_id: NodeId,
}

pub fn setup_inspector(mut commands: Commands) {
    commands
        .spawn((
            InspectorPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                padding: UiRect::all(Val::Px(12.0)),
                min_width: Val::Px(220.0),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.88)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.15)),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                InspectorText,
                Text::new(""),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.9, 0.9, 0.95)),
            ));
            parent.spawn((
                InspectorEdgeList,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    width: Val::Percent(100.0),
                    ..default()
                },
            ));
            parent.spawn((
                InspectorOverlapList,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    width: Val::Percent(100.0),
                    ..default()
                },
            ));
        });
}

pub fn update_inspector(
    mut commands: Commands,
    selected_q: Query<&GraphNode, With<Selected>>,
    graph: Res<GraphData>,
    registry: Res<NodeUuidRegistry>,
    mut panel_q: Query<&mut Visibility, With<InspectorPanel>>,
    mut text_q: Query<&mut Text, With<InspectorText>>,
    edge_list_q: Query<Entity, With<InspectorEdgeList>>,
    overlap_list_q: Query<Entity, With<InspectorOverlapList>>,
    mut cache: Local<Option<(NodeId, Vec<NodeId>, Vec<NodeId>)>>,
) {
    let Ok(mut vis) = panel_q.single_mut() else {
        return;
    };
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };
    let Ok(edge_list_entity) = edge_list_q.single() else {
        return;
    };
    let Ok(overlap_list_entity) = overlap_list_q.single() else {
        return;
    };

    let Some(graph_node) = selected_q.iter().next() else {
        *vis = Visibility::Hidden;
        *cache = None;
        return;
    };

    let Some(node_data) = graph.nodes.get(&graph_node.id) else {
        *vis = Visibility::Hidden;
        *cache = None;
        return;
    };

    *vis = Visibility::Inherited;

    let neighbor_ids = graph.neighbor_node_ids(graph_node.id);
    let neighbor_count = neighbor_ids.len();

    let uuid_str = registry
        .node_to_uuid
        .get(&graph_node.id)
        .map(|u| u.to_string())
        .unwrap_or_else(|| format!("{}", graph_node.id.0));

    let color_hex = color_to_hex(&node_data.color);

    let overlap_ids = overlapping_node_ids(&graph, graph_node.id);

    let mut content = format!(
        "ID: {}\nName: {}\nColor: {}\nPos: ({:.0}, {:.0})\nSize: {:.0} x {:.0}\nEdges: {}",
        uuid_str,
        node_data.name,
        color_hex,
        node_data.position.x,
        node_data.position.y,
        node_data.size.x,
        node_data.size.y,
        neighbor_count,
    );

    if !node_data.data.is_empty() {
        content.push_str(&format!("\n\nData:\n{}", node_data.data));
    }

    **text = content;

    let needs_rebuild = match cache.as_ref() {
        Some((id, cached_overlap, cached_neighbors)) => {
            *id != graph_node.id
                || cached_overlap != &overlap_ids
                || cached_neighbors != &neighbor_ids
        }
        None => true,
    };

    if needs_rebuild {
        commands.entity(edge_list_entity).despawn_children();
        commands.entity(overlap_list_entity).despawn_children();

        if neighbor_ids.is_empty() {
            commands.entity(edge_list_entity).with_children(|parent| {
                parent.spawn((
                    Text::new("Edges: (none)"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.9, 0.9, 0.9, 0.95)),
                ));
            });
        } else {
            commands.entity(edge_list_entity).with_children(|parent| {
                parent.spawn((
                    Text::new("Edges:"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.9, 0.9, 0.9, 0.95)),
                ));
                for nid in neighbor_ids.iter().copied() {
                    let label = registry
                        .node_to_uuid
                        .get(&nid)
                        .map(|u| u.to_string())
                        .unwrap_or_else(|| format!("{}", nid.0));
                    parent
                        .spawn((
                            Button,
                            InspectorNodeLink { node_id: nid },
                            Node {
                                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                width: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.12, 0.14, 0.22, 0.85)),
                            BorderColor::all(Color::srgba(0.4, 0.55, 0.9, 0.35)),
                        ))
                        .with_child((
                            Text::new(label),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.65, 0.78, 1.0, 0.98)),
                        ));
                }
            });
        }

        if overlap_ids.is_empty() {
            commands.entity(overlap_list_entity).with_children(|parent| {
                parent.spawn((
                    Text::new("Overlaps: (none)"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.9, 0.9, 0.9, 0.95)),
                ));
            });
        } else {
            commands.entity(overlap_list_entity).with_children(|parent| {
                parent.spawn((
                    Text::new("Overlaps:"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.9, 0.9, 0.9, 0.95)),
                ));
                for oid in overlap_ids.iter().copied() {
                    let label = registry
                        .node_to_uuid
                        .get(&oid)
                        .map(|u| u.to_string())
                        .unwrap_or_else(|| format!("{}", oid.0));
                    parent
                        .spawn((
                            Button,
                            InspectorNodeLink { node_id: oid },
                            Node {
                                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                width: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.12, 0.14, 0.22, 0.85)),
                            BorderColor::all(Color::srgba(0.4, 0.55, 0.9, 0.35)),
                        ))
                        .with_child((
                            Text::new(label),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.65, 0.78, 1.0, 0.98)),
                        ));
                }
            });
        }

        *cache = Some((graph_node.id, overlap_ids, neighbor_ids));
    }
}

/// Runs after [`crate::ui::selection::handle_selection`] so inspector link clicks override accidental world picks under the panel.
pub fn handle_inspector_node_link_click(
    mut commands: Commands,
    interaction_q: Query<
        (&Interaction, &InspectorNodeLink),
        (Changed<Interaction>, With<Button>),
    >,
    selected_q: Query<Entity, With<Selected>>,
    graph: Res<GraphData>,
    mut camera_q: Query<&mut CameraState, With<CameraState>>,
) {
    for (interaction, link) in interaction_q.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(node_data) = graph.nodes.get(&link.node_id) else {
            continue;
        };
        let Some(target_entity) = node_data.entity else {
            continue;
        };

        for e in selected_q.iter() {
            commands.entity(e).remove::<Selected>();
        }
        commands.entity(target_entity).insert(Selected);
        if let Ok(mut cam) = camera_q.single_mut() {
            cam.target_position = node_data.position;
        }
    }
}

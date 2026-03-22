use bevy::prelude::*;

use crate::graph::events::UpdateNodeEvent;
use crate::graph::model::GraphData;
use crate::graph::overlaps::{overlapping_node_count, resolve_overlapping_positions};
use crate::persist::GraphPersistenceDirty;

#[derive(Component)]
pub struct FixOverlapsRoot;

#[derive(Component)]
pub struct FixOverlapsButton;

pub fn setup_fix_overlaps(mut commands: Commands) {
    commands
        .spawn((
            FixOverlapsRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(38.0),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    FixOverlapsButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.18, 0.92)),
                    BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                ))
                .with_child((
                    Text::new("Fix overlaps"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.9, 0.9, 0.9, 0.95)),
                ));
        });
}

pub fn update_fix_overlaps_visibility(
    graph: Res<GraphData>,
    mut q: Query<&mut Visibility, With<FixOverlapsRoot>>,
) {
    let Ok(mut vis) = q.single_mut() else {
        return;
    };
    *vis = if overlapping_node_count(&graph) > 0 {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
}

pub fn handle_fix_overlaps_click(
    interaction_q: Query<&Interaction, (Changed<Interaction>, With<FixOverlapsButton>)>,
    graph: Res<GraphData>,
    mut writer: MessageWriter<UpdateNodeEvent>,
    mut persistence_dirty: ResMut<GraphPersistenceDirty>,
) {
    let Ok(interaction) = interaction_q.single() else {
        return;
    };
    if *interaction != Interaction::Pressed {
        return;
    }

    let resolved = resolve_overlapping_positions(&graph);
    if resolved.is_empty() {
        return;
    }

    let mut changed = false;
    for (node_id, new_pos) in resolved {
        let Some(node) = graph.nodes.get(&node_id) else {
            continue;
        };
        let desired_neighbor_ids = graph.neighbor_node_ids(node_id);
        writer.write(UpdateNodeEvent {
            node_id,
            name: node.name.clone(),
            data: node.data.clone(),
            color: node.color,
            position: new_pos,
            desired_neighbor_ids,
        });
        changed = true;
    }
    if changed {
        persistence_dirty.0 = true;
    }
}

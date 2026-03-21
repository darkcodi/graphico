use bevy::prelude::*;

use crate::api::state::{color_to_hex, NodeUuidRegistry};
use crate::graph::components::{GraphNode, Selected};
use crate::graph::model::GraphData;

#[derive(Component)]
pub struct InspectorPanel;

#[derive(Component)]
pub struct InspectorText;

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
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.88)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.15)),
            Visibility::Hidden,
        ))
        .with_child((
            InspectorText,
            Text::new(""),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgba(0.9, 0.9, 0.9, 0.95)),
        ));
}

pub fn update_inspector(
    selected_q: Query<&GraphNode, With<Selected>>,
    graph: Res<GraphData>,
    registry: Res<NodeUuidRegistry>,
    mut panel_q: Query<&mut Visibility, With<InspectorPanel>>,
    mut text_q: Query<&mut Text, With<InspectorText>>,
) {
    let Ok(mut vis) = panel_q.single_mut() else {
        return;
    };
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    let Some(graph_node) = selected_q.iter().next() else {
        *vis = Visibility::Hidden;
        return;
    };

    let Some(node_data) = graph.nodes.get(&graph_node.id) else {
        *vis = Visibility::Hidden;
        return;
    };

    *vis = Visibility::Inherited;

    let edge_count = graph
        .adjacency
        .get(&graph_node.id)
        .map_or(0, |e| e.len());

    let uuid_str = registry
        .node_to_uuid
        .get(&graph_node.id)
        .map(|u| u.to_string())
        .unwrap_or_else(|| format!("{}", graph_node.id.0));

    let color_hex = color_to_hex(&node_data.color);

    let mut content = format!(
        "ID: {}\nName: {}\nColor: {}\nPos: ({:.0}, {:.0})\nSize: {:.0} x {:.0}\nEdges: {}",
        uuid_str,
        node_data.name,
        color_hex,
        node_data.position.x,
        node_data.position.y,
        node_data.size.x,
        node_data.size.y,
        edge_count,
    );

    if !node_data.data.is_empty() {
        content.push_str(&format!("\n\nData:\n{}", node_data.data));
    }

    **text = content;
}

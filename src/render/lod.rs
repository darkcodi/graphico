use bevy::prelude::*;

use crate::camera::components::CameraState;
use crate::graph::components::GraphNode;
use crate::graph::model::GraphData;

/// Hide nodes whose projected screen size would be less than ~1 pixel.
pub fn lod_visibility(
    camera_q: Query<&Projection, With<CameraState>>,
    mut nodes_q: Query<(&GraphNode, &mut Visibility)>,
    graph: Res<GraphData>,
) {
    let Ok(projection) = camera_q.single() else {
        return;
    };

    let scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };

    for (graph_node, mut visibility) in nodes_q.iter_mut() {
        if let Some(node_data) = graph.nodes.get(&graph_node.id) {
            let screen_size = (node_data.radius * 2.0) / scale;
            if screen_size < 0.5 {
                *visibility = Visibility::Hidden;
            } else {
                *visibility = Visibility::Inherited;
            }
        }
    }
}

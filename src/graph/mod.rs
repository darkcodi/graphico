pub mod components;
pub mod events;
pub mod model;
pub mod systems;

use bevy::prelude::*;

use crate::GraphSystems;
use events::{AddEdgeEvent, AddNodeEvent, DeleteNodeEvent};
use model::GraphData;

pub struct GraphPlugin;

impl Plugin for GraphPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GraphData>()
            .add_message::<AddNodeEvent>()
            .add_message::<AddEdgeEvent>()
            .add_message::<DeleteNodeEvent>()
            .add_systems(
                Update,
                (
                    systems::process_add_node_events,
                    systems::process_add_edge_events,
                    systems::process_delete_node_events,
                )
                    .in_set(GraphSystems::EventProcessing),
            );
    }
}

pub mod components;
pub mod events;
pub mod model;
pub mod systems;

use bevy::prelude::*;

use crate::GraphSystems;
use events::{AddEdgeEvent, AddNodeEvent};
use model::GraphData;
use systems::LayoutState;

pub struct GraphPlugin;

impl Plugin for GraphPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GraphData>()
            .init_resource::<LayoutState>()
            .add_message::<AddNodeEvent>()
            .add_message::<AddEdgeEvent>()
            .add_systems(
                Update,
                (
                    systems::process_add_node_events,
                    systems::process_add_edge_events,
                )
                    .in_set(GraphSystems::EventProcessing),
            )
            .add_systems(
                Update,
                systems::incremental_layout.in_set(GraphSystems::Layout),
            );
    }
}

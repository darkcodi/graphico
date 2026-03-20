use bevy::prelude::*;
use rand::Rng;

use crate::graph::events::{AddEdgeEvent, AddNodeEvent};
use crate::graph::model::{GraphData, NodeId};

/// Timer controlling how frequently new nodes/edges are spawned.
#[derive(Resource)]
pub struct DemoTimer {
    pub timer: Timer,
    pub nodes_per_tick: usize,
    pub max_nodes: usize,
}

impl Default for DemoTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.05, TimerMode::Repeating),
            nodes_per_tick: 10,
            max_nodes: 100_000,
        }
    }
}

/// Demo system that gradually builds a large graph.
/// A real application would use AddNodeEvent/AddEdgeEvent directly
/// from its own data source (file loader, network stream, etc.).
pub fn demo_spawn_system(
    time: Res<Time>,
    mut demo: ResMut<DemoTimer>,
    graph: Res<GraphData>,
    mut node_events: MessageWriter<AddNodeEvent>,
    mut edge_events: MessageWriter<AddEdgeEvent>,
) {
    demo.timer.tick(time.delta());

    if !demo.timer.just_finished() {
        return;
    }

    let current_count = graph.node_count();
    if current_count >= demo.max_nodes {
        return;
    }

    let mut rng = rand::rng();

    let existing_ids: Vec<NodeId> = graph.nodes.keys().copied().collect();

    for _ in 0..demo.nodes_per_tick {
        let position = if !existing_ids.is_empty() && rng.random_bool(0.7) {
            let neighbor_id = existing_ids[rng.random_range(0..existing_ids.len())];
            let neighbor_pos = graph.nodes[&neighbor_id].position;
            neighbor_pos
                + Vec2::new(
                    rng.random_range(-200.0..200.0),
                    rng.random_range(-200.0..200.0),
                )
        } else {
            let spread = (current_count as f32).sqrt() * 50.0 + 500.0;
            Vec2::new(
                rng.random_range(-spread..spread),
                rng.random_range(-spread..spread),
            )
        };

        let color = Color::hsl(rng.random_range(0.0..360.0), 0.7, 0.6);

        node_events.write(AddNodeEvent {
            position,
            label: format!("N{}", current_count + 1),
            color,
            radius: rng.random_range(4.0..8.0),
        });
    }

    // Connect edges between existing nodes (edges to new nodes will be added in subsequent frames)
    if existing_ids.len() >= 2 {
        let edge_count = (demo.nodes_per_tick / 2).max(1);
        for _ in 0..edge_count {
            let a = existing_ids[rng.random_range(0..existing_ids.len())];
            let b = existing_ids[rng.random_range(0..existing_ids.len())];
            if a != b {
                edge_events.write(AddEdgeEvent {
                    source: a,
                    target: b,
                    color: Color::srgba(0.5, 0.5, 0.5, 0.4),
                });
            }
        }
    }
}

pub struct DemoPlugin;

impl Plugin for DemoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DemoTimer>().add_systems(
            Update,
            demo_spawn_system.before(crate::GraphSystems::EventProcessing),
        );
    }
}

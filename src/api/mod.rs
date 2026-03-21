pub mod handlers;
pub mod state;

use std::sync::{Mutex, mpsc, Arc, RwLock};

use axum::routing::{delete, get, post};
use axum::Router;
use bevy::prelude::*;
use rand::Rng;

use crate::graph::events::{AddEdgeEvent, AddNodeEvent, DeleteNodeEvent};
use crate::graph::model::GraphData;
use crate::GraphSystems;

use state::{
    ApiCommand, ApiCommandReceiver, ApiNode, ApiPosition, AxumAppState, NodeUuidRegistry,
    SharedGraphState, SharedStateHandle, color_to_hex,
};

pub struct ApiPlugin;

impl Plugin for ApiPlugin {
    fn build(&self, app: &mut App) {
        let shared = Arc::new(RwLock::new(SharedGraphState::default()));
        let (cmd_tx, cmd_rx) = mpsc::sync_channel::<ApiCommand>(1024);

        let axum_state = AxumAppState {
            shared: shared.clone(),
            cmd_tx,
        };

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime for API server");

            rt.block_on(async {
                let router = Router::new()
                    .route("/node", post(handlers::create_node))
                    .route("/node/{id}", get(handlers::get_node))
                    .route("/node/{id}", delete(handlers::delete_node))
                    .route("/nodes", get(handlers::get_all_nodes))
                    .with_state(axum_state);

                let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
                    .await
                    .expect("failed to bind API server to 127.0.0.1:3000");

                info!("API server listening on http://127.0.0.1:3000");
                axum::serve(listener, router).await.unwrap();
            });
        });

        app.insert_resource(SharedStateHandle(shared))
            .insert_resource(ApiCommandReceiver(Mutex::new(cmd_rx)))
            .init_resource::<NodeUuidRegistry>()
            .add_systems(
                Update,
                api_command_system.before(GraphSystems::EventProcessing),
            )
            .add_systems(
                Update,
                api_sync_system.after(GraphSystems::EventProcessing),
            );
    }
}

fn api_command_system(
    receiver: Res<ApiCommandReceiver>,
    mut registry: ResMut<NodeUuidRegistry>,
    mut graph: ResMut<GraphData>,
    mut node_events: MessageWriter<AddNodeEvent>,
    mut edge_events: MessageWriter<AddEdgeEvent>,
    mut delete_events: MessageWriter<DeleteNodeEvent>,
) {
    let mut rng = rand::rng();

    let rx = receiver.0.lock().unwrap();
    while let Ok(cmd) = rx.try_recv() {
        match cmd {
            ApiCommand::CreateNode {
                uuid,
                data,
                color,
                edges,
                position,
            } => {
                let node_id = graph.next_node_id();
                registry.register(uuid, node_id);

                let bevy_color = match color {
                    Some([r, g, b]) => Color::srgb(r, g, b),
                    None => Color::hsl(rng.random_range(0.0..360.0), 0.7, 0.6),
                };

                node_events.write(AddNodeEvent {
                    position,
                    data,
                    color: bevy_color,
                    pre_allocated_id: Some(node_id),
                });

                for target_uuid in edges {
                    if let Some(&target_node_id) = registry.uuid_to_node.get(&target_uuid) {
                        edge_events.write(AddEdgeEvent {
                            source: node_id,
                            target: target_node_id,
                            color: Color::srgba(0.5, 0.5, 0.5, 0.4),
                        });
                    }
                }
            }
            ApiCommand::DeleteNode { uuid } => {
                if let Some(node_id) = registry.uuid_to_node.get(&uuid).copied() {
                    delete_events.write(DeleteNodeEvent { id: node_id });
                    registry.remove_by_uuid(&uuid);
                }
            }
        }
    }
}

fn api_sync_system(
    graph: Res<GraphData>,
    registry: Res<NodeUuidRegistry>,
    shared: Res<SharedStateHandle>,
) {
    let mut state = shared.0.write().unwrap();
    state.nodes.clear();

    for (uuid, &node_id) in &registry.uuid_to_node {
        let Some(node_data) = graph.nodes.get(&node_id) else {
            continue;
        };

        let mut neighbor_uuids = Vec::new();
        if let Some(edge_ids) = graph.adjacency.get(&node_id) {
            for edge_id in edge_ids {
                if let Some(edge) = graph.edges.get(edge_id) {
                    let other_id = if edge.source == node_id {
                        edge.target
                    } else {
                        edge.source
                    };
                    if let Some(other_uuid) = registry.node_to_uuid.get(&other_id)
                        && !neighbor_uuids.contains(other_uuid)
                    {
                        neighbor_uuids.push(*other_uuid);
                    }
                }
            }
        }

        state.nodes.insert(
            *uuid,
            ApiNode {
                id: *uuid,
                data: node_data.data.clone(),
                color: color_to_hex(&node_data.color),
                edges: neighbor_uuids,
                position: ApiPosition {
                    x: node_data.position.x,
                    y: node_data.position.y,
                },
            },
        );
    }
}

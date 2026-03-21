pub mod handlers;
pub mod snapshot;
pub mod state;

use std::sync::{mpsc, Arc, Mutex, RwLock};

use axum::routing::get;
use axum::Router;
use bevy::prelude::*;
use rand::Rng;

use crate::graph::events::{AddEdgeEvent, AddNodeEvent, DeleteNodeEvent, UpdateNodeEvent};
use crate::graph::model::{GraphData, NodeId};
use crate::persist::{
    inject_load_creates, inject_load_edges, persist_snapshot_system, GraphPersistenceDirty,
};
use crate::GraphSystems;

use snapshot::build_api_snapshot;
use state::{
    ApiCommand, ApiCommandReceiver, ApiCommandSender, AxumAppState, NodeUuidRegistry,
    SharedGraphState, SharedStateHandle,
};

pub struct ApiPlugin;

impl Plugin for ApiPlugin {
    fn build(&self, app: &mut App) {
        let shared = Arc::new(RwLock::new(SharedGraphState::default()));
        let (cmd_tx, cmd_rx) = mpsc::sync_channel::<ApiCommand>(1024);

        app.insert_resource(ApiCommandSender(cmd_tx.clone()));

        let axum_state = AxumAppState {
            shared: shared.clone(),
            cmd_tx: cmd_tx.clone(),
        };

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime for API server");

            rt.block_on(async {
                let router = Router::new()
                    .route(
                        "/nodes",
                        get(handlers::get_all_nodes)
                            .post(handlers::create_node)
                            .delete(handlers::delete_all_nodes),
                    )
                    .route(
                        "/nodes/{id}",
                        get(handlers::get_node)
                            .put(handlers::update_node)
                            .delete(handlers::delete_node),
                    )
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
                (
                    inject_load_creates,
                    inject_load_edges,
                    api_command_system,
                )
                    .chain()
                    .before(GraphSystems::EventProcessing),
            )
            .add_systems(
                Update,
                api_sync_system.after(GraphSystems::EventProcessing),
            )
            .add_systems(
                Update,
                persist_snapshot_system.after(api_sync_system),
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
    mut update_events: MessageWriter<UpdateNodeEvent>,
    mut persistence_dirty: ResMut<GraphPersistenceDirty>,
) {
    let mut rng = rand::rng();

    let rx = receiver.0.lock().unwrap();
    let mut any_cmd = false;
    while let Ok(cmd) = rx.try_recv() {
        any_cmd = true;
        match cmd {
            ApiCommand::CreateNode {
                uuid,
                name,
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
                    name,
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
            ApiCommand::UpdateNode {
                uuid,
                name,
                data,
                color,
                edges,
                position,
            } => {
                let Some(&node_id) = registry.uuid_to_node.get(&uuid) else {
                    continue;
                };

                let bevy_color = match color {
                    Some([r, g, b]) => Color::srgb(r, g, b),
                    None => graph
                        .nodes
                        .get(&node_id)
                        .map(|n| n.color)
                        .unwrap_or(Color::WHITE),
                };

                let mut seen = std::collections::HashSet::new();
                let mut desired_neighbor_ids = Vec::new();
                for target_uuid in edges {
                    if let Some(&target_node_id) = registry.uuid_to_node.get(&target_uuid) {
                        if target_node_id == node_id {
                            continue;
                        }
                        if seen.insert(target_node_id) {
                            desired_neighbor_ids.push(target_node_id);
                        }
                    }
                }

                update_events.write(UpdateNodeEvent {
                    node_id,
                    name,
                    data,
                    color: bevy_color,
                    position,
                    desired_neighbor_ids,
                });
            }
            ApiCommand::DeleteNode { uuid } => {
                if let Some(node_id) = registry.uuid_to_node.get(&uuid).copied() {
                    delete_events.write(DeleteNodeEvent { id: node_id });
                    registry.remove_by_uuid(&uuid);
                }
            }
            ApiCommand::DeleteAllNodes => {
                let pairs: Vec<(uuid::Uuid, NodeId)> = registry
                    .uuid_to_node
                    .iter()
                    .map(|(u, &n)| (*u, n))
                    .collect();
                for (uuid, node_id) in pairs {
                    delete_events.write(DeleteNodeEvent { id: node_id });
                    registry.remove_by_uuid(&uuid);
                }
            }
        }
    }
    if any_cmd {
        persistence_dirty.0 = true;
    }
}

fn api_sync_system(
    graph: Res<GraphData>,
    registry: Res<NodeUuidRegistry>,
    shared: Res<SharedStateHandle>,
) {
    let snapshot = build_api_snapshot(&graph, &registry);
    let mut state = shared.0.write().unwrap();
    state.nodes = snapshot;
}

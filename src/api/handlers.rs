use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use bevy::prelude::Vec2;
use uuid::Uuid;

use super::state::{
    ApiCommand, AxumAppState, CreateNodeRequest, CreateNodeResponse, UpdateNodeRequest,
    parse_hex_color,
};

pub async fn create_node(
    State(state): State<AxumAppState>,
    Json(body): Json<CreateNodeRequest>,
) -> impl IntoResponse {
    let uuid = Uuid::new_v4();
    let color = body.color.as_deref().and_then(parse_hex_color);
    let edges = body.edges.unwrap_or_default();
    let position = Vec2::new(body.position.x, body.position.y);

    let cmd = ApiCommand::CreateNode {
        uuid,
        name: body.name,
        data: body.data,
        color,
        edges,
        position,
    };

    if state.cmd_tx.send(cmd).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "server shutting down"}))).into_response();
    }

    (StatusCode::CREATED, Json(CreateNodeResponse { id: uuid })).into_response()
}

pub async fn get_node(
    State(state): State<AxumAppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let shared = state.shared.read().unwrap();
    match shared.nodes.get(&id) {
        Some(node) => (StatusCode::OK, Json(serde_json::to_value(node).unwrap())).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "node not found"}))).into_response(),
    }
}

pub async fn get_all_nodes(State(state): State<AxumAppState>) -> impl IntoResponse {
    let shared = state.shared.read().unwrap();
    let nodes: Vec<_> = shared.nodes.values().cloned().collect();
    Json(nodes)
}

pub async fn update_node(
    State(state): State<AxumAppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateNodeRequest>,
) -> impl IntoResponse {
    let existing = {
        let shared = state.shared.read().unwrap();
        shared.nodes.get(&id).cloned()
    };
    let Some(existing) = existing else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "node not found"})))
            .into_response();
    };

    let color_rgb = match body.color.as_ref() {
        None => None,
        Some(s) => match parse_hex_color(s) {
            Some(rgb) => Some(rgb),
            None => parse_hex_color(&existing.color),
        },
    };

    let name = body.name.unwrap_or(existing.name);
    let data = body.data.unwrap_or(existing.data);
    let position = body
        .position
        .map(|p| Vec2::new(p.x, p.y))
        .unwrap_or_else(|| Vec2::new(existing.position.x, existing.position.y));
    let edges = body.edges.unwrap_or(existing.edges);

    let cmd = ApiCommand::UpdateNode {
        uuid: id,
        name,
        data,
        color: color_rgb,
        edges,
        position,
    };

    if state.cmd_tx.send(cmd).is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "server shutting down"})),
        )
            .into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}

pub async fn delete_node(
    State(state): State<AxumAppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    {
        let shared = state.shared.read().unwrap();
        if !shared.nodes.contains_key(&id) {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "node not found"}))).into_response();
        }
    }

    let cmd = ApiCommand::DeleteNode { uuid: id };
    if state.cmd_tx.send(cmd).is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "server shutting down"}))).into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}

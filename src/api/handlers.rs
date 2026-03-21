use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use uuid::Uuid;

use super::state::{
    ApiCommand, AxumAppState, CreateNodeRequest, CreateNodeResponse, parse_hex_color,
};

pub async fn create_node(
    State(state): State<AxumAppState>,
    Json(body): Json<CreateNodeRequest>,
) -> impl IntoResponse {
    let uuid = Uuid::new_v4();
    let color = body.color.as_deref().and_then(parse_hex_color);
    let edges = body.edges.unwrap_or_default();

    if let Some(r) = body.radius
        && r == 0
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "radius must be a positive integer"})),
        )
            .into_response();
    }

    let cmd = ApiCommand::CreateNode {
        uuid,
        name: body.name,
        color,
        edges,
        radius: body.radius,
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

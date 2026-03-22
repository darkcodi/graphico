use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex, RwLock};
use uuid::Uuid;

use crate::graph::model::NodeId;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Coordinates {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiNode {
    pub id: Uuid,
    pub name: String,
    pub data: String,
    pub color: String,
    pub edges: Vec<Uuid>,
    pub position: Coordinates,
    pub size: Coordinates,
    /// UUIDs of other nodes whose axis-aligned bounds (center `position`, `size`) intersect with positive area.
    #[serde(default)]
    pub overlaps: Vec<Uuid>,
}

#[derive(Deserialize)]
pub struct CreateNodeRequest {
    pub name: String,
    #[serde(default)]
    pub data: String,
    pub color: Option<String>,
    pub edges: Option<Vec<Uuid>>,
    pub position: Coordinates,
}

#[derive(Serialize)]
pub struct CreateNodeResponse {
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct BulkCreateNodesRequest {
    pub nodes: Vec<CreateNodeRequest>,
}

#[derive(Serialize)]
pub struct BulkCreateNodesResponse {
    pub ids: Vec<Uuid>,
}

/// Partial update: omit a field to leave it unchanged. `id` is taken from the URL path only.
#[derive(Deserialize)]
pub struct UpdateNodeRequest {
    pub name: Option<String>,
    pub data: Option<String>,
    pub color: Option<String>,
    pub edges: Option<Vec<Uuid>>,
    pub position: Option<Coordinates>,
}

pub enum ApiCommand {
    CreateNode {
        uuid: Uuid,
        name: String,
        data: String,
        color: Option<[f32; 3]>,
        edges: Vec<Uuid>,
        position: Vec2,
    },
    UpdateNode {
        uuid: Uuid,
        name: String,
        data: String,
        /// `None` = keep existing node color.
        color: Option<[f32; 3]>,
        edges: Vec<Uuid>,
        position: Vec2,
    },
    DeleteNode {
        uuid: Uuid,
    },
    DeleteAllNodes,
}

#[derive(Default)]
pub struct SharedGraphState {
    pub nodes: HashMap<Uuid, ApiNode>,
}

#[derive(Resource, Clone)]
pub struct SharedStateHandle(pub Arc<RwLock<SharedGraphState>>);

#[derive(Resource)]
pub struct ApiCommandReceiver(pub Mutex<mpsc::Receiver<ApiCommand>>);

#[derive(Resource, Clone)]
pub struct ApiCommandSender(pub mpsc::Sender<ApiCommand>);

#[derive(Resource, Default)]
pub struct NodeUuidRegistry {
    pub uuid_to_node: HashMap<Uuid, NodeId>,
    pub node_to_uuid: HashMap<NodeId, Uuid>,
}   

impl NodeUuidRegistry {
    pub fn register(&mut self, uuid: Uuid, node_id: NodeId) {
        self.uuid_to_node.insert(uuid, node_id);
        self.node_to_uuid.insert(node_id, uuid);
    }

    pub fn remove_by_uuid(&mut self, uuid: &Uuid) -> Option<NodeId> {
        if let Some(node_id) = self.uuid_to_node.remove(uuid) {
            self.node_to_uuid.remove(&node_id);
            Some(node_id)
        } else {
            None
        }
    }

    pub fn remove_by_node_id(&mut self, node_id: &NodeId) -> Option<Uuid> {
        if let Some(uuid) = self.node_to_uuid.remove(node_id) {
            self.uuid_to_node.remove(&uuid);
            Some(uuid)
        } else {
            None
        }
    }
}

pub fn parse_hex_color(hex: &str) -> Option<[f32; 3]> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}

pub fn color_to_hex(color: &Color) -> String {
    let srgba = color.to_srgba();
    let r = (srgba.red.clamp(0.0, 1.0) * 255.0) as u8;
    let g = (srgba.green.clamp(0.0, 1.0) * 255.0) as u8;
    let b = (srgba.blue.clamp(0.0, 1.0) * 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

#[derive(Clone)]
pub struct AxumAppState {
    pub shared: Arc<RwLock<SharedGraphState>>,
    pub cmd_tx: mpsc::Sender<ApiCommand>,
}

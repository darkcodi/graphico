//! SQLite snapshot persistence for the graph (API-shaped rows).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use bevy::prelude::*;
use rusqlite::{params, Connection, TransactionBehavior};
use uuid::Uuid;

use crate::api::state::{
    ApiCommand, ApiNode, ApiPosition, NodeUuidRegistry, parse_hex_color,
};
use crate::graph::model::GraphData;

/// Override with `GRAPHICO_DB_PATH` to use a custom database file.
pub fn database_path() -> PathBuf {
    std::env::var("GRAPHICO_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut p = dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."));
            p.push("graphico");
            let _ = std::fs::create_dir_all(&p);
            p.push("graphico.db");
            p
        })
}

pub fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    apply_pragmas(&conn)?;
    create_schema(&conn)?;
    Ok(conn)
}

fn apply_pragmas(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = FULL;
        PRAGMA foreign_keys = ON;
        ",
    )?;
    Ok(())
}

fn create_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value INTEGER NOT NULL
        );
        INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', 1);

        CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            data TEXT NOT NULL,
            color TEXT NOT NULL,
            pos_x REAL NOT NULL,
            pos_y REAL NOT NULL,
            edges_json TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

/// Load all node rows from the database.
pub fn load_all_nodes(conn: &Connection) -> rusqlite::Result<Vec<ApiNode>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, data, color, pos_x, pos_y, edges_json FROM nodes ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let edges_json: String = row.get(6)?;
        let edges: Vec<Uuid> = serde_json::from_str(&edges_json).unwrap_or_default();
        let uuid = Uuid::parse_str(&id).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        Ok(ApiNode {
            id: uuid,
            name: row.get(1)?,
            data: row.get(2)?,
            color: row.get(3)?,
            edges,
            position: ApiPosition {
                x: row.get(4)?,
                y: row.get(5)?,
            },
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn save_snapshot(conn: &mut Connection, snapshot: &HashMap<Uuid, ApiNode>) -> rusqlite::Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute("DELETE FROM nodes", [])?;
    for node in snapshot.values() {
        let edges_json = serde_json::to_string(&node.edges).unwrap_or_else(|_| "[]".into());
        tx.execute(
            "INSERT INTO nodes (id, name, data, color, pos_x, pos_y, edges_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                node.id.to_string(),
                node.name,
                node.data,
                node.color,
                node.position.x,
                node.position.y,
                edges_json,
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

// --- Bevy resources ---

#[derive(Resource)]
pub struct GraphPersistenceDirty(pub bool);

#[derive(Resource)]
pub struct PersistConnection(pub Option<Mutex<Connection>>);

/// Two-phase restore: create all nodes (no edges), then connect edges.
#[derive(Resource)]
pub enum PendingGraphLoad {
    /// Nothing to load or finished.
    Idle,
    /// Send `CreateNode` with empty edges for each row.
    PendingCreates { rows: Vec<ApiNode> },
    /// Wait until registry size matches, then send `UpdateNode` with edges.
    PendingEdges { rows: Vec<ApiNode> },
}

impl PendingGraphLoad {
    pub fn from_rows(rows: Vec<ApiNode>) -> Self {
        if rows.is_empty() {
            PendingGraphLoad::Idle
        } else {
            PendingGraphLoad::PendingCreates { rows }
        }
    }
}

pub struct PersistPlugin;

impl Plugin for PersistPlugin {
    fn build(&self, app: &mut App) {
        let path = database_path();
        match open_connection(&path) {
            Ok(conn) => {
                let rows = load_all_nodes(&conn).unwrap_or_default();
                info!(
                    "Graph database: {} ({} nodes on disk)",
                    path.display(),
                    rows.len()
                );
                app.insert_resource(PersistConnection(Some(Mutex::new(conn))))
                    .insert_resource(PendingGraphLoad::from_rows(rows))
                    .insert_resource(GraphPersistenceDirty(false));
            }
            Err(e) => {
                tracing::error!(
                    "Failed to open graph database at {}: {} — persistence disabled",
                    path.display(),
                    e
                );
                app.insert_resource(PersistConnection(None))
                    .insert_resource(PendingGraphLoad::Idle)
                    .insert_resource(GraphPersistenceDirty(false));
            }
        }
    }
}

pub fn inject_load_creates(
    mut pending: ResMut<PendingGraphLoad>,
    cmd: Res<crate::api::state::ApiCommandSender>,
) {
    let rows = match std::mem::replace(&mut *pending, PendingGraphLoad::Idle) {
        PendingGraphLoad::PendingCreates { rows } => rows,
        other => {
            *pending = other;
            return;
        }
    };

    if rows.is_empty() {
        *pending = PendingGraphLoad::Idle;
        return;
    }

    for row in &rows {
        let color = parse_hex_color(&row.color);
        let position = Vec2::new(row.position.x, row.position.y);
        let _ = cmd.0.send(ApiCommand::CreateNode {
            uuid: row.id,
            name: row.name.clone(),
            data: row.data.clone(),
            color,
            edges: vec![],
            position,
        });
    }

    *pending = PendingGraphLoad::PendingEdges { rows };
}

pub fn inject_load_edges(
    mut pending: ResMut<PendingGraphLoad>,
    registry: Res<NodeUuidRegistry>,
    cmd: Res<crate::api::state::ApiCommandSender>,
) {
    let expected = match &*pending {
        PendingGraphLoad::PendingEdges { rows } => rows.len(),
        _ => return,
    };

    if registry.uuid_to_node.len() != expected {
        return;
    }

    let rows = match std::mem::replace(&mut *pending, PendingGraphLoad::Idle) {
        PendingGraphLoad::PendingEdges { rows } => rows,
        other => {
            *pending = other;
            return;
        }
    };

    for row in &rows {
        let color = parse_hex_color(&row.color);
        let position = Vec2::new(row.position.x, row.position.y);
        let _ = cmd.0.send(ApiCommand::UpdateNode {
            uuid: row.id,
            name: row.name.clone(),
            data: row.data.clone(),
            color,
            edges: row.edges.clone(),
            position,
        });
    }

    *pending = PendingGraphLoad::Idle;
}

pub fn persist_snapshot_system(
    mut dirty: ResMut<GraphPersistenceDirty>,
    graph: Res<GraphData>,
    registry: Res<NodeUuidRegistry>,
    conn: Res<PersistConnection>,
) {
    if !dirty.0 {
        return;
    }

    let Some(ref mutex) = conn.0 else {
        dirty.0 = false;
        return;
    };

    let snapshot = crate::api::snapshot::build_api_snapshot(&graph, &registry);
    let mut guard = match mutex.lock() {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("persist lock poisoned: {}", e);
            return;
        }
    };
    match save_snapshot(&mut *guard, &snapshot) {
        Ok(()) => {
            dirty.0 = false;
        }
        Err(e) => {
            tracing::error!("Failed to save graph snapshot: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn round_trip_two_nodes_one_edge() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let mut conn = open_connection(&path).unwrap();

        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let mut snapshot = HashMap::new();
        snapshot.insert(
            a,
            ApiNode {
                id: a,
                name: "A".into(),
                data: "".into(),
                color: "#FF0000".into(),
                edges: vec![b],
                position: ApiPosition { x: 1.0, y: 2.0 },
            },
        );
        snapshot.insert(
            b,
            ApiNode {
                id: b,
                name: "B".into(),
                data: "x".into(),
                color: "#00FF00".into(),
                edges: vec![a],
                position: ApiPosition { x: 3.0, y: 4.0 },
            },
        );

        save_snapshot(&mut conn, &snapshot).unwrap();
        drop(conn);

        let conn2 = Connection::open(&path).unwrap();
        let loaded = load_all_nodes(&conn2).unwrap();
        assert_eq!(loaded.len(), 2);

        let by_id: HashMap<Uuid, ApiNode> = loaded.into_iter().map(|n| (n.id, n)).collect();
        assert_eq!(by_id[&a].edges, vec![b]);
        assert_eq!(by_id[&b].edges, vec![a]);
        assert_eq!(by_id[&a].position.x, 1.0);
        assert_eq!(by_id[&b].name, "B");
    }
}
